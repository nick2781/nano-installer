//! Wraps a finished setup in the package an administrator deploys.
//!
//! A setup produced here is one executable a person runs. Larger estates deploy
//! through Windows Installer instead: a `.msi` that Group Policy, Intune or
//! Configuration Manager can hand to a machine and take away again. This module
//! writes that package around the setup this build just finished, so a project
//! ships both without a second toolchain in the pipeline.
//!
//! The package carries the setup image in its `Binary` table and runs it from a
//! deferred custom action, which is what lets an unattended `msiexec /qn` work
//! where the setup alone would have to draw its wizard. Two actions do the real
//! work: the install action runs the setup with `--silent --dir "<location>"`,
//! and the uninstall action runs the uninstaller the setup left in that
//! location. Everything else the package carries is bookkeeping Windows
//! Installer needs to consider the product installed and to take it back.
//!
//! The database itself is written through the Installer's own engine rather than
//! by assembling the compound file by hand: `MsiOpenDatabase` creates a package
//! with the standard schema, rows go in through parameterised views, and the
//! summary information is written through its API. That keeps the layout of an
//! MSI, which is a documented but intricate binary format, with the component
//! that owns it and on every machine that can run the result.
//!
//! Two properties of the wrapper are worth stating because they shape what a
//! project can expect. First, the package decides where the product goes: it
//! installs into `INSTALLDIR` (`%ProgramFiles%\<name>` for a setup that asks for
//! administrator rights, `%LOCALAPPDATA%\Programs\<name>` otherwise), and an
//! administrator can name another directory on the command line. Second, the
//! setup it wraps has to accept a windowless run: a project without
//! `advanced.silent_mode_support` and `advanced.uninstall_mode_support` has no
//! unattended flow for the package to drive, and the build refuses rather than
//! produce a package that cannot install.
//!
//! An administrative install (`msiexec /a`) leaves the setup alone: the image is
//! extracted for the image's own sake, and the actions that install and remove
//! the product run only in a real install or uninstall.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use windows::core::PCWSTR;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::ApplicationInstallationAndServicing::{
    MsiCloseHandle, MsiCreateRecord, MsiDatabaseCommit, MsiDatabaseOpenViewW,
    MsiGetSummaryInformationW, MsiOpenDatabaseW, MsiRecordSetInteger, MsiRecordSetStreamW,
    MsiRecordSetStringW, MsiSummaryInfoPersist, MsiSummaryInfoSetPropertyW, MsiViewClose,
    MsiViewExecute, MsiViewModify, MSIHANDLE, MSIMODIFY_INSERT,
};

use crate::version;

/// What a call that worked returns.
const ERROR_SUCCESS: u32 = 0;

/// The persistence mode `MsiOpenDatabase` takes for a database this build is
/// writing: the constants in the Installer's header are pointers whose value is
/// the mode, not the names written out as text.
const MSIDBOPEN_CREATE: usize = 3;

/// The value of the `CustomAction.Type` column, built from the documented type
/// and flag values: an executable the package carries (2) or one a property
/// names (50), queued into the installation script (1024), and running without
/// impersonation (2048) when the product needs administrator rights.
const ACTION_EXE_FROM_BINARY: u32 = 2;
const ACTION_EXE_FROM_PROPERTY: u32 = 50;
const ACTION_SET_PROPERTY: u32 = 51;
const ACTION_IN_SCRIPT: u32 = 1024;
const ACTION_NO_IMPERSONATION: u32 = 2048;

/// The tables this package writes, in the shape Windows Installer's own schema
/// defines them.
///
/// A database created here starts empty, so the package declares the columns it
/// uses: the key columns come first as the Installer requires, and a column that
/// may hold nothing is left nullable so the rows that have nothing to say can
/// say it with a null. The sizes and the attributes are the ones the standard
/// schema gives these columns, which is what a validation tool compares against.
const SCHEMA: [&str; 18] = [
    "CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(0) LOCALIZABLE PRIMARY KEY `Property`)",
    "CREATE TABLE `Binary` (`Name` CHAR(72) NOT NULL, `Data` OBJECT PRIMARY KEY `Name`)",
    "CREATE TABLE `Directory` (`Directory` CHAR(72) NOT NULL, `Directory_Parent` CHAR(72), `DefaultDir` CHAR(255) NOT NULL PRIMARY KEY `Directory`)",
    "CREATE TABLE `Component` (`Component` CHAR(72) NOT NULL, `ComponentId` CHAR(38), `Directory_` CHAR(72) NOT NULL, `Attributes` SHORT NOT NULL, `Condition` CHAR(255), `KeyPath` CHAR(72) PRIMARY KEY `Component`)",
    "CREATE TABLE `CreateFolder` (`Directory_` CHAR(72) NOT NULL, `Component_` CHAR(72) NOT NULL PRIMARY KEY `Directory_`)",
    "CREATE TABLE `Registry` (`Registry` CHAR(72) NOT NULL, `Root` SHORT NOT NULL, `Key` CHAR(255) NOT NULL, `Name` CHAR(255), `Value` CHAR(0) LOCALIZABLE, `Component_` CHAR(72) NOT NULL PRIMARY KEY `Registry`)",
    "CREATE TABLE `Feature` (`Feature` CHAR(38) NOT NULL, `Feature_Parent` CHAR(38), `Title` CHAR(64) LOCALIZABLE, `Description` CHAR(255) LOCALIZABLE, `Display` SHORT, `Level` SHORT NOT NULL, `Directory_` CHAR(72), `Attributes` SHORT NOT NULL PRIMARY KEY `Feature`)",
    "CREATE TABLE `FeatureComponents` (`Feature_` CHAR(38) NOT NULL, `Component_` CHAR(72) NOT NULL PRIMARY KEY `Feature_`, `Component_`)",
    "CREATE TABLE `CustomAction` (`Action` CHAR(72) NOT NULL, `Type` SHORT NOT NULL, `Source` CHAR(72), `Target` CHAR(255) LOCALIZABLE PRIMARY KEY `Action`)",
    "CREATE TABLE `InstallExecuteSequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT NOT NULL PRIMARY KEY `Action`)",
    "CREATE TABLE `InstallUISequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT NOT NULL PRIMARY KEY `Action`)",
    "CREATE TABLE `AdminExecuteSequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT NOT NULL PRIMARY KEY `Action`)",
    "CREATE TABLE `AdvtExecuteSequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT NOT NULL PRIMARY KEY `Action`)",
    "CREATE TABLE `Upgrade` (`UpgradeCode` CHAR(38) NOT NULL, `VersionMin` CHAR(20), `VersionMax` CHAR(20), `Language` CHAR(255), `Attributes` LONG NOT NULL, `Remove` CHAR(255), `ActionProperty` CHAR(72) NOT NULL PRIMARY KEY `UpgradeCode`)",
    // An empty package still carries one medium: registering the product reads
    // it, and a package without the table is refused as damaged.
    "CREATE TABLE `Media` (`DiskId` SHORT NOT NULL, `LastSequence` LONG NOT NULL, `DiskPrompt` CHAR(64) LOCALIZABLE, `Cabinet` CHAR(255), `VolumeLabel` CHAR(32), `Source` CHAR(72) PRIMARY KEY `DiskId`)",
    // Where the package records the directory it installed into, and the search
    // that reads it back when the product is removed: a caller may name the
    // directory on the command line, and the removal has to find the product
    // where it went rather than where this build would have put it.
    "CREATE TABLE `RegLocator` (`Signature_` CHAR(72) NOT NULL, `Root` SHORT NOT NULL, `Key` CHAR(255) NOT NULL, `Name` CHAR(255), `Type` SHORT NOT NULL PRIMARY KEY `Signature_`)",
    "CREATE TABLE `AppSearch` (`Property` CHAR(72) NOT NULL, `Signature_` CHAR(72) NOT NULL PRIMARY KEY `Property`)",
    // The search consults this table for the file and directory signatures it
    // knows; a package that searches only for a registry value leaves it empty,
    // but the search still reads it.
    "CREATE TABLE `Signature` (`Signature` CHAR(72) NOT NULL, `FileName` CHAR(255) NOT NULL, `MinVersion` CHAR(20), `MaxVersion` CHAR(20), `MinSize` LONG, `MaxSize` LONG, `MinDate` LONG, `MaxDate` LONG, `Languages` CHAR(255) PRIMARY KEY `Signature`)",
];

