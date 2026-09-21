//! What the builder reports about a project before it builds one.
//!
//! The warning list is the only place a project's own gaps surface cheaply. The
//! runtime falls back to the default locale and to the 1x artwork, so a missing
//! translation or a missing density pair is invisible until the installed
//! product happens to run on the machine that would have shown it.

use std::path::PathBuf;

use nano_installer_core::{inspect_project, PayloadFormat};

/// A project with one page, one language and a payload, so a case changes only
/// the thing it is about.
struct Project {
    /// Held only so the project directory is removed when the case ends.
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Project {
    fn new() -> anyhow::Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        for directory in ["layouts", "assets", "locales", "payload"] {
            std::fs::create_dir_all(root.join(directory))?;
        }
        // One key the default locale answers, one it does too, and one no
        // locale answers at all: a page is allowed to ask for text the product
        // does not ship, and the build must not nag about it.
        std::fs::write(
            root.join("layouts/configpage.xml"),
            r#"<Page width="720" height="450">
                 <Label text="@install_button" />
                 <Label text="@version_info" />
                 <Label text="@product_specific" />
               </Page>"#,
        )?;
        std::fs::write(
            root.join("locales/en-US.json"),
            r#"{"install_button": "Install", "version_info": "Version 1.2.3"}"#,
        )?;
        // The inspector reads the signature and the size, never the contents.
        std::fs::write(
            root.join("payload/app.zip"),
            b"PK\x03\x04 a payload the inspector does not open",
        )?;
        std::fs::write(
            root.join("installer_config.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "project": {"name": "Inspection", "version": "1.2.3", "publisher": "Test"},
                "output": {
                    "installer_name": "Inspection_Setup.exe",
                    "uninstaller_name": "uninst.exe"
                },
                "install": {
                    "exe_name": "Inspection.exe",
                    "default_path": "C:\\Program Files\\Inspection"
                },
                "localization": {"default_locale": "en-US", "supported_locales": ["en-US"]},
                "resources": {
                    "layouts_dir": "layouts",
                    "assets_dir": "assets",
                    "locales_dir": "locales",
                    "payload_file": "payload/app.zip"
                },
                "wizard": {
                    "pages": [{"id": "config", "layout": "layouts/configpage.xml", "title": "Options"}]
                }
            }))?,
        )?;
        Ok(Self { _temp: temp, root })
    }

    fn edit_config(&self, edit: impl FnOnce(&mut serde_json::Value)) -> anyhow::Result<()> {
        let path = self.root.join("installer_config.json");
        let mut config: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        edit(&mut config);
        std::fs::write(&path, serde_json::to_vec_pretty(&config)?)?;
        Ok(())
    }

    fn write_locale(&self, locale: &str, text: &str) -> anyhow::Result<()> {
        std::fs::write(self.root.join(format!("locales/{locale}.json")), text)?;
        Ok(())
    }

    /// Writes an image file. Only its name matters: the inspector never opens
    /// the pixels to decide whether a density pair is complete.
    fn touch_asset(&self, name: &str) -> anyhow::Result<()> {
        std::fs::write(self.root.join("assets").join(name), b"")?;
        Ok(())
    }

    fn warnings(&self) -> anyhow::Result<Vec<String>> {
        Ok(inspect_project(&self.root)?.warnings)
    }
}