/// The name the setup image is stored under in the package's `Binary` table.
///
/// The Installer copies the stream to a temporary file and runs it, so the name
/// carries the extension the file is going to have; Microsoft's own samples name
/// their streams the same way.
const SETUP_STREAM: &str = "NanoInstallerSetup.exe";

/// The action that installs the product, and the one that takes it away.
const INSTALL_ACTION: &str = "NanoInstallerInstallProduct";
const UNINSTALL_COMMAND_ACTION: &str = "NanoInstallerReadUninstaller";
const UNINSTALL_ACTION: &str = "NanoInstallerRemoveProduct";

/// The public property the upgrade search fills with the products it found.
const UPGRADE_PROPERTY: &str = "NANOINSTALLERUPGRADEFOUND";

/// The public property the uninstall action reads the uninstaller from.
const UNINSTALLER_PROPERTY: &str = "NANOINSTALLERUNINSTALLER";

/// The name the search goes by, and the property it answers with.
const LOCATION_SIGNATURE: &str = "NanoInstallerLocation";
const LOCATION_PROPERTY: &str = "NANOINSTALLERLOCATION";

/// Summary-information property ids, the OLE property ids the Installer reads.
mod summary_property {
    pub(super) const CODE_PAGE: u32 = 1;
    pub(super) const TITLE: u32 = 2;
    pub(super) const SUBJECT: u32 = 3;
    pub(super) const AUTHOR: u32 = 4;
    pub(super) const KEYWORDS: u32 = 5;
    pub(super) const COMMENTS: u32 = 6;
    pub(super) const TEMPLATE: u32 = 7;
    pub(super) const REVISION: u32 = 9;
    pub(super) const CREATED: u32 = 12;
    pub(super) const LAST_SAVED: u32 = 13;
    pub(super) const PAGE_COUNT: u32 = 14;
    pub(super) const WORD_COUNT: u32 = 15;
    pub(super) const APPLICATION: u32 = 18;
    pub(super) const SECURITY: u32 = 19;
}

/// The variant types the summary information takes.
mod summary_type {
    pub(super) const SHORT: u32 = 2;
    pub(super) const TEXT: u32 = 30;
    pub(super) const TIME: u32 = 64;
}

/// What the wrapper needs to know about the project it packages.
pub(crate) struct Wrapper<'a> {
    /// The finished setup image the package carries and runs.
    pub setup: &'a Path,
    /// Where the package is written.
    pub output: &'a Path,
    pub product_name: &'a str,
    /// The project version, which may carry a release suffix.
    pub product_version: &'a str,
    pub manufacturer: &'a str,
    /// The locale whose code page the package's own text is stored in.
    pub locale: &'a str,
    /// The uninstaller's file name inside the installation directory.
    pub uninstaller_name: &'a str,
    /// Whether the wrapped setup asks Windows for administrator rights.
    pub require_admin: bool,
}

/// What writing the package came to, for the build log and the caller.
#[derive(Debug)]
pub(crate) struct WrapperSummary {
    pub output: PathBuf,
    pub size: u64,
    pub product_code: String,
    pub upgrade_code: String,
    /// The directory the package installs into unless a caller names another.
    pub install_directory: String,
    /// Whether the product is installed for the machine or for one user.
    pub per_machine: bool,
}

/// Refuses a project whose text this machine cannot store in a package.
///
/// The tables of a package are stored in the machine's ANSI code page, so a
/// product whose name that code page cannot hold would arrive as replacement
/// characters on every machine that opens the package. The check runs before a
/// build writes anything, so a project that cannot be packaged is told so
/// instead of being left with a setup nobody can deploy.
pub(crate) fn ensure_text_is_storable(
    product_name: &str,
    manufacturer: &str,
    uninstaller_name: &str,
) -> Result<()> {
    let code_page = machine_code_page();
    for (what, text) in [
        ("product name", product_name),
        ("publisher", manufacturer),
        ("uninstaller name", uninstaller_name),
    ] {
        if !code_page_holds(code_page, text) {
            bail!(
                "this build machine stores an installer package in code page {code_page}, which \
                 cannot hold the {what} \"{text}\"; build the package on a machine whose code page \
                 matches the product's language, or on one with the \"Beta: Use Unicode UTF-8 for \
                 worldwide language support\" option turned on"
            );
        }
    }
    Ok(())
}

pub(crate) fn write_wrapper(wrapper: &Wrapper<'_>) -> Result<WrapperSummary> {
    let version = version::installer_version(wrapper.product_version)?;
    let per_machine = wrapper.require_admin;
    // The product code names this release. It is derived from the version as the
    // project writes it, not from the three fields the Installer compares, so a
    // project that re-releases a version -- `2026.9.17-r2` after `2026.9.17` --
    // ships a product of its own instead of one the Installer would refuse as
    // "another version of this product is already installed" (1638), and the
    // upgrade search is what takes the release it replaces away. A rebuild of the
    // same version string keeps the code, so the machine can still repair or
    // remove what it installed.
    let product_code = derive_guid(&format!(
        "nano-installer product|{}|{}|{}",
        wrapper.manufacturer, wrapper.product_name, wrapper.product_version
    ))?;
    // The upgrade code names the product across versions, so a newer package
    // finds the older one; the product code names this version alone.
    let upgrade_code = derive_guid(&format!(
        "nano-installer upgrade|{}|{}",
        wrapper.manufacturer, wrapper.product_name
    ))?;
    let component_id = derive_guid(&format!("nano-installer wrapper|{product_code}"))?;
    // The package code names this build of this package, and the summary
    // information keeps it: deriving it from the image makes two packages that
    // carry the same setup share one code and a rebuilt image get a new one.
    let package_code = derive_guid(&format!(
        "nano-installer package|{}",
        crate::net::sha256_file(wrapper.setup)?
    ))?;
    let folder = directory_name(wrapper.product_name)?;
    let root_directory = if per_machine {
        "ProgramFiles64Folder"
    } else {
        "LocalAppDataFolder"
    };
    let install_directory = if per_machine {
        format!("%ProgramFiles%\\{}", wrapper.product_name)
    } else {
        format!("%LOCALAPPDATA%\\Programs\\{}", wrapper.product_name)
    };
    let tracking_key = format!(
        "Software\\{}\\{}",
        registry_component(wrapper.manufacturer),
        registry_component(wrapper.product_name)
    );
    // A directory property ends with a backslash, and a quote after a backslash
    // is an escaped quote to the parser Windows gives a program, so the path is
    // written as the directory itself plus a dot rather than ending in one.
    let setup_arguments = "--silent --dir \"[INSTALLDIR].\"".to_string();

    // The package's tables are stored in this machine's ANSI code page, so text
    // the machine cannot store would arrive as replacement characters. Refusing
    // is the only honest answer: the name would be unreadable on every machine
    // that opens the package.
    ensure_text_is_storable(
        wrapper.product_name,
        wrapper.manufacturer,
        wrapper.uninstaller_name,
    )?;
    let database = Database::create(wrapper.output)?;
    // The code page the package's own text is stored in is written before
    // anything is stored in it.
    write_summary_information(&database, wrapper, &version, &package_code)?;
    database.create_schema()?;
    write_property_table(
        &database,
        wrapper,
        &version,
        &product_code,
        &upgrade_code,
        per_machine,
    )?;
    write_directory_table(&database, root_directory, &folder)?;
    write_component_tables(&database, &component_id, &tracking_key, per_machine)?;
    // The package ships no files of its own -- the setup image is a stream, not
    // a file the Installer copies -- so it declares one empty medium, which is
    // what a package with nothing to lay out still needs to be a package.
    database.insert(
        "Media",
        &[
            "DiskId",
            "LastSequence",
            "DiskPrompt",
            "Cabinet",
            "VolumeLabel",
        ],
        &[
            Field::Number(1),
            Field::Number(1),
            Field::Null,
            Field::Null,
            Field::Null,
        ],
    )?;
    write_action_tables(
        &database,
        &install_action_type(per_machine),
        &setup_arguments,
        wrapper.uninstaller_name,
        per_machine,
    )?;
    write_sequence_tables(&database)?;
    write_upgrade_table(&database, &upgrade_code, &version)?;
    database.insert_stream(
        "Binary",
        &["Name", "Data"],
        &[Field::Text(SETUP_STREAM), Field::Stream(wrapper.setup)],
    )?;
    database.commit()?;
    drop(database);
    let size = std::fs::metadata(wrapper.output)
        .with_context(|| format!("failed to measure {}", wrapper.output.display()))?
        .len();
    Ok(WrapperSummary {
        output: wrapper.output.to_path_buf(),
        size,
        product_code,
        upgrade_code,
        install_directory,
        per_machine,
    })
}

fn install_action_type(per_machine: bool) -> u32 {
    let mut action = ACTION_EXE_FROM_BINARY + ACTION_IN_SCRIPT;
    if per_machine {
        action += ACTION_NO_IMPERSONATION;
    }
    action
}

fn uninstall_action_type(per_machine: bool) -> u32 {
    let mut action = ACTION_EXE_FROM_PROPERTY + ACTION_IN_SCRIPT;
    if per_machine {
        action += ACTION_NO_IMPERSONATION;
    }
    action
}

/// The rows that name the product, and the ones Windows reads to decide what
/// kind of installation this is.
fn write_property_table(
    database: &Database,
    wrapper: &Wrapper<'_>,
    version: &str,
    product_code: &str,
    upgrade_code: &str,
    per_machine: bool,
) -> Result<()> {
    let mut properties = vec![
        ("ProductName", wrapper.product_name.to_string()),
        ("ProductVersion", version.to_string()),
        ("ProductCode", product_code.to_string()),
        ("UpgradeCode", upgrade_code.to_string()),
        (
            "ProductLanguage",
            version::language_id(wrapper.locale).to_string(),
        ),
        ("Manufacturer", wrapper.manufacturer.to_string()),
        // The product registers itself with Windows, so the package must not
        // add a second entry to the same list.
        ("ARPSYSTEMCOMPONENT", "1".to_string()),
        // A directory property can be named on the command line, and the
        // install action reads it from the script, which needs it declared.
        (
            "SecureCustomProperties",
            format!("INSTALLDIR;{UPGRADE_PROPERTY};{LOCATION_PROPERTY};{UNINSTALLER_PROPERTY}"),
        ),
    ];
    if per_machine {
        properties.push(("ALLUSERS", "1".to_string()));
    } else {
        // Per-user unless the caller says otherwise, which is what lets an
        // administrator install for the machine on the command line.
        properties.push(("ALLUSERS", "2".to_string()));
        properties.push(("MSIINSTALLPERUSER", "1".to_string()));
    }
    for (name, value) in properties {
        insert_property(database, name, &value)?;
    }
    Ok(())
}

fn insert_property(database: &Database, name: &str, value: &str) -> Result<()> {
    database.insert(
        "Property",
        &["Property", "Value"],
        &[Field::Text(name), Field::Text(value)],
    )
}

/// Where the product goes: the package's own directory, under the root that
/// matches what the wrapped setup asks for.
fn write_directory_table(database: &Database, root: &str, folder: &str) -> Result<()> {
    let directories: [(&str, &str, &str); 4] = if root == "LocalAppDataFolder" {
        [
            ("TARGETDIR", "", "SourceDir"),
            ("LocalAppDataFolder", "TARGETDIR", "."),
            ("ProgramsFolder", "LocalAppDataFolder", "Programs"),
            ("INSTALLDIR", "ProgramsFolder", folder),
        ]
    } else {
        [
            ("TARGETDIR", "", "SourceDir"),
            ("ProgramFiles64Folder", "TARGETDIR", "."),
            ("INSTALLDIR", "ProgramFiles64Folder", folder),
            ("", "", ""),
        ]
    };
    for (directory, parent, default) in directories {
        if directory.is_empty() {
            continue;
        }
        database.insert(
            "Directory",
            &["Directory", "Directory_Parent", "DefaultDir"],
            &[
                Field::Text(directory),
                Field::Text(parent),
                Field::Text(default),
            ],
        )?;
    }
    Ok(())
}

/// The bookkeeping that makes Windows Installer treat the product as installed:
/// one component whose key path is a registry value this package owns.
fn write_component_tables(
    database: &Database,
    component_id: &str,
    tracking_key: &str,
    per_machine: bool,
) -> Result<()> {
    // 4 is a registry key path, 256 marks the component as 64-bit, which the
    // package's own template already says it is.
    const REGISTRY_KEY_PATH: i32 = 4;
    const COMPONENT_64_BIT: i32 = 256;
    database.insert(
        "Component",
        &[
            "Component",
            "ComponentId",
            "Directory_",
            "Attributes",
            "Condition",
            "KeyPath",
        ],
        &[
            Field::Text("NanoInstallerWrapper"),
            Field::Text(component_id),
            Field::Text("INSTALLDIR"),
            Field::Number(REGISTRY_KEY_PATH + COMPONENT_64_BIT),
            Field::Null,
            Field::Text("NanoInstallerTracking"),
        ],
    )?;
    database.insert(
        "CreateFolder",
        &["Directory_", "Component_"],
        &[
            Field::Text("INSTALLDIR"),
            Field::Text("NanoInstallerWrapper"),
        ],
    )?;
    database.insert(
        "Registry",
        &["Registry", "Root", "Key", "Name", "Value", "Component_"],
        &[
            Field::Text("NanoInstallerTracking"),
            // 2 is the machine, 1 the current user.
            Field::Number(if per_machine { 2 } else { 1 }),
            Field::Text(tracking_key),
            Field::Text("Installed"),
            Field::Text("1"),
            Field::Text("NanoInstallerWrapper"),
        ],
    )?;
    // The directory the product really went into, written where the removal can
    // find it again. The value is formatted by the Installer at install time, so
    // a directory named on the command line is what lands here.
    database.insert(
        "Registry",
        &["Registry", "Root", "Key", "Name", "Value", "Component_"],
        &[
            Field::Text("NanoInstallerLocation"),
            Field::Number(if per_machine { 2 } else { 1 }),
            Field::Text(tracking_key),
            Field::Text("InstallLocation"),
            Field::Text("[INSTALLDIR]"),
            Field::Text("NanoInstallerWrapper"),
        ],
    )?;
    database.insert(
        "RegLocator",
        &["Signature_", "Root", "Key", "Name", "Type"],
        &[
            Field::Text(LOCATION_SIGNATURE),
            Field::Number(if per_machine { 2 } else { 1 }),
            Field::Text(tracking_key),
            Field::Text("InstallLocation"),
            // 2 reads a raw registry value, 16 asks for the 64-bit view, which
            // is the view a 64-bit package's own component writes into.
            Field::Number(18),
        ],
    )?;
    database.insert(
        "AppSearch",
        &["Property", "Signature_"],
        &[
            Field::Text(LOCATION_PROPERTY),
            Field::Text(LOCATION_SIGNATURE),
        ],
    )?;
    database.insert(
        "Feature",
        &[
            "Feature",
            "Feature_Parent",
            "Title",
            "Description",
            "Display",
            "Level",
            "Directory_",
            "Attributes",
        ],
        &[
            Field::Text("NanoInstallerWrapper"),
            Field::Null,
            Field::Text("NanoInstallerWrapper"),
            Field::Text("The product this package installs"),
            Field::Number(2),
            Field::Number(1),
            Field::Text("INSTALLDIR"),
            Field::Number(0),
        ],
    )?;
    database.insert(
        "FeatureComponents",
        &["Feature_", "Component_"],
        &[
            Field::Text("NanoInstallerWrapper"),
            Field::Text("NanoInstallerWrapper"),
        ],
    )?;
    Ok(())
}