/// A page key the default locale answers is reported for every language that
/// leaves it out, and nothing else is.
#[test]
fn a_translation_missing_page_text_is_reported() -> anyhow::Result<()> {
    let project = Project::new()?;
    project.write_locale("ru", r#"{"install_button": "Установить"}"#)?;
    project.write_locale(
        "ja",
        r#"{"install_button": "インストール", "version_info": "バージョン 1.2.3"}"#,
    )?;
    project.edit_config(|config| {
        config["localization"]["supported_locales"] = serde_json::json!(["en-US", "ru", "ja"]);
    })?;

    // The complete translation and the key no locale answers are both silent,
    // so one line here is the whole report.
    assert_eq!(
        project.warnings()?,
        vec!["locales/ru.json is missing 1 page text(s): `version_info`".to_string()],
        "only the partially translated language has a gap"
    );
    Ok(())
}

/// A language the project says it ships but has no file for is reported, since
/// nothing else would: the runtime falls back to the default locale and the
/// product simply appears in the wrong language.
#[test]
fn a_supported_locale_without_a_file_is_reported() -> anyhow::Result<()> {
    let project = Project::new()?;
    project.edit_config(|config| {
        config["localization"]["supported_locales"] = serde_json::json!(["en-US", "ja"]);
    })?;

    assert_eq!(
        project.warnings()?,
        vec!["localization.supported_locales lists ja, but locales/ja.json is missing".to_string()]
    );
    Ok(())
}

/// Artwork is chosen by density while the page draws, so a file without its
/// counterpart is only ever missed on a display at the other scaling factor.
/// The build reports both directions: a 1x with no 2x, and a 2x with no 1x.
#[test]
fn an_asset_without_its_density_pair_is_reported() -> anyhow::Result<()> {
    let project = Project::new()?;
    project.touch_asset("logo.png")?;
    project.touch_asset("logo@2x.png")?;
    project.touch_asset("extra@2x.png")?;

    assert_eq!(
        project.warnings()?,
        vec!["missing DPI pair for assets/extra@2x.png".to_string()],
        "a complete pair is silent, and the 2x without a 1x is not"
    );

    project.touch_asset("extra.png")?;
    assert!(
        project.warnings()?.is_empty(),
        "a project with every pair present still reports one"
    );
    Ok(())
}

/// The summary the window shows is what the project says, including the
/// defaults a project is allowed to leave out.
#[test]
fn the_summary_reports_what_the_project_declares() -> anyhow::Result<()> {
    let project = Project::new()?;
    let summary = inspect_project(&project.root)?;
    assert_eq!(summary.project_name, "Inspection");
    assert_eq!(summary.file_version, "1.2.3");
    assert_eq!(summary.payload_format, PayloadFormat::Zip);
    assert_eq!(summary.uninstaller_name, "uninst.exe");
    assert_eq!(summary.default_locale, "en-US");
    assert_eq!(
        summary.default_install_path.as_deref(),
        Some("C:\\Program Files\\Inspection")
    );
    assert!(!summary.require_admin, "the project never asked for rights");
    assert!(
        summary.dpi_aware,
        "scaling is on unless a project turns it off"
    );

    project.edit_config(|config| {
        config["project"]["file_version"] = serde_json::Value::from("2.0.0.0");
        config["install"]["require_admin"] = serde_json::Value::from(true);
        config["ui"] = serde_json::json!({"dpi_aware": false});
    })?;

    let summary = inspect_project(&project.root)?;
    assert!(
        summary.require_admin,
        "the switch did not reach the summary"
    );
    assert!(!summary.dpi_aware, "the switch did not reach the summary");
    assert_eq!(
        summary.file_version, "2.0.0.0",
        "an explicit file version must win over the release version"
    );
    Ok(())
}

/// The message a field shows while its value is not acceptable is a locale key
/// like any other page text, so a language that leaves it out is reported here
/// rather than discovered by the user whose hint came out in the wrong words.
#[test]
fn a_validation_message_the_page_asks_for_is_reported() -> anyhow::Result<()> {
    let project = Project::new()?;
    std::fs::write(
        project.root.join("layouts/configpage.xml"),
        r##"<Page width="720" height="450">
             <TextInput id="editDir" required="true" required-message="@dir_needed" />
             <Label text="@install_button" />
           </Page>"##,
    )?;
    project.write_locale(
        "en-US",
        r#"{"install_button": "Install", "dir_needed": "Choose a folder"}"#,
    )?;
    project.write_locale("ru", r#"{"install_button": "Установить"}"#)?;
    project.edit_config(|config| {
        config["localization"]["supported_locales"] = serde_json::json!(["en-US", "ru"]);
    })?;

    assert_eq!(
        project.warnings()?,
        vec!["locales/ru.json is missing 1 page text(s): `dir_needed`".to_string()],
        "the hint a field shows counts as page text"
    );
    Ok(())
}