/// The two actions that install and remove the product, and the one that hands
/// the second the uninstaller's path.
fn write_action_tables(
    database: &Database,
    install_type: &u32,
    setup_arguments: &str,
    uninstaller_name: &str,
    per_machine: bool,
) -> Result<()> {
    let actions: [(&str, u32, &str, &str); 3] = [
        // The setup image comes out of the package's own Binary table.
        (INSTALL_ACTION, *install_type, SETUP_STREAM, setup_arguments),
        // An immediate action resolves the uninstaller's path while the
        // installation session still holds the properties, because the action
        // that runs the uninstaller is deferred and only sees what the script
        // was given. The path comes from the directory the package recorded when
        // it installed the product, which is where the product really is.
        (
            UNINSTALL_COMMAND_ACTION,
            ACTION_SET_PROPERTY,
            UNINSTALLER_PROPERTY,
            &format!("[{LOCATION_PROPERTY}]{uninstaller_name}"),
        ),
        // The uninstaller lives in the installation, not in the package, so the
        // path is read from the property the action above filled.
        (
            UNINSTALL_ACTION,
            uninstall_action_type(per_machine),
            UNINSTALLER_PROPERTY,
            "--silent",
        ),
    ];
    for (action, action_type, source, target) in actions {
        database.insert(
            "CustomAction",
            &["Action", "Type", "Source", "Target"],
            &[
                Field::Text(action),
                Field::Number(action_type as i32),
                Field::Text(source),
                Field::Text(target),
            ],
        )?;
    }
    Ok(())
}

/// The order the install and remove actions run in, among the standard actions
/// a package needs to be installable and removable.
///
/// The install action runs right after `InstallInitialize`, which is where the
/// directories and properties it needs are settled and still before the script
/// is executed; the remove action runs at the end, after Windows Installer has
/// taken back its own bookkeeping.
fn write_sequence_tables(database: &Database) -> Result<()> {
    // The product is only removed where the package knows it installed one,
    // which is what the search's answer says.
    let removal = format!("REMOVE=\"ALL\" AND {LOCATION_PROPERTY}");
    let execute: [(&str, &str, i32); 23] = [
        ("FindRelatedProducts", "", 200),
        // Where the product is only becomes known from the record this package
        // wrote when it installed it, so the search runs before anything reads
        // it -- and before Windows Installer takes its own values back.
        ("AppSearch", "", 400),
        ("CostInitialize", "", 800),
        ("FileCost", "", 900),
        ("CostFinalize", "", 1000),
        ("InstallValidate", "", 1400),
        // An older release goes before this one arrives: the wrapped setup
        // installs over whatever it finds, so leaving the older product in place
        // until the end would have its removal take the new one away again.
        ("RemoveExistingProducts", "", 1450),
        ("InstallInitialize", "", 1500),
        ("NanoInstallerInstallProduct", "NOT REMOVE", 1551),
        ("ProcessComponents", "", 1600),
        ("UnpublishFeatures", "", 1800),
        ("RemoveRegistryValues", "", 2600),
        ("RemoveShortcuts", "", 3200),
        ("RemoveFiles", "", 3500),
        ("RemoveFolders", "", 3600),
        ("InstallFiles", "", 4000),
        ("WriteRegistryValues", "", 5000),
        ("RegisterUser", "", 6000),
        ("RegisterProduct", "", 6100),
        ("PublishFeatures", "", 6300),
        ("PublishProduct", "", 6400),
        (UNINSTALL_ACTION, removal.as_str(), 6499),
        ("InstallFinalize", "", 6600),
    ];
    for (action, condition, sequence) in execute {
        database.insert(
            "InstallExecuteSequence",
            &["Action", "Condition", "Sequence"],
            &[
                Field::Text(action),
                Field::Text(condition),
                Field::Number(sequence),
            ],
        )?;
    }
    // The immediate action that reads the uninstaller's path belongs with the
    // deferred one it feeds, and only in an uninstall of a product this package
    // installed: without the search's answer there is nothing to remove.
    database.insert(
        "InstallExecuteSequence",
        &["Action", "Condition", "Sequence"],
        &[
            Field::Text(UNINSTALL_COMMAND_ACTION),
            Field::Text(removal.as_str()),
            Field::Number(6498),
        ],
    )?;
    let user_interface: [(&str, i32); 6] = [
        ("FindRelatedProducts", 200),
        ("AppSearch", 400),
        ("CostInitialize", 800),
        ("FileCost", 900),
        ("CostFinalize", 1000),
        ("ExecuteAction", 1300),
    ];
    for (action, sequence) in user_interface {
        database.insert(
            "InstallUISequence",
            &["Action", "Condition", "Sequence"],
            &[Field::Text(action), Field::Null, Field::Number(sequence)],
        )?;
    }
    let administrative: [&str; 6] = [
        "CostInitialize",
        "FileCost",
        "CostFinalize",
        "InstallValidate",
        "InstallInitialize",
        "InstallFinalize",
    ];
    let sequences: [i32; 6] = [800, 900, 1000, 1400, 1500, 6600];
    for (action, sequence) in administrative.iter().zip(sequences) {
        database.insert(
            "AdminExecuteSequence",
            &["Action", "Condition", "Sequence"],
            &[Field::Text(action), Field::Null, Field::Number(sequence)],
        )?;
    }
    let advertised: [(&str, i32); 7] = [
        ("CostInitialize", 800),
        ("CostFinalize", 1000),
        ("InstallValidate", 1400),
        ("InstallInitialize", 1500),
        ("PublishFeatures", 6300),
        ("PublishProduct", 6400),
        ("InstallFinalize", 6600),
    ];
    for (action, sequence) in advertised {
        database.insert(
            "AdvtExecuteSequence",
            &["Action", "Condition", "Sequence"],
            &[Field::Text(action), Field::Null, Field::Number(sequence)],
        )?;
    }
    Ok(())
}

/// The search that finds the release this package replaces.
///
/// The upper bound is the version being installed and **includes** it. Windows
/// Installer compares three fields, so a project that re-releases a version --
/// `2026.9.17-r2` after `2026.9.17` -- ships a package whose three fields are the
/// ones already on the machine; a bound that excluded them would leave that
/// release installed and let this one sit beside it, or, because two packages
/// that share a product code may not differ in version, have the Installer refuse
/// the second one with "another version of this product is already installed".
/// A re-release is a product of its own (the product code is derived from the
/// version as written), so the row cannot match the product being installed, and
/// the removal happens before this setup runs.
///
/// The three nullable columns are left null rather than empty: an empty `Remove`
/// field removes no features at all, while a null one is what makes the Installer
/// take the whole older product away, and a null lower bound is what makes the
/// search cover every earlier version.
fn write_upgrade_table(database: &Database, upgrade_code: &str, version: &str) -> Result<()> {
    // 1 migrates feature states, which an older release of this wrapper has none
    // of; 512 is the inclusive upper bound (2 would be detect-only, which finds
    // the product and takes nothing away); 256 would include an explicit lower
    // bound, and there is none.
    const MIGRATE_FEATURES: i32 = 1;
    const VERSION_MAX_INCLUSIVE: i32 = 512;
    database.insert(
        "Upgrade",
        &[
            "UpgradeCode",
            "VersionMin",
            "VersionMax",
            "Language",
            "Attributes",
            "Remove",
            "ActionProperty",
        ],
        &[
            Field::Text(upgrade_code),
            Field::Null,
            Field::Text(version),
            Field::Null,
            Field::Number(MIGRATE_FEATURES | VERSION_MAX_INCLUSIVE),
            Field::Null,
            Field::Text(UPGRADE_PROPERTY),
        ],
    )
}

/// The properties Windows Explorer and the installer itself read out of the
/// package before its tables are opened.
fn write_summary_information(
    database: &Database,
    wrapper: &Wrapper<'_>,
    version: &str,
    package_code: &str,
) -> Result<()> {
    let mut summary = MSIHANDLE::default();
    check(
        unsafe {
            MsiGetSummaryInformationW(
                database.handle(),
                PCWSTR::null(),
                20,
                &mut summary as *mut MSIHANDLE,
            )
        },
        "open the summary information of the package",
    )?;
    let code_page = summary_code_page(
        wrapper.locale,
        &[wrapper.product_name, wrapper.manufacturer],
    );
    // The summary is what a file listing shows; the product's own name lives in
    // the `Property` table, which is what Windows' list of installed programs
    // reads. The package therefore declares the code page its tables are stored
    // in, and the summary follows it.
    let subject = format!("{} {version}", wrapper.product_name);
    let comments = format!(
        "This package installs {} with the setup image it carries.",
        wrapper.product_name
    );
    // The code page goes first: it is the one the package's own text is stored
    // in, and everything written after it is stored in that code page.
    write_summary_number(&summary, summary_property::CODE_PAGE, code_page as i32)?;
    write_summary_text(&summary, summary_property::TITLE, "Installation Database")?;
    write_summary_text(&summary, summary_property::SUBJECT, &subject)?;
    write_summary_text(&summary, summary_property::AUTHOR, wrapper.manufacturer)?;
    write_summary_text(&summary, summary_property::KEYWORDS, "Installer")?;
    write_summary_text(&summary, summary_property::COMMENTS, &comments)?;
    // The template names the platform and the language of the package.
    write_summary_text(&summary, summary_property::TEMPLATE, "x64;1033")?;
    write_summary_text(&summary, summary_property::REVISION, package_code)?;
    write_summary_number(&summary, summary_property::PAGE_COUNT, 200)?;
    write_summary_number(&summary, summary_property::WORD_COUNT, 2)?;
    write_summary_text(&summary, summary_property::APPLICATION, "nano-installer")?;
    write_summary_number(&summary, summary_property::SECURITY, 2)?;
    let now = file_time_now();
    for property in [summary_property::CREATED, summary_property::LAST_SAVED] {
        write_summary_time(&summary, property, now)?;
    }
    check(
        unsafe { MsiSummaryInfoPersist(summary) },
        "save the summary information",
    )?;
    let _ = unsafe { MsiCloseHandle(summary) };
    Ok(())
}

/// Writes one numeric summary-information property.
fn write_summary_number(summary: &MSIHANDLE, property: u32, value: i32) -> Result<()> {
    check(
        unsafe {
            MsiSummaryInfoSetPropertyW(
                *summary,
                property,
                summary_type::SHORT,
                value,
                std::ptr::null_mut(),
                PCWSTR::null(),
            )
        },
        "write a summary-information number",
    )
}

/// Writes one text summary-information property.
///
/// A product name, a publisher or a comment that the package's code page cannot
/// hold is cosmetic: the product's own name lives in the `Property` table, which
/// is Unicode and is what Windows' list of installed products reads. Dropping
/// the text keeps the package; refusing the build over a tooltip would not.
fn write_summary_text(summary: &MSIHANDLE, property: u32, text: &str) -> Result<()> {
    let text = wide(text);
    let code = unsafe {
        MsiSummaryInfoSetPropertyW(
            *summary,
            property,
            summary_type::TEXT,
            0,
            std::ptr::null_mut(),
            PCWSTR::from_raw(text.as_ptr()),
        )
    };
    if code == ERROR_SUCCESS {
        return Ok(());
    }
    check(
        unsafe {
            MsiSummaryInfoSetPropertyW(
                *summary,
                property,
                summary_type::TEXT,
                0,
                std::ptr::null_mut(),
                PCWSTR::null(),
            )
        },
        "clear a summary-information string the package's code page cannot hold",
    )
}

/// Writes one summary-information timestamp.
fn write_summary_time(summary: &MSIHANDLE, property: u32, value: FILETIME) -> Result<()> {
    let mut value = value;
    check(
        unsafe {
            MsiSummaryInfoSetPropertyW(
                *summary,
                property,
                summary_type::TIME,
                0,
                &mut value as *mut FILETIME,
                PCWSTR::null(),
            )
        },
        "write a summary-information timestamp",
    )
}

/// The code page a package's own text is stored in.
///
/// Not the one the package declares: this is what Windows Installer measured
/// here. A database created by this build stores its strings in the machine's
/// ANSI code page whatever code page the summary information names -- a package
/// written on a machine with the UTF-8 option turned on held a Chinese name as
/// UTF-8 while its summary said 936, and a `_ForceCodepage` row naming another
/// code page changed nothing. The package therefore declares the code page its
/// strings really are in, and a name that code page cannot hold is refused
/// rather than stored as one replacement character per character, which is what
/// the Installer writes and what nobody can read back.
fn machine_code_page() -> u16 {
    use windows::Win32::Globalization::GetACP;
    unsafe { GetACP() as u16 }
}

/// The code page the summary information is stored in.
///
/// It is the locale's own code page where the machine can put this package's
/// words into it, and the Latin one otherwise -- never UTF-8: the Installer
/// refuses a summary-information string written in 65001, and the template that
/// says which platform the package is for is one of them.
fn summary_code_page(locale: &str, texts: &[&str]) -> u16 {
    let code_page = locale_code_page(locale);
    if texts.iter().all(|text| code_page_holds(code_page, text)) {
        code_page
    } else {
        1252
    }
}

/// A code page an installation authoring tool would pick for a locale.
fn locale_code_page(locale: &str) -> u16 {
    match locale.to_ascii_lowercase().as_str() {
        "zh-cn" => 936,
        "zh-tw" => 950,
        "ja" | "ja-jp" => 932,
        "ko" | "ko-kr" => 949,
        "ru" | "ru-ru" => 1251,
        "pl" | "pl-pl" | "cs" | "cs-cz" | "hu" | "hu-hu" | "ro" | "ro-ro" => 1250,
        "tr" | "tr-tr" => 1254,
        "el" | "el-gr" => 1253,
        "he" | "he-il" => 1255,
        "ar" | "ar-sa" => 1256,
        "th" | "th-th" => 874,
        "vi" | "vi-vn" => 1258,
        _ => 1252,
    }
}

/// Whether a code page can hold every character of a text.
///
/// Windows answers this while converting: a character the code page has no room
/// for is replaced by a default character, and the conversion says whether it had
/// to do that. A code page the machine does not have at all cannot hold anything.
fn code_page_holds(code_page: u16, text: &str) -> bool {
    use windows::Win32::Globalization::WideCharToMultiByte;
    if text.is_ascii() {
        return true;
    }
    let wide: Vec<u16> = text.encode_utf16().collect();
    let mut substituted = windows::Win32::Foundation::BOOL(0);
    let written = unsafe {
        WideCharToMultiByte(
            u32::from(code_page),
            0,
            &wide,
            None,
            windows::core::PCSTR::null(),
            Some(&mut substituted as *mut _),
        )
    };
    written > 0 && !substituted.as_bool()
}

/// A directory name the installer can use for the product's own folder.
///
/// Windows Installer wants a short name and a long one, separated by a bar, for
/// anything the 8.3 rules cannot express, and the short name has to be made of
/// the ASCII characters a file system keeps. A name that is already short and
/// plain is written as it is; anything else gets a short name built from its own
/// letters and digits, so a product whose name is not written in the Latin
/// alphabet still has a package.
fn directory_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        bail!("the product name is empty, so the package has no directory to install into");
    }
    if let Some(bad) = trimmed.chars().find(|character| {
        matches!(
            character,
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
        )
    }) {
        bail!("the product name {trimmed} cannot name a directory: it contains {bad}");
    }
    if is_a_plain_short_name(trimmed) {
        return Ok(trimmed.to_string());
    }
    let letters: String = trimmed
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_uppercase())
        .take(8)
        .collect();
    let letters = if letters.is_empty() {
        "NANOAPP".to_string()
    } else {
        letters
    };
    Ok(format!(
        "{}{}|{}",
        letters,
        short_name_stamp(trimmed)?,
        trimmed
    ))
}

/// Whether a name is already the short form a file system keeps.
fn is_a_plain_short_name(name: &str) -> bool {
    if !name.is_ascii() || name.len() > 8 || name.contains(' ') {
        return false;
    }
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return false;
    }
    // A name Windows keeps for a device is not a name a directory can take.
    let upper = name.to_ascii_uppercase();
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    !RESERVED.contains(&upper.as_str())
}

/// The two characters that keep two long names apart when their first eight
/// letters agree: the name's own digits where it has them, and a digest of the
/// whole name where it does not.
fn short_name_stamp(name: &str) -> Result<String> {
    let digits: String = name.chars().filter(char::is_ascii_digit).take(2).collect();
    if digits.len() == 2 {
        return Ok(digits);
    }
    let digest = crate::net::sha256_bytes(name.as_bytes())?;
    Ok(digest[..2]
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>()[..4]
        .to_string())
}

/// The part of a registry path one name contributes.
///
/// A key path cannot hold a separator, so the one character a name may not bring
/// with it is replaced; everything else, including a name that is not written in
/// the Latin alphabet, a registry key holds as it is.
fn registry_component(name: &str) -> String {
    let trimmed = name.trim();
    let trimmed = if trimmed.is_empty() {
        "nano-installer"
    } else {
        trimmed
    };
    trimmed
        .chars()
        .map(|character| match character {
            '\\' | '/' => '_',
            other => other,
        })
        .collect()
}

/// A GUID that is the same for the same seed and differs for a different one.
///
/// The installer identifies a product by its codes, so they have to survive a
/// rebuild: a fresh random pair on every build would leave a machine that
/// installed the previous one unable to upgrade or remove it.
fn derive_guid(seed: &str) -> Result<String> {
    let mut digest = crate::net::sha256_bytes(seed.as_bytes())?;
    digest[6] = (digest[6] & 0x0F) | 0x40;
    digest[8] = (digest[8] & 0x3F) | 0x80;
    let hex: String = digest.iter().map(|byte| format!("{byte:02X}")).collect();
    Ok(format!(
        "{{{}-{}-{}-{}-{}}}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

/// The moment this build ran, in the form the summary information keeps time.
fn file_time_now() -> FILETIME {
    use std::time::{SystemTime, UNIX_EPOCH};
    // FILETIME counts 100-nanosecond intervals since 1601, which is the Unix
    // epoch shifted by 369 years.
    const EPOCH_SHIFT_SECONDS: u64 = 11_644_473_600;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let ticks =
        (now.as_secs() + EPOCH_SHIFT_SECONDS) * 10_000_000 + u64::from(now.subsec_nanos()) / 100;
    FILETIME {
        dwLowDateTime: (ticks & 0xFFFF_FFFF) as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// One value of a row, in the shape its column takes.
///
/// A nullable column takes a null rather than an empty string, and for the
/// upgrade search the difference changes what the Installer does: an empty
/// `Remove` field removes nothing.
enum Field<'a> {
    Text(&'a str),
    Number(i32),
    Null,
    Stream(&'a Path),
}

/// An open package database, closed when the build stops looking at it.
struct Database {
    handle: MSIHANDLE,
}

impl Database {
    fn create(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        // A package this build wrote before is replaced rather than added to:
        // the Installer would otherwise keep rows nobody asked for.
        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("failed to replace {}", path.display()))?;
        }
        let path_wide = wide(&path.to_string_lossy());
        let mut handle = MSIHANDLE::default();
        check(
            unsafe {
                MsiOpenDatabaseW(
                    PCWSTR::from_raw(path_wide.as_ptr()),
                    PCWSTR::from_raw(MSIDBOPEN_CREATE as *const u16),
                    &mut handle as *mut MSIHANDLE,
                )
            },
            "create the package database",
        )?;
        Ok(Self { handle })
    }

    fn handle(&self) -> MSIHANDLE {
        self.handle
    }

    /// Declares the tables this package writes.
    ///
    /// A database created by this build starts with no tables at all, so the
    /// package states the schema it uses; a statement that builds no result set
    /// runs through a view of its own and nothing else.
    fn create_schema(&self) -> Result<()> {
        for statement in SCHEMA {
            self.execute(statement, MSIHANDLE::default())
                .with_context(|| {
                    format!("failed to declare the tables of the package: {statement}")
                })?;
        }
        Ok(())
    }

    /// Runs one statement through a view of its own.
    fn execute(&self, statement: &str, record: MSIHANDLE) -> Result<()> {
        let statement = wide(statement);
        let mut view = MSIHANDLE::default();
        check(
            unsafe {
                MsiDatabaseOpenViewW(
                    self.handle,
                    PCWSTR::from_raw(statement.as_ptr()),
                    &mut view as *mut MSIHANDLE,
                )
            },
            "open a view on the package database",
        )?;
        let executed = check(
            unsafe { MsiViewExecute(view, record) },
            "run a statement against the package database",
        );
        let _ = unsafe { MsiViewClose(view) };
        let _ = unsafe { MsiCloseHandle(view) };
        executed
    }

    /// Adds one row to a table.
    ///
    /// The Installer's SQL takes a row through `INSERT` with a marker for every
    /// value, and the values arrive in the record the view is executed with.
    /// Binary data is the one thing this path cannot carry, which is why the
    /// setup image has an entry of its own.
    fn insert(&self, table: &str, columns: &[&str], fields: &[Field<'_>]) -> Result<()> {
        assert_eq!(columns.len(), fields.len());
        let markers = vec!["?"; columns.len()].join(", ");
        let statement = format!(
            "INSERT INTO `{table}` ({}) VALUES ({markers})",
            columns
                .iter()
                .map(|column| format!("`{column}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let record = self.record(table, fields)?;
        let written = self.execute(&statement, record);
        let _ = unsafe { MsiCloseHandle(record) };
        written
    }

    /// Adds the row whose column holds a file.
    ///
    /// Binary data cannot go in through `INSERT`, so the row is added through a
    /// view of the table and `MsiViewModify`, which is the documented way to put
    /// a stream into a package.
    fn insert_stream(&self, table: &str, columns: &[&str], fields: &[Field<'_>]) -> Result<()> {
        assert_eq!(columns.len(), fields.len());
        let statement = format!(
            "SELECT {} FROM `{table}`",
            columns
                .iter()
                .map(|column| format!("`{column}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let statement = wide(&statement);
        let mut view = MSIHANDLE::default();
        check(
            unsafe {
                MsiDatabaseOpenViewW(
                    self.handle,
                    PCWSTR::from_raw(statement.as_ptr()),
                    &mut view as *mut MSIHANDLE,
                )
            },
            "open a view on the package database",
        )?;
        let record = self.record(table, fields)?;
        let written = check(
            unsafe { MsiViewExecute(view, MSIHANDLE::default()) },
            "open the table the stream goes into",
        )
        .and_then(|()| {
            check(
                unsafe { MsiViewModify(view, MSIMODIFY_INSERT, record) },
                &format!("write the row that carries the setup image into {table}"),
            )
        });
        let _ = unsafe { MsiCloseHandle(record) };
        let _ = unsafe { MsiViewClose(view) };
        let _ = unsafe { MsiCloseHandle(view) };
        written
    }

    /// Fills a record with one row's values.
    fn record(&self, table: &str, fields: &[Field<'_>]) -> Result<MSIHANDLE> {
        let record = unsafe { MsiCreateRecord(fields.len() as u32) };
        if record == MSIHANDLE::default() {
            bail!("failed to create a record for the package database");
        }
        for (index, field) in fields.iter().enumerate() {
            let position = index as u32 + 1;
            let code = unsafe {
                match field {
                    Field::Text(text) => {
                        let text = wide(text);
                        MsiRecordSetStringW(record, position, PCWSTR::from_raw(text.as_ptr()))
                    }
                    Field::Number(number) => MsiRecordSetInteger(record, position, *number),
                    Field::Null => MsiRecordSetStringW(record, position, PCWSTR::null()),
                    Field::Stream(path) => {
                        let path = wide(&path.to_string_lossy());
                        MsiRecordSetStreamW(record, position, PCWSTR::from_raw(path.as_ptr()))
                    }
                }
            };
            if code != ERROR_SUCCESS {
                let _ = unsafe { MsiCloseHandle(record) };
                bail!(
                    "failed to fill field {position} of a {table} row: {}",
                    describe_code(code)
                );
            }
        }
        Ok(record)
    }

    fn commit(&self) -> Result<()> {
        check(
            unsafe { MsiDatabaseCommit(self.handle) },
            "save the package database",
        )
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        if self.handle != MSIHANDLE::default() {
            let _ = unsafe { MsiCloseHandle(self.handle) };
        }
    }
}

/// Turns the Installer's numeric answer into something a build log can carry.
fn describe_code(code: u32) -> String {
    let text = std::io::Error::from_raw_os_error(code as i32).to_string();
    if text.is_empty() {
        format!("error {code}")
    } else {
        format!("error {code}: {text}")
    }
}

fn check(code: u32, what: &str) -> Result<()> {
    if code == ERROR_SUCCESS {
        return Ok(());
    }
    bail!("failed to {what}: {}", describe_code(code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::ApplicationInstallationAndServicing::{
        MsiDatabaseOpenViewW, MsiOpenDatabaseW, MsiRecordGetStringW, MsiSummaryInfoGetPropertyW,
        MsiViewFetch,
    };

    /// What a fetch reports when the view has no rows left.
    const ERROR_NO_MORE_ITEMS: u32 = 259;

    /// The mode for opening a package to read it back.
    const MSIDBOPEN_READONLY: usize = 0;

    fn scratch(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "nano-installer-msi-{}-{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        directory
    }

    /// A stand-in for the setup a package carries.
    fn setup_image(directory: &Path) -> PathBuf {
        let setup = directory.join("Probe_Setup.exe");
        std::fs::write(&setup, b"MZ this stands in for a setup image\r\n").expect("a setup image");
        setup
    }

    fn wrapper<'a>(
        setup: &'a Path,
        output: &'a Path,
        name: &'a str,
        version: &'a str,
    ) -> Wrapper<'a> {
        Wrapper {
            setup,
            output,
            product_name: name,
            product_version: version,
            manufacturer: "nano-installer tests",
            locale: "en-US",
            uninstaller_name: "uninst.exe",
            require_admin: false,
        }
    }

    /// Reads rows out of a package the way any other tool would: open the
    /// database, run the query, and take the fields of every row.
    fn read_rows(package: &Path, query: &str, fields: usize) -> Result<Vec<Vec<String>>> {
        let path = wide(&package.to_string_lossy());
        let mut database = MSIHANDLE::default();
        check(
            unsafe {
                MsiOpenDatabaseW(
                    PCWSTR::from_raw(path.as_ptr()),
                    PCWSTR::from_raw(MSIDBOPEN_READONLY as *const u16),
                    &mut database as *mut MSIHANDLE,
                )
            },
            "open the package this build wrote",
        )?;
        let query = wide(query);
        let mut view = MSIHANDLE::default();
        check(
            unsafe {
                MsiDatabaseOpenViewW(
                    database,
                    PCWSTR::from_raw(query.as_ptr()),
                    &mut view as *mut MSIHANDLE,
                )
            },
            "open a view on the package this build wrote",
        )?;
        check(
            unsafe { MsiViewExecute(view, MSIHANDLE::default()) },
            "run the query",
        )?;
        let mut rows = Vec::new();
        loop {
            let mut record = MSIHANDLE::default();
            let code = unsafe { MsiViewFetch(view, &mut record as *mut MSIHANDLE) };
            if code == ERROR_NO_MORE_ITEMS {
                break;
            }
            check(code, "read a row of the package")?;
            let mut row = Vec::new();
            for field in 1..=fields as u32 {
                let mut buffer = [0u16; 512];
                let mut length = buffer.len() as u32;
                check(
                    unsafe {
                        MsiRecordGetStringW(
                            record,
                            field,
                            windows::core::PWSTR::from_raw(buffer.as_mut_ptr()),
                            Some(&mut length as *mut u32),
                        )
                    },
                    "read a field of a package row",
                )?;
                row.push(String::from_utf16_lossy(&buffer[..length as usize]));
            }
            rows.push(row);
            let _ = unsafe { MsiCloseHandle(record) };
        }
        let _ = unsafe { MsiViewClose(view) };
        let _ = unsafe { MsiCloseHandle(view) };
        let _ = unsafe { MsiCloseHandle(database) };
        Ok(rows)
    }

    fn single_value(package: &Path, query: &str) -> Result<String> {
        let rows = read_rows(package, query, 1)?;
        Ok(rows
            .first()
            .and_then(|row| row.first())
            .cloned()
            .unwrap_or_default())
    }

    fn summary_text(package: &Path, property: u32) -> Result<String> {
        let path = wide(&package.to_string_lossy());
        let mut summary = MSIHANDLE::default();
        check(
            unsafe {
                MsiGetSummaryInformationW(
                    MSIHANDLE::default(),
                    PCWSTR::from_raw(path.as_ptr()),
                    0,
                    &mut summary as *mut MSIHANDLE,
                )
            },
            "open the summary information of the package this build wrote",
        )?;
        let mut kind = 0u32;
        let mut number = 0i32;
        let mut buffer = [0u16; 512];
        let mut length = buffer.len() as u32;
        let code = unsafe {
            MsiSummaryInfoGetPropertyW(
                summary,
                property,
                &mut kind,
                &mut number,
                None,
                windows::core::PWSTR::from_raw(buffer.as_mut_ptr()),
                Some(&mut length as *mut u32),
            )
        };
        let _ = unsafe { MsiCloseHandle(summary) };
        check(code, "read a summary-information property")?;
        Ok(String::from_utf16_lossy(&buffer[..length as usize]))
    }

    #[test]
    fn a_wrapper_carries_the_identity_the_project_declares() -> Result<()> {
        let directory = scratch("identity");
        let setup = setup_image(&directory);
        let package = directory.join("Probe.msi");
        let summary = write_wrapper(&wrapper(&setup, &package, "Probe", "3.4.1"))?;

        assert_eq!(
            single_value(
                &package,
                "SELECT `Value` FROM `Property` WHERE `Property`='ProductName'"
            )?,
            "Probe"
        );
        assert_eq!(
            single_value(
                &package,
                "SELECT `Value` FROM `Property` WHERE `Property`='ProductVersion'"
            )?,
            "3.4.1"
        );
        assert_eq!(
            single_value(
                &package,
                "SELECT `Value` FROM `Property` WHERE `Property`='ProductCode'"
            )?,
            summary.product_code
        );
        assert_eq!(
            single_value(
                &package,
                "SELECT `Value` FROM `Property` WHERE `Property`='UpgradeCode'"
            )?,
            summary.upgrade_code
        );
        // A setup that needs no administrator installs for one user, which is
        // what lets a machine without rights take the package at all.
        assert_eq!(
            single_value(
                &package,
                "SELECT `Value` FROM `Property` WHERE `Property`='ALLUSERS'"
            )?,
            "2"
        );
        // The product registers itself, so the package must not add a second
        // entry to the list of installed programs.
        assert_eq!(
            single_value(
                &package,
                "SELECT `Value` FROM `Property` WHERE `Property`='ARPSYSTEMCOMPONENT'"
            )?,
            "1"
        );
        assert!(summary.product_code.starts_with('{') && summary.product_code.ends_with('}'));
        assert!(!summary.per_machine);
        Ok(())
    }

    #[test]
    fn the_setup_the_package_carries_is_the_image_the_build_finished() -> Result<()> {
        let directory = scratch("stream");
        let setup = setup_image(&directory);
        let package = directory.join("Probe.msi");
        write_wrapper(&wrapper(&setup, &package, "Probe", "1.0.0"))?;

        let rows = read_rows(&package, "SELECT `Name` FROM `Binary`", 1)?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], SETUP_STREAM);
        // The action that installs the product reads that stream, and the one
        // that removes it runs the uninstaller the setup deployed.
        let actions = read_rows(
            &package,
            "SELECT `Action`, `Type`, `Source`, `Target` FROM `CustomAction` ORDER BY `Action`",
            4,
        )?;
        let types: Vec<(&str, &str)> = actions
            .iter()
            .map(|row| (row[0].as_str(), row[1].as_str()))
            .collect();
        // 2 + 1024 is an executable out of the package, queued for the script,
        // and 50 + 1024 one a property names.
        assert!(types.contains(&(INSTALL_ACTION, "1026")));
        assert!(types.contains(&(UNINSTALL_ACTION, "1074")));
        assert!(types.contains(&(UNINSTALL_COMMAND_ACTION, "51")));
        let install = actions
            .iter()
            .find(|row| row[0] == INSTALL_ACTION)
            .expect("the install action");
        assert_eq!(install[2], SETUP_STREAM);
        assert!(install[3].starts_with("--silent --dir"));
        Ok(())
    }

    #[test]
    fn the_package_installs_and_removes_the_product_in_the_order_it_declares() -> Result<()> {
        let directory = scratch("sequence");
        let setup = setup_image(&directory);
        let package = directory.join("Probe.msi");
        write_wrapper(&wrapper(&setup, &package, "Probe", "1.0.0"))?;

        let rows = read_rows(
            &package,
            "SELECT `Action`, `Condition`, `Sequence` FROM `InstallExecuteSequence` ORDER BY `Sequence`",
            3,
        )?;
        let order: Vec<&str> = rows.iter().map(|row| row[0].as_str()).collect();
        let position = |name: &str| {
            order
                .iter()
                .position(|action| *action == name)
                .unwrap_or_else(|| panic!("{name} is in the sequence"))
        };
        let install = position(INSTALL_ACTION);
        let remove = position(UNINSTALL_ACTION);
        // The older release goes before this one arrives, the product is
        // installed inside the script's own window, and the uninstaller runs
        // after Windows Installer has taken its own bookkeeping back.
        assert!(position("RemoveExistingProducts") < position("InstallInitialize"));
        assert!(position("InstallInitialize") < install);
        assert!(install < position("InstallFinalize"));
        assert!(remove < position("InstallFinalize"));
        assert!(position("InstallFiles") < remove);
        // The install action only runs when the product is not being removed.
        let install_row = &rows[install];
        assert_eq!(install_row[1], "NOT REMOVE");
        let remove_row = &rows[remove];
        // The product is only removed where the package knows it installed one,
        // which is what the search's answer says.
        assert_eq!(
            remove_row[1],
            format!("REMOVE=\"ALL\" AND {LOCATION_PROPERTY}")
        );
        Ok(())
    }

    #[test]
    fn the_package_offers_itself_as_an_upgrade_of_the_versions_before_it() -> Result<()> {
        let directory = scratch("upgrade");
        let setup = setup_image(&directory);
        let package = directory.join("Probe.msi");
        write_wrapper(&wrapper(&setup, &package, "Probe", "3.4.1"))?;

        let rows = read_rows(
            &package,
            "SELECT `UpgradeCode`, `VersionMax`, `Attributes`, `ActionProperty` FROM `Upgrade`",
            4,
        )?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][1], "3.4.1");
        // The bound includes the version it names: a re-release of one day
        // carries the three fields already on the machine, and a bound that
        // excluded them would leave that release installed.
        assert_eq!(rows[0][2], "513");
        assert_eq!(rows[0][3], UPGRADE_PROPERTY);
        // The search fills a property the script carries into the server side,
        // which only happens for a property named as secure.
        let secure = single_value(
            &package,
            "SELECT `Value` FROM `Property` WHERE `Property`='SecureCustomProperties'",
        )?;
        assert!(secure.contains(UPGRADE_PROPERTY));
        assert!(secure.contains("INSTALLDIR"));
        // An empty removal list would take nothing away, so the column is null.
        let removal = read_rows(&package, "SELECT `Remove` FROM `Upgrade`", 1)?;
        assert_eq!(removal[0][0], "");
        Ok(())
    }

    /// A code page without room for a name cannot be talked into holding it.
    ///
    /// This is the fact the build's own check rests on, and it does not depend on
    /// what the machine running the case has installed.
    #[test]
    fn a_latin_code_page_cannot_hold_a_chinese_name() {
        assert!(!code_page_holds(1252, "易玩安装器"));
        assert!(code_page_holds(1252, "Widget Factory"));
    }

    /// The product's own name is what Windows' list of installed programs shows,
    /// so the package either stores it whole or refuses to be built at all.
    ///
    /// Which of the two happens depends on the code page this machine stores a
    /// package in, and the case holds the build to the one that applies here: a
    /// machine that can store the name stores it unchanged, and one that cannot
    /// is refused rather than handed a package whose name is a row of replacement
    /// characters.
    #[test]
    fn a_product_name_is_stored_whole_or_the_build_is_refused() -> Result<()> {
        let directory = scratch("unicode");
        let setup = setup_image(&directory);
        let package = directory.join("Probe.msi");
        let mut wrapper = wrapper(&setup, &package, "易玩安装器", "2026.9.22");
        wrapper.locale = "zh-CN";
        let storable = code_page_holds(machine_code_page(), "易玩安装器");
        let built = write_wrapper(&wrapper);
        match (built, storable) {
            (Ok(_), true) => assert_eq!(
                single_value(
                    &package,
                    "SELECT `Value` FROM `Property` WHERE `Property`='ProductName'"
                )?,
                "易玩安装器",
                "the name the machine can store did not come back"
            ),
            (Err(error), false) => {
                let text = format!("{error:#}");
                assert!(
                    text.contains("code page"),
                    "the refusal does not say what the machine stores: {text}"
                );
            }
            (Ok(_), false) => panic!("a machine that cannot store the name stored it anyway"),
            (Err(error), true) => {
                panic!("a name the machine can store was refused: {error:#}")
            }
        }
        Ok(())
    }

    #[test]
    fn the_package_summary_names_the_platform_and_the_language() -> Result<()> {
        let directory = scratch("summary");
        let setup = setup_image(&directory);
        let package = directory.join("Probe.msi");
        let summary = write_wrapper(&wrapper(&setup, &package, "Probe", "1.2.3"))?;

        assert_eq!(
            summary_text(&package, summary_property::TEMPLATE)?,
            "x64;1033"
        );
        assert_eq!(
            summary_text(&package, summary_property::SUBJECT)?,
            "Probe 1.2.3"
        );
        // The revision number is the package code, which changes when the image
        // does; a package Windows cannot read the summary of is not one it will
        // open at all.
        assert!(summary_text(&package, summary_property::REVISION)?.starts_with('{'));
        assert!(summary.size > 0);
        Ok(())
    }

    #[test]
    fn codes_stay_the_same_across_a_rebuild_and_the_upgrade_code_outlives_a_version() -> Result<()>
    {
        let directory = scratch("codes");
        let setup = setup_image(&directory);
        let first = write_wrapper(&wrapper(
            &setup,
            &directory.join("One.msi"),
            "Probe",
            "1.0.0",
        ))?;
        let rebuilt = write_wrapper(&wrapper(
            &setup,
            &directory.join("Two.msi"),
            "Probe",
            "1.0.0",
        ))?;
        let next = write_wrapper(&wrapper(
            &setup,
            &directory.join("Three.msi"),
            "Probe",
            "1.0.1",
        ))?;
        // A re-release of one day keeps the three fields Windows Installer
        // compares and is a product of its own, which is what lets it replace
        // the release it re-releases instead of being refused by the Installer.
        let re_release = write_wrapper(&wrapper(
            &setup,
            &directory.join("Four.msi"),
            "Probe",
            "1.0.0-r2",
        ))?;

        // A machine that installed one package has to be able to upgrade or
        // remove what a rebuild wrote, so the codes cannot be random.
        assert_eq!(first.product_code, rebuilt.product_code);
        assert_eq!(first.upgrade_code, rebuilt.upgrade_code);
        // A new version is a new product that upgrades the old one.
        assert_ne!(first.product_code, next.product_code);
        assert_eq!(first.upgrade_code, next.upgrade_code);
        assert_ne!(first.product_code, re_release.product_code);
        assert_eq!(first.upgrade_code, re_release.upgrade_code);
        Ok(())
    }

    #[test]
    fn a_version_the_installer_cannot_compare_is_refused() -> Result<()> {
        let directory = scratch("version");
        let setup = setup_image(&directory);
        let package = directory.join("Probe.msi");
        for version in ["", "one.two", "1.2.beta"] {
            let error = write_wrapper(&wrapper(&setup, &package, "Probe", version))
                .expect_err("a version the installer cannot read is refused");
            assert!(
                format!("{error:#}").contains("version"),
                "{version} should be reported as a version: {error:#}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_product_name_that_cannot_name_a_directory_is_refused() -> Result<()> {
        let directory = scratch("names");
        let setup = setup_image(&directory);
        let package = directory.join("Probe.msi");
        for name in ["", "  ", "Probe/Setup", "Probe:Setup"] {
            let error = write_wrapper(&wrapper(&setup, &package, name, "1.0.0"))
                .expect_err("a name the installer cannot use as a directory is refused");
            assert!(
                format!("{error:#}").contains("directory"),
                "{name:?} should be reported as a directory name: {error:#}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_name_too_long_for_a_short_name_carries_one_the_file_system_can_keep() -> Result<()> {
        // A name the file system can already express is written as it is.
        assert_eq!(directory_name("Widget")?, "Widget");
        // A longer or non-Latin name gets a short name made of the characters a
        // file system keeps, and the name the user knows follows the bar.
        for name in ["Widget Factory", "易玩安装器"] {
            let written = directory_name(name)?;
            let (short, long) = written.split_once('|').expect("a short and a long name");
            assert_eq!(long, name);
            assert!(!short.is_empty() && short.len() <= 12);
            assert!(short
                .chars()
                .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit()));
        }
        // A name Windows keeps for a device is not one a directory can take.
        assert!(directory_name("CON")?.contains('|'));
        // The short name is the same on every build of the same product.
        assert_eq!(
            directory_name("Widget Factory")?,
            directory_name("Widget Factory")?
        );
        Ok(())
    }
}
