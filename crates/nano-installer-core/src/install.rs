use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND};
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegDeleteTreeW, RegDeleteValueW, RegOpenKeyExW,
    RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE,
    KEY_READ, KEY_SET_VALUE, KEY_WRITE, REG_CREATED_NEW_KEY, REG_CREATE_KEY_DISPOSITION, REG_DWORD,
    REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE,
};

use super::{script, shell, BundleIndex, RuntimeMode, UI};

/// Shortcut and autostart preferences gathered from the installer UI.
#[derive(Default, Clone)]
pub(super) struct InstallSelection {
    destination: Option<String>,
    checkboxes: std::collections::HashMap<String, bool>,
}

impl InstallSelection {
    fn checked(&self, id: &str, fallback: bool) -> bool {
        self.checkboxes.get(id).copied().unwrap_or(fallback)
    }

    /// The checkbox states the installer UI held when the user started the task.
    pub(super) fn checkboxes(&self) -> &std::collections::HashMap<String, bool> {
        &self.checkboxes
    }
}

pub(super) const MANIFEST_NAME: &str = "nano-installer-manifest.json";
static BUSY: AtomicBool = AtomicBool::new(false);
static STAGING_ID: AtomicU64 = AtomicU64::new(0);

pub(super) fn busy() -> bool {
    BUSY.load(Ordering::Acquire)
}

pub(super) struct StagingDirectory(pub(super) PathBuf);

impl StagingDirectory {
    pub(super) fn create() -> Result<Self> {
        for _ in 0..100 {
            let id = STAGING_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("nano-installer-stage-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error).context("failed to prepare install staging"),
            }
        }
        bail!("failed to allocate a unique staging directory")
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn start_install() {
    let selection = UI
        .get()
        .and_then(|runtime| runtime.lock().ok())
        .and_then(|state| {
            matches!(state.mode, RuntimeMode::Installer).then(|| InstallSelection {
                destination: state.interaction.text_input_values.get("editDir").cloned(),
                checkboxes: state.interaction.checkbox_states.clone(),
            })
        });
    let Some(selection) = selection else {
        show_result(Err(anyhow::anyhow!(
            "installation directory is not configured"
        )));
        return;
    };
    let Some(destination) = selection.destination.clone() else {
        show_result(Err(anyhow::anyhow!(
            "installation directory is not configured"
        )));
        return;
    };
    run_worker(move || {
        let setup = std::env::current_exe()?;
        install_setup(&setup, Path::new(&destination), &selection)
    });
}

pub(super) fn start_uninstall() {
    // Data is preserved unless the user explicitly clears the keep-data box;
    // an uninstall page without that checkbox therefore never destroys data.
    let keep_data = UI
        .get()
        .and_then(|runtime| runtime.lock().ok())
        .and_then(|state| {
            matches!(state.mode, RuntimeMode::Uninstaller)
                .then(|| {
                    state
                        .interaction
                        .checkbox_states
                        .get("chkReserveData")
                        .copied()
                })
                .flatten()
        })
        .unwrap_or(true);
    run_worker(move || {
        let uninstaller = std::env::current_exe()?;
        uninstall(&uninstaller, keep_data)
    });
}

fn run_worker(work: impl FnOnce() -> Result<()> + Send + 'static) {
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    let pages = super::page_count();
    // Page 1 is the task page, the last page is the completion page. A layout
    // list shorter than that leaves the wizard on its first page.
    if pages > 1 {
        let _ = super::show_page(1);
    }
    std::thread::spawn(move || {
        let result = work();
        BUSY.store(false, Ordering::Release);
        match &result {
            // The completion page reports the outcome and owns the next action.
            Ok(()) if pages > 1 => {
                let _ = super::show_page(pages - 1);
            }
            Ok(()) => show_result(Ok(())),
            Err(error) => {
                let _ = super::show_page(0);
                show_result(Err(anyhow::anyhow!("{error:#}")));
            }
        }
    });
}

/// Reports how the task ended.
///
/// A project with a completion page shows the outcome there. A project without
/// one still has to be told, and that notice is drawn inside the installer
/// window so it carries the product's skin rather than the system default.
fn show_result(result: Result<()>) {
    let text = match result {
        Ok(()) => "Operation complete".to_string(),
        Err(error) => format!("Operation failed:\n{error:#}"),
    };
    super::show_notice(&text);
}

fn install_setup(setup: &Path, destination: &Path, selection: &InstallSelection) -> Result<()> {
    validate_destination(destination)?;
    super::report_progress(5, "status.preparing")?;
    let bundle = BundleIndex::read(setup)?.context("installer resource bundle missing")?;
    let config = bundle.read_config()?;
    let prep = prepare_install(&bundle, &config, destination)?;

    let stage = StagingDirectory::create()?;
    let extracted = stage.0.join("files");
    let backups = stage.0.join("rollback");
    if bundle.contains(script::INSTALL_SCRIPT) {
        return script::run_install(script::InstallRequest {
            setup: setup.to_path_buf(),
            bundle,
            config,
            destination: destination.to_path_buf(),
            selection: selection.clone(),
            stage: stage.0.clone(),
            prep,
        });
    }
    super::report_progress(15, "status.extracting")?;
    let files = extract_payload(setup, &bundle, &config, &stage.0, &extracted)?;
    super::report_progress(50, "status.deploying")?;

    let exe_name = config["install"]["exe_name"]
        .as_str()
        .context("install.exe_name is required")?;
    // A running instance would keep the destination files locked. Both flags
    // mean "stop the product first"; they differ only in how strictly the
    // installer treats a failure to do so.
    if config["install"]["kill_process_on_install"].as_bool() == Some(true)
        || config["install"]["detect_running_process"].as_bool() == Some(true)
    {
        shell::kill_processes(exe_name)
            .with_context(|| format!("cannot close the running {exe_name}"))?;
    }
    let artifacts = InstallArtifacts::plan(
        &config,
        selection,
        destination,
        exe_name,
        prep.previous.as_ref().map(|install| &install.manifest),
    )?;
    Deployment {
        extracted: &extracted,
        destination,
        files: &files,
        uninstaller_name: &prep.uninstaller_name,
        uninstaller: &prep.uninstaller,
        root: prep.root,
        registry_path: &prep.registry_path,
        previous: prep.previous.as_ref(),
        backup_root: &backups,
        artifacts: &artifacts,
        registry_values: &[],
        registry_keys: &[],
    }
    .run(|| {
        register_uninstaller(
            prep.root,
            &prep.registry_path,
            destination,
            &prep.uninstaller_name,
            &config,
            prep.upgrade,
        )
    })?;
    super::report_progress(95, "status.finishing")?;
    super::record_installed_app(destination.join(exe_name))?;
    super::report_progress(100, "status.install_complete")?;
    Ok(())
}

/// A previous installation of this project found at the destination.
pub(super) struct PreviousInstall {
    pub(super) files: Vec<PathBuf>,
    /// Shortcut and autostart entries the previous version recorded.
    pub(super) manifest: Value,
}

/// Creates the installation directory and opens a rollback journal for it.
///
/// A fresh install gets a directory of its own, so a failed attempt can be
/// removed whole; an upgrade reuses the existing one and relies on the journal
/// alone. Either way the directory is validated first, so a relative or
/// drive-root path is rejected before anything is written.
pub(super) fn begin_deployment(
    destination: &Path,
    backup_root: &Path,
    previous: Option<&PreviousInstall>,
) -> Result<RollbackJournal> {
    validate_destination(destination)?;
    let fresh = if previous.is_some() {
        fs::create_dir_all(destination)?;
        None
    } else {
        fs::create_dir_all(
            destination
                .parent()
                .context("installation directory has no parent")?,
        )?;
        fs::create_dir(destination).with_context(|| {
            format!(
                "installation directory must be new: {}",
                destination.display()
            )
        })?;
        Some(destination.to_path_buf())
    };
    Ok(RollbackJournal::new(backup_root.to_path_buf(), fresh))
}

/// Everything a deployment needs that does not depend on the payload contents:
/// the uninstaller to embed, the registry target, and the installation this run
/// replaces, if any.
pub(super) struct InstallPrep {
    pub(super) uninstaller_name: String,
    pub(super) uninstaller: Vec<u8>,
    pub(super) root: HKEY,
    pub(super) registry_path: String,
    pub(super) previous: Option<PreviousInstall>,
    pub(super) upgrade: bool,
}

/// Validates the destination and the bundle before any file is written.
///
/// A previous installation of this project at the destination is an upgrade.
/// Anything else at the destination, or a foreign uninstall key, is refused so
/// an unrelated installation is never overwritten.
fn prepare_install(
    bundle: &BundleIndex,
    config: &Value,
    destination: &Path,
) -> Result<InstallPrep> {
    let uninstaller_name = config["output"]["uninstaller_name"]
        .as_str()
        .unwrap_or("uninst.exe")
        .to_string();
    super::validate_output_filename(&uninstaller_name, "output.uninstaller_name")?;
    let uninstaller = bundle.read_file(&format!("runtime/{uninstaller_name}"))?;
    let (root, registry_path) = uninstall_registry_key(config)?;
    let previous = previous_install(destination, config)?;
    let upgrade = previous.is_some();
    if !upgrade && registry_key_exists(root, &registry_path)? {
        bail!("uninstall registry key already exists; refusing to overwrite another installation");
    }
    if let Some(required_mb) = config["install"]["required_space_mb"].as_u64() {
        let drive = super::disk_root(destination).context("cannot determine installation drive")?;
        let free = super::query_disk_free_bytes(&drive)?;
        let required = required_mb.saturating_mul(1024 * 1024);
        if free < required {
            bail!(
                "not enough disk space: {required_mb} MiB required on {}",
                drive.display()
            );
        }
    }
    Ok(InstallPrep {
        uninstaller_name,
        uninstaller,
        root,
        registry_path,
        previous,
        upgrade,
    })
}

/// Streams the payload out of the setup image and expands it into `target`.
///
/// The payload is copied to disk first and expanded by the stub that matches
/// its format, so a large archive never has to fit in the process address
/// space. The expanded file list is validated before it is deployed.
pub(super) fn extract_payload(
    setup: &Path,
    bundle: &BundleIndex,
    config: &Value,
    scratch: &Path,
    target: &Path,
) -> Result<Vec<PathBuf>> {
    let payload_name = config["resources"]["payload_file"]
        .as_str()
        .context("resources.payload_file is required")?;
    if !bundle.contains(payload_name) {
        bail!("payload missing: {payload_name}");
    }
    let uninstaller_name = config["output"]["uninstaller_name"]
        .as_str()
        .unwrap_or("uninst.exe");
    let archive = scratch.join("payload.archive");
    bundle.copy_file_to(payload_name, &archive)?;
    let output = std::process::Command::new(setup)
        .arg("--extract")
        .arg(&archive)
        .arg(target)
        .output()
        .context("failed to start installer extraction backend")?;
    if !output.status.success() {
        bail!(
            "payload extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let files = collect_staged_files(target)?;
    if files.is_empty() {
        bail!("payload archive contains no files")
    }
    if files
        .iter()
        .any(|path| path == Path::new(uninstaller_name) || path == Path::new(MANIFEST_NAME))
    {
        bail!("payload conflicts with installer-managed files")
    }
    let exe_name = config["install"]["exe_name"]
        .as_str()
        .context("install.exe_name is required")?;
    super::validate_output_filename(exe_name, "install.exe_name")?;
    if !files.iter().any(|path| path == Path::new(exe_name)) {
        bail!("payload does not contain the configured application executable: {exe_name}")
    }
    Ok(files)
}

/// Inspects `destination` and reports an existing installation of this project.
///
/// Returns `Ok(None)` when the destination is free. A directory that exists
/// without a manifest, or whose manifest belongs to another project, is
/// reported as an error so that unrelated user data is never overwritten.
fn previous_install(destination: &Path, config: &Value) -> Result<Option<PreviousInstall>> {
    if !destination.exists() {
        return Ok(None);
    }
    if !destination.is_dir() {
        bail!(
            "installation path exists and is not a directory: {}",
            destination.display()
        );
    }
    let manifest_file = destination.join(MANIFEST_NAME);
    if !manifest_file.is_file() {
        bail!(
            "installation directory already exists and was not created by this installer: {}",
            destination.display()
        );
    }
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_file)?)
        .context("invalid installation manifest")?;
    if manifest["version"].as_u64() != Some(1) {
        bail!("unsupported installation manifest version")
    }
    let (root, expected_path) = uninstall_registry_key(config)?;
    let root_name = registry_root_name(root);
    if manifest["registry_root"].as_str() != Some(root_name)
        || manifest["registry_path"].as_str() != Some(expected_path.as_str())
    {
        bail!(
            "installation directory belongs to a different project: {}",
            destination.display()
        );
    }
    Ok(Some(PreviousInstall {
        files: manifest_file_paths(&manifest)?,
        manifest,
    }))
}

pub(super) fn registry_root_name(root: windows::Win32::System::Registry::HKEY) -> &'static str {
    if root == HKEY_CURRENT_USER {
        "HKCU"
    } else {
        "HKLM"
    }
}

/// Validates the manifest file list and returns the stored relative paths.
pub(super) fn manifest_file_paths(manifest: &Value) -> Result<Vec<PathBuf>> {
    let files = manifest["files"]
        .as_array()
        .context("manifest file list missing")?;
    let mut paths = Vec::with_capacity(files.len());
    for item in files {
        let relative = item
            .as_str()
            .context("manifest file path is not a string")?;
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            bail!("unsafe manifest file path: {relative}")
        }
        paths.push(path.to_path_buf());
    }
    Ok(paths)
}

/// One deployment of an installed file set, replayed from `extracted` into
/// `destination`.
struct Deployment<'a> {
    extracted: &'a Path,
    destination: &'a Path,
    files: &'a [PathBuf],
    uninstaller_name: &'a str,
    uninstaller: &'a [u8],
    root: windows::Win32::System::Registry::HKEY,
    registry_path: &'a str,
    previous: Option<&'a PreviousInstall>,
    backup_root: &'a Path,
    artifacts: &'a InstallArtifacts,
    /// Registry entries a project script wrote, replayed by the uninstaller.
    registry_values: &'a [(String, String)],
    registry_keys: &'a [String],
}

impl Deployment<'_> {
    /// Applies the deployment. On failure the destination is restored to its
    /// previous state: a fresh install is removed, an upgrade is rolled back
    /// from the journal.
    fn run(self, register: impl FnOnce() -> Result<()>) -> Result<()> {
        let mut journal = begin_deployment(self.destination, self.backup_root, self.previous)?;
        let result = self.deploy(&mut journal, register);
        if result.is_err() {
            journal.rollback();
            self.artifacts.undo();
        }
        result
    }

    fn deploy(
        &self,
        journal: &mut RollbackJournal,
        register: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        deploy_files(
            self.extracted,
            self.destination,
            self.files,
            self.previous,
            journal,
        )?;
        let uninstaller = self.destination.join(self.uninstaller_name);
        journal.track(&uninstaller)?;
        fs::write(&uninstaller, self.uninstaller)?;

        self.artifacts.apply(journal)?;

        write_manifest(
            self.destination,
            journal,
            registry_root_name(self.root),
            self.registry_path,
            self.files,
            &ManifestArtifacts {
                shortcuts: self.artifacts.shortcut_paths(),
                shortcut_dirs: self.artifacts.shortcut_dir_paths(),
                autostart: self.artifacts.autostart_json(),
                registry_values: self
                    .registry_values
                    .iter()
                    .map(|(path, name)| (path.clone(), name.clone()))
                    .collect(),
                registry_keys: self.registry_keys.to_vec(),
            },
        )?;
        register()
    }
}

/// The entries beside the deployed files that an uninstall has to replay.
pub(super) struct ManifestArtifacts {
    pub(super) shortcuts: Vec<String>,
    pub(super) shortcut_dirs: Vec<String>,
    pub(super) autostart: Value,
    /// Registry values a project script wrote, as `(key, value name)`.
    pub(super) registry_values: Vec<(String, String)>,
    /// Registry keys a project script created, including their subkeys.
    pub(super) registry_keys: Vec<String>,
}

/// Writes the manifest the uninstaller replays.
///
/// `files` holds paths relative to `destination`; the uninstaller joins them
/// back onto its own directory, so an installation stays relocatable.
pub(super) fn write_manifest(
    destination: &Path,
    journal: &mut RollbackJournal,
    registry_root: &str,
    registry_path: &str,
    files: &[PathBuf],
    artifacts: &ManifestArtifacts,
) -> Result<()> {
    let manifest = json!({
        "version": 1,
        "registry_root": registry_root,
        "registry_path": registry_path,
        "files": files.iter().map(|path| path.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "shortcuts": artifacts.shortcuts,
        "shortcut_dirs": artifacts.shortcut_dirs,
        "autostart": artifacts.autostart,
        "registry_values": artifacts
            .registry_values
            .iter()
            .map(|(path, name)| json!({"path": path, "name": name}))
            .collect::<Vec<_>>(),
        "registry_keys": artifacts.registry_keys,
    });
    let target = destination.join(MANIFEST_NAME);
    journal.track(&target)?;
    fs::write(target, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}

/// Copies the expanded payload into `destination`, replacing the same relative
/// paths and dropping files the previous version shipped but this one does not.
///
/// Every target is journaled first, so a failure part-way through restores what
/// was there before. Files this installer never wrote are left alone.
pub(super) fn deploy_files(
    extracted: &Path,
    destination: &Path,
    files: &[PathBuf],
    previous: Option<&PreviousInstall>,
    journal: &mut RollbackJournal,
) -> Result<()> {
    for relative in files {
        let source = extracted.join(relative);
        let target = destination.join(relative);
        fs::create_dir_all(target.parent().context("payload target has no parent")?)?;
        journal.track(&target)?;
        fs::copy(source, target)?;
    }
    if let Some(previous) = previous {
        for relative in &previous.files {
            if files.iter().any(|path| path == relative) {
                continue;
            }
            let target = destination.join(relative);
            if !target.is_file() {
                continue;
            }
            journal.track(&target)?;
            fs::remove_file(&target)?;
        }
    }
    Ok(())
}

/// Non-payload artifacts an install creates: shortcuts and an autostart entry.
///
/// They live outside the installation directory. Shortcut files go through the
/// rollback journal like payload files, so they are restored on undo; the
/// autostart value is overwritten in place, so the value it replaces is
/// captured up front and written back when the deployment fails.
#[derive(Default)]
struct InstallArtifacts {
    /// The installed application executable the artifacts point at.
    target: PathBuf,
    shortcuts: Vec<ShortcutPlan>,
    /// Directories this run created for shortcuts. Only removed while empty, so
    /// a shared Start Menu folder that holds other products survives.
    shortcut_dirs: Vec<PathBuf>,
    /// Shortcuts an earlier version created and this run no longer wants.
    stale_shortcuts: Vec<PathBuf>,
    autostart_changes: Vec<AutostartChange>,
}

#[derive(Clone)]
struct AutostartEntry {
    root: HKEY,
    path: String,
    value_name: String,
}

/// A change to one autostart value, remembering what was there before.
struct AutostartChange {
    entry: AutostartEntry,
    action: AutostartAction,
    /// The command the value held before this run, when it existed.
    previous_command: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AutostartAction {
    Write,
    Delete,
}

impl InstallArtifacts {
    /// Resolves the project's shortcut and autostart configuration together
    /// with what the installer UI selected, discarding anything the previous
    /// version recorded that this run no longer wants.
    fn plan(
        config: &Value,
        selection: &InstallSelection,
        destination: &Path,
        exe_name: &str,
        previous: Option<&Value>,
    ) -> Result<Self> {
        let project_name = config["project"]["name"].as_str().unwrap_or("Application");
        let target = destination.join(exe_name);
        let mut artifacts = Self {
            target: target.clone(),
            ..Self::default()
        };
        let shortcuts = &config["shortcuts"];
        if shortcuts["desktop_shortcut"].as_bool() == Some(true)
            && selection.checked(
                "chkShotcut",
                shortcuts["desktop_default"].as_bool().unwrap_or(true),
            )
        {
            artifacts.shortcuts.push(ShortcutPlan::new(
                shell::desktop_directory()?.join(format!("{project_name}.lnk")),
                target.clone(),
                destination.to_path_buf(),
            ));
        }
        if shortcuts["start_menu"].as_bool() == Some(true) {
            let folder = shortcuts["start_menu_folder"]
                .as_str()
                .unwrap_or(project_name);
            let programs = shell::programs_directory()?.join(folder);
            artifacts.shortcut_dirs.push(programs.clone());
            artifacts.shortcuts.push(ShortcutPlan::new(
                programs.join(format!("{project_name}.lnk")),
                target.clone(),
                destination.to_path_buf(),
            ));
            artifacts.shortcuts.push(ShortcutPlan::new(
                programs.join(format!("Uninstall {project_name}.lnk")),
                destination.join(
                    config["output"]["uninstaller_name"]
                        .as_str()
                        .unwrap_or("uninst.exe"),
                ),
                destination.to_path_buf(),
            ));
        }
        let planned_autostart = if config["autostart"]["enabled"].as_bool() == Some(true)
            && selection.checked(
                "chkAutoRun",
                config["autostart"]["default"].as_bool().unwrap_or(false),
            ) {
            let configured = config["autostart"]["registry_key"]
                .as_str()
                .unwrap_or("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run");
            let (root, path) = registry_path(configured)?;
            Some(AutostartEntry {
                root,
                path,
                value_name: config["autostart"]["registry_value_name"]
                    .as_str()
                    .unwrap_or(project_name)
                    .to_string(),
            })
        } else {
            None
        };

        // A shortcut the previous version created is removed unless this run
        // creates the same file again; otherwise disabling it in the UI would
        // leave a dead link behind forever.
        if let Some(recorded) = previous {
            for stale in recorded_shortcuts(recorded) {
                if !artifacts.shortcuts.iter().any(|plan| plan.link == stale) {
                    artifacts.stale_shortcuts.push(stale);
                }
            }
        }

        // Whatever the previous manifest recorded is dropped unless this run
        // writes the same value.
        if let Some(recorded) = previous.and_then(recorded_autostart) {
            let replaced = planned_autostart.as_ref().is_some_and(|planned| {
                planned.path == recorded.path && planned.value_name == recorded.value_name
            });
            if !replaced {
                artifacts.push_autostart_change(recorded, AutostartAction::Delete)?;
            }
        }
        if let Some(entry) = planned_autostart {
            artifacts.push_autostart_change(entry, AutostartAction::Write)?;
        }
        Ok(artifacts)
    }

    fn push_autostart_change(
        &mut self,
        entry: AutostartEntry,
        action: AutostartAction,
    ) -> Result<()> {
        let previous_command = read_registry_string(entry.root, &entry.path, &entry.value_name)?;
        self.autostart_changes.push(AutostartChange {
            entry,
            action,
            previous_command,
        });
        Ok(())
    }

    /// Applies every change; a failure here makes the caller roll back.
    fn apply(&self, journal: &mut RollbackJournal) -> Result<()> {
        for change in &self.autostart_changes {
            match change.action {
                AutostartAction::Delete => {
                    delete_registry_value(
                        change.entry.root,
                        &change.entry.path,
                        &change.entry.value_name,
                    )?;
                }
                AutostartAction::Write => {
                    write_registry_string(
                        change.entry.root,
                        &change.entry.path,
                        &change.entry.value_name,
                        &format!("\"{}\"", self.target.display()),
                    )?;
                }
            }
        }
        for stale in &self.stale_shortcuts {
            if stale.is_file() {
                journal.track(stale)?;
                fs::remove_file(stale)?;
            }
        }
        for shortcut in &self.shortcuts {
            journal.track(&shortcut.link)?;
            shell::create_shortcut(&shortcut.link, &shortcut.target, &shortcut.working_dir)?;
        }
        shell::notify_shell();
        Ok(())
    }

    fn shortcut_paths(&self) -> Vec<String> {
        self.shortcuts
            .iter()
            .map(|shortcut| shortcut.link.to_string_lossy().to_string())
            .collect()
    }

    fn shortcut_dir_paths(&self) -> Vec<String> {
        self.shortcut_dirs
            .iter()
            .map(|dir| dir.to_string_lossy().to_string())
            .collect()
    }

    /// The autostart value to record in the manifest, if this run installed one.
    fn autostart_json(&self) -> Value {
        match self
            .autostart_changes
            .iter()
            .find(|change| change.action == AutostartAction::Write)
        {
            Some(change) => json!({
                "registry_root": registry_root_name(change.entry.root),
                "registry_path": change.entry.path,
                "value_name": change.entry.value_name,
            }),
            None => Value::Null,
        }
    }

    /// Puts every autostart value back the way this run found it.
    ///
    /// Shortcut files are already restored by the rollback journal.
    fn undo(&self) {
        for change in self.autostart_changes.iter().rev() {
            let entry = &change.entry;
            let _ = match &change.previous_command {
                Some(command) => {
                    write_registry_string(entry.root, &entry.path, &entry.value_name, command)
                }
                None => delete_registry_value(entry.root, &entry.path, &entry.value_name),
            };
        }
    }

    /// Removes the artifacts an earlier install recorded in the manifest.
    ///
    /// Only the directories this installer created are candidates, and only
    /// while they are empty: deleting ancestors instead would walk up into the
    /// shared Start Menu tree.
    fn remove_recorded(manifest: &Value) {
        let created = recorded_shortcut_dirs(manifest);
        for shortcut in recorded_shortcuts(manifest) {
            let _ = fs::remove_file(&shortcut);
        }
        for dir in created {
            let _ = fs::remove_dir(dir);
        }
        if let Some(recorded) = recorded_autostart(manifest) {
            let _ = delete_registry_value(recorded.root, &recorded.path, &recorded.value_name);
        }
    }
}

/// The shortcut files an earlier install recorded in its manifest.
pub(super) fn recorded_shortcuts(manifest: &Value) -> Vec<PathBuf> {
    manifest["shortcuts"]
        .as_array()
        .map(|shortcuts| {
            shortcuts
                .iter()
                .filter_map(Value::as_str)
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

/// The directories an earlier install created for its shortcuts.
///
/// Manifests written before this field existed report none, so an upgrade over
/// such an install removes the links but leaves the folder behind.
fn recorded_shortcut_dirs(manifest: &Value) -> Vec<PathBuf> {
    manifest["shortcut_dirs"]
        .as_array()
        .map(|dirs| {
            dirs.iter()
                .filter_map(Value::as_str)
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Reads the autostart entry an install recorded in its manifest.
fn recorded_autostart(manifest: &Value) -> Option<AutostartEntry> {
    let autostart = &manifest["autostart"];
    Some(AutostartEntry {
        root: if autostart["registry_root"].as_str()? == "HKLM" {
            HKEY_LOCAL_MACHINE
        } else {
            HKEY_CURRENT_USER
        },
        path: autostart["registry_path"].as_str()?.to_string(),
        value_name: autostart["value_name"].as_str()?.to_string(),
    })
}

/// One `.lnk` an install should create, with everything `IShellLinkW` needs.
struct ShortcutPlan {
    link: PathBuf,
    target: PathBuf,
    working_dir: PathBuf,
}

impl ShortcutPlan {
    fn new(link: PathBuf, target: PathBuf, working_dir: PathBuf) -> Self {
        Self {
            link,
            target,
            working_dir,
        }
    }
}

/// Records every file a deployment is about to change so it can be undone.
pub(super) struct RollbackJournal {
    backup_root: PathBuf,
    counter: usize,
    entries: Vec<JournalEntry>,
    /// Set for a fresh install: the directory did not exist before this run,
    /// so it holds nothing but files from this attempt.
    fresh_destination: Option<PathBuf>,
}

struct JournalEntry {
    target: PathBuf,
    /// `Some` when the file existed before and was copied here.
    backup: Option<PathBuf>,
}

impl RollbackJournal {
    pub(super) fn new(backup_root: PathBuf, fresh_destination: Option<PathBuf>) -> Self {
        Self {
            backup_root,
            counter: 0,
            entries: Vec::new(),
            fresh_destination,
        }
    }

    /// Records `target` before it is created, overwritten, or removed.
    pub(super) fn track(&mut self, target: &Path) -> Result<()> {
        let backup = if target.is_file() {
            fs::create_dir_all(&self.backup_root)?;
            let backup = self.backup_root.join(format!("entry-{}", self.counter));
            self.counter += 1;
            fs::copy(target, &backup).with_context(|| {
                format!("failed to back up {} before installing", target.display())
            })?;
            Some(backup)
        } else {
            None
        };
        self.entries.push(JournalEntry {
            target: target.to_path_buf(),
            backup,
        });
        Ok(())
    }

    /// Undoes the tracked changes in reverse order. Best effort: a partially
    /// restored directory is still better than a half-written install.
    pub(super) fn rollback(&self) {
        if let Some(destination) = &self.fresh_destination {
            let _ = fs::remove_dir_all(destination);
        }
        // Entries are replayed even for a fresh install: a shortcut may have
        // existed in the user's profile before this run created it.
        for entry in self.entries.iter().rev() {
            match &entry.backup {
                Some(backup) => {
                    let _ = fs::copy(backup, &entry.target);
                }
                None => {
                    let _ = fs::remove_file(&entry.target);
                }
            }
        }
    }
}

pub(super) fn validate_destination(destination: &Path) -> Result<()> {
    if !destination.is_absolute() || destination.parent().is_none() {
        bail!("install path must be an absolute directory below a drive root")
    }
    if destination
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        bail!("install path cannot contain parent traversal")
    }
    if destination
        .parent()
        .is_some_and(|parent| parent.parent().is_none())
    {
        bail!("install path cannot be a drive root")
    }
    Ok(())
}

pub(super) fn collect_staged_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn visit(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            if entry.file_type()?.is_symlink() {
                bail!("archive contains a symbolic link")
            }
            if entry.file_type()?.is_dir() {
                visit(root, &entry.path(), files)?;
            } else if entry.file_type()?.is_file() {
                files.push(entry.path().strip_prefix(root)?.to_path_buf());
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

/// Directories that belong to the user rather than to the payload: the
/// per-user data roots the project configures under `uninstall.data_paths`.
///
/// Only paths that expand to a real location below a known user profile are
/// returned, so a broad pattern can never delete an unrelated directory.
pub(super) fn preserved_data_paths(config: &Value) -> Result<Vec<PathBuf>> {
    let configured = config["uninstall"]["data_paths"]
        .as_array()
        .map(|paths| {
            paths
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut resolved = Vec::new();
    for raw in configured {
        let expanded = shell::expand_environment(&raw)?;
        let path = PathBuf::from(&expanded);
        if path.is_absolute() && shell::is_user_data_path(&path) {
            resolved.push(path);
        }
    }
    Ok(resolved)
}

/// Reads a single `REG_SZ` value, returning `None` when it is absent.
pub(super) fn read_registry_string(root: HKEY, path: &str, name: &str) -> Result<Option<String>> {
    let mut key = Default::default();
    let opened = unsafe {
        RegOpenKeyExW(
            root,
            PCWSTR(wide(path).as_ptr()),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        )
    };
    if opened == ERROR_FILE_NOT_FOUND || opened == ERROR_PATH_NOT_FOUND {
        return Ok(None);
    }
    opened.ok()?;
    let mut kind = REG_VALUE_TYPE::default();
    let mut size = 0u32;
    let status = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(wide(name).as_ptr()),
            None,
            Some(&mut kind),
            None,
            Some(&mut size),
        )
    };
    if status == ERROR_FILE_NOT_FOUND || status == ERROR_PATH_NOT_FOUND {
        unsafe {
            let _ = RegCloseKey(key);
        }
        return Ok(None);
    }
    if status.is_err() || kind != REG_SZ || size < 2 {
        unsafe {
            let _ = RegCloseKey(key);
        }
        status.ok()?;
        return Ok(None);
    }
    let mut buffer = vec![0u8; size as usize];
    let status = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(wide(name).as_ptr()),
            None,
            Some(&mut kind),
            Some(buffer.as_mut_ptr()),
            Some(&mut size),
        )
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    status.ok()?;
    buffer.truncate(size as usize);
    let units = buffer
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|unit| *unit != 0)
        .collect::<Vec<_>>();
    Ok(Some(String::from_utf16(&units)?))
}

/// Writes a single `REG_SZ` value, creating the key when needed.
pub(super) fn write_registry_string(root: HKEY, path: &str, name: &str, value: &str) -> Result<()> {
    let mut key = Default::default();
    unsafe {
        RegCreateKeyExW(
            root,
            PCWSTR(wide(path).as_ptr()),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()?;
    }
    let encoded = wide(value);
    let bytes =
        unsafe { std::slice::from_raw_parts(encoded.as_ptr().cast::<u8>(), encoded.len() * 2) };
    let status =
        unsafe { RegSetValueExW(key, PCWSTR(wide(name).as_ptr()), 0, REG_SZ, Some(bytes)) };
    unsafe {
        let _ = RegCloseKey(key);
    }
    status.ok()?;
    Ok(())
}

/// Writes a single `REG_DWORD` value, creating the key when needed.
pub(super) fn write_registry_dword(root: HKEY, path: &str, name: &str, value: u32) -> Result<()> {
    let mut key = Default::default();
    unsafe {
        RegCreateKeyExW(
            root,
            PCWSTR(wide(path).as_ptr()),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()?;
    }
    let bytes = value.to_le_bytes();
    let status =
        unsafe { RegSetValueExW(key, PCWSTR(wide(name).as_ptr()), 0, REG_DWORD, Some(&bytes)) };
    unsafe {
        let _ = RegCloseKey(key);
    }
    status.ok()?;
    Ok(())
}

/// Deletes a whole key and its subkeys; a missing key is not an error.
pub(super) fn delete_registry_key(root: HKEY, path: &str) -> Result<()> {
    let status = unsafe { RegDeleteTreeW(root, PCWSTR(wide(path).as_ptr())) };
    if status != ERROR_FILE_NOT_FOUND && status != ERROR_PATH_NOT_FOUND {
        status.ok()?;
    }
    Ok(())
}

/// Removes the registry entries a project script recorded in the manifest.
///
/// Values are deleted before keys: a key recorded for a product also holds the
/// values recorded for it, so removing the key first would make the value
/// deletion a no-op and leave nothing behind either way.
fn remove_recorded_registry(manifest: &Value) {
    if let Some(values) = manifest["registry_values"].as_array() {
        for value in values {
            let (Some(path), Some(name)) = (value["path"].as_str(), value["name"].as_str()) else {
                continue;
            };
            if let Ok((root, path)) = registry_path(path) {
                let _ = delete_registry_value(root, &path, name);
            }
        }
    }
    if let Some(keys) = manifest["registry_keys"].as_array() {
        for key in keys.iter().filter_map(Value::as_str) {
            if let Ok((root, path)) = registry_path(key) {
                let _ = delete_registry_key(root, &path);
            }
        }
    }
}

/// Deletes a single value; a missing key or value is not an error.
pub(super) fn delete_registry_value(root: HKEY, path: &str, name: &str) -> Result<()> {
    let mut key = Default::default();
    let opened = unsafe {
        RegOpenKeyExW(
            root,
            PCWSTR(wide(path).as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
    };
    if opened == ERROR_FILE_NOT_FOUND || opened == ERROR_PATH_NOT_FOUND {
        return Ok(());
    }
    opened.ok()?;
    let status = unsafe { RegDeleteValueW(key, PCWSTR(wide(name).as_ptr())) };
    unsafe {
        let _ = RegCloseKey(key);
    }
    if status != ERROR_FILE_NOT_FOUND && status != ERROR_PATH_NOT_FOUND {
        status.ok()?;
    }
    Ok(())
}

pub(super) fn uninstall_registry_key(
    config: &Value,
) -> Result<(windows::Win32::System::Registry::HKEY, String)> {
    let configured = config["registry"]["uninstall_key"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}",
                config["project"]["name"]
                    .as_str()
                    .unwrap_or("nano-installer")
            )
        });
    registry_path(&configured)
}

pub(super) fn registry_path(raw: &str) -> Result<(windows::Win32::System::Registry::HKEY, String)> {
    let mut parts = raw.split('\\').filter(|part| !part.is_empty());
    let root = match parts
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase()
        .as_str()
    {
        "HKCU" | "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
        "HKLM" | "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
        _ => bail!("uninstall registry key must use HKCU or HKLM"),
    };
    let path = parts.collect::<Vec<_>>().join("\\");
    if path.is_empty() || path.contains("..") {
        bail!("invalid uninstall registry key")
    }
    Ok((root, path))
}

pub(super) fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(super) fn registry_key_exists(
    root: windows::Win32::System::Registry::HKEY,
    path: &str,
) -> Result<bool> {
    let mut key = Default::default();
    let result = unsafe { RegOpenKeyExW(root, PCWSTR(wide(path).as_ptr()), 0, KEY_READ, &mut key) };
    if result == ERROR_FILE_NOT_FOUND || result == ERROR_PATH_NOT_FOUND {
        return Ok(false);
    }
    result.ok()?;
    unsafe {
        RegCloseKey(key).ok()?;
    }
    Ok(true)
}

pub(super) fn register_uninstaller(
    root: windows::Win32::System::Registry::HKEY,
    path: &str,
    destination: &Path,
    uninstaller_name: &str,
    config: &Value,
    upgrade: bool,
) -> Result<()> {
    write_uninstall_registration(root, path, destination, uninstaller_name, config, upgrade)
}

/// Writes the uninstall registration over a key the project owns already.
///
/// A project script may write the same key itself to add its own fields, so the
/// script path reaches this instead of `register_uninstaller`. The key can only
/// be this project's: `prepare_install` refused a foreign one before the script
/// ran, and an upgrade is allowed to replace its own.
pub(super) fn rewrite_uninstall_registration(
    root: windows::Win32::System::Registry::HKEY,
    path: &str,
    destination: &Path,
    uninstaller_name: &str,
    config: &Value,
) -> Result<()> {
    write_uninstall_registration(root, path, destination, uninstaller_name, config, true)
}

fn write_uninstall_registration(
    root: windows::Win32::System::Registry::HKEY,
    path: &str,
    destination: &Path,
    uninstaller_name: &str,
    config: &Value,
    upgrade: bool,
) -> Result<()> {
    let mut key = Default::default();
    let mut disposition = REG_CREATE_KEY_DISPOSITION::default();
    unsafe {
        RegCreateKeyExW(
            root,
            PCWSTR(wide(path).as_ptr()),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            Some(&mut disposition),
        )
        .ok()?;
    }
    if disposition != REG_CREATED_NEW_KEY && !upgrade {
        unsafe {
            let _ = RegCloseKey(key);
        }
        bail!("uninstall registry key already exists; refusing to overwrite")
    }
    let uninstaller = destination.join(uninstaller_name);
    let fields = [
        (
            "DisplayName",
            config["project"]["name"]
                .as_str()
                .unwrap_or("nano-installer")
                .to_string(),
        ),
        (
            "DisplayVersion",
            config["project"]["version"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        ),
        (
            "Publisher",
            config["project"]["publisher"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        ),
        ("InstallLocation", destination.display().to_string()),
        ("UninstallString", format!("\"{}\"", uninstaller.display())),
        ("DisplayIcon", uninstaller.display().to_string()),
    ];
    let result = fields
        .into_iter()
        .try_for_each(|(name, value)| -> Result<()> {
            let encoded = wide(&value);
            let bytes = unsafe {
                std::slice::from_raw_parts(encoded.as_ptr().cast::<u8>(), encoded.len() * 2)
            };
            unsafe {
                RegSetValueExW(key, PCWSTR(wide(name).as_ptr()), 0, REG_SZ, Some(bytes)).ok()?;
            }
            Ok(())
        });
    unsafe {
        let _ = RegCloseKey(key);
    }
    if result.is_err() {
        unsafe {
            let _ = RegDeleteKeyW(root, PCWSTR(wide(path).as_ptr()));
        }
    }
    result
}

fn uninstall(uninstaller: &Path, keep_data: bool) -> Result<()> {
    let destination = uninstaller
        .parent()
        .context("uninstaller has no parent directory")?;
    validate_destination(destination)?;
    let manifest_file = destination.join(MANIFEST_NAME);
    let manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_file).with_context(|| {
            format!("installation manifest missing: {}", manifest_file.display())
        })?)?;
    if manifest["version"].as_u64() != Some(1) {
        bail!("unsupported installation manifest version")
    }
    let configured_name = uninstaller
        .file_name()
        .and_then(|name| name.to_str())
        .context("uninstaller filename is not Unicode")?;
    let bundle = BundleIndex::read(uninstaller)?.context("uninstaller bundle missing")?;
    let config = bundle.read_config()?;
    if configured_name
        != config["output"]["uninstaller_name"]
            .as_str()
            .unwrap_or("uninst.exe")
    {
        bail!("uninstaller name does not match the project")
    }
    let manifest_root = manifest["registry_root"]
        .as_str()
        .context("registry root missing")?;
    let manifest_path = manifest["registry_path"]
        .as_str()
        .context("registry path missing")?;
    let (root, expected_path) = uninstall_registry_key(&config)?;
    if manifest_root != registry_root_name(root) || manifest_path != expected_path {
        bail!("installation manifest registry target does not match the project")
    }
    let exe_name = config["install"]["exe_name"]
        .as_str()
        .context("install.exe_name is required")?;
    if bundle.contains(script::UNINSTALL_SCRIPT) {
        let stage = StagingDirectory::create()?;
        return script::run_uninstall(script::UninstallRequest {
            uninstaller: uninstaller.to_path_buf(),
            bundle,
            config,
            destination: destination.to_path_buf(),
            manifest,
            keep_data,
            stage: stage.0.clone(),
            root,
            registry_path: expected_path,
        });
    }
    if config["install"]["kill_process_on_uninstall"].as_bool() == Some(true)
        || config["install"]["detect_running_process"].as_bool() == Some(true)
    {
        shell::kill_processes(exe_name)
            .with_context(|| format!("cannot close the running {exe_name}"))?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    super::report_progress(45, "uninstall.status.removing_shortcuts")?;
    remove_recorded_artifacts(&manifest);
    if !keep_data {
        super::report_progress(60, "uninstall.status.removing_user_data")?;
        remove_user_data(&config)?;
    }
    super::report_progress(75, "uninstall.status.removing_files")?;
    remove_installed_files(destination, &manifest)?;
    super::report_progress(90, "uninstall.status.finishing")?;
    finish_uninstall(destination, uninstaller, root, &expected_path)?;
    shell::notify_shell();
    super::report_progress(100, "uninstall.status.complete")?;
    Ok(())
}

/// Removes the shortcut, autostart, and registry entries the manifest records.
pub(super) fn remove_recorded_artifacts(manifest: &Value) {
    InstallArtifacts::remove_recorded(manifest);
    remove_recorded_registry(manifest);
}

/// Deletes the per-user data directories `uninstall.data_paths` names.
pub(super) fn remove_user_data(config: &Value) -> Result<()> {
    for data_path in preserved_data_paths(config)? {
        if data_path.is_dir() {
            fs::remove_dir_all(&data_path)
                .with_context(|| format!("failed to remove {}", data_path.display()))?;
        } else if data_path.is_file() {
            fs::remove_file(&data_path)?;
        }
    }
    Ok(())
}

/// Deletes the deployed files the manifest lists, then the directories that
/// held only them. Directories that still hold anything are left in place.
pub(super) fn remove_installed_files(destination: &Path, manifest: &Value) -> Result<()> {
    for relative in manifest_file_paths(manifest)? {
        let path = destination.join(relative);
        if path.is_file() {
            fs::remove_file(&path)?;
        }
        for parent in path
            .ancestors()
            .skip(1)
            .take_while(|parent| *parent != destination)
        {
            let _ = fs::remove_dir(parent);
        }
    }
    Ok(())
}

/// Removes the uninstall registration and the manifest itself.
///
/// The uninstaller cannot delete its own file while it runs, so a helper copy
/// finishes that afterwards; see `spawn_cleanup_helper`. Everything else in the
/// directory is already gone by this point.
pub(super) fn finish_uninstall(
    destination: &Path,
    uninstaller: &Path,
    root: HKEY,
    registry_path: &str,
) -> Result<()> {
    // The registration is dropped first. A helper that removed the directory
    // but failed here would leave an entry that uninstalls nothing.
    unsafe {
        let status = RegDeleteKeyW(root, PCWSTR(wide(registry_path).as_ptr()));
        if status != ERROR_FILE_NOT_FOUND && status != ERROR_PATH_NOT_FOUND {
            status.ok()?;
        }
    }
    fs::remove_file(destination.join(MANIFEST_NAME))?;
    spawn_cleanup_helper(destination, uninstaller);
    Ok(())
}

/// Starts the copy that deletes the finished installation's own directory.
///
/// The running uninstaller keeps a lock on its own image, so a second process
/// has to remove it. The helper is a copy in the temporary directory, so
/// nothing in the installation is left holding the directory open.
fn spawn_cleanup_helper(destination: &Path, uninstaller: &Path) {
    if let Err(error) = try_spawn_cleanup_helper(destination, uninstaller) {
        // Without a helper the directory can only be emptied and then removed on
        // the next restart, which is the older behaviour, and a failed cleanup
        // must never fail the uninstall the user already completed.
        let _ = error;
        schedule_removal_at_reboot(uninstaller);
    }
}

fn try_spawn_cleanup_helper(destination: &Path, uninstaller: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use windows::Win32::System::Threading::CREATE_NO_WINDOW;

    let source = std::env::current_exe().context("cannot locate the running uninstaller")?;
    let mut helper = None;
    for attempt in 0..100 {
        let candidate = std::env::temp_dir().join(format!(
            "nano-installer-cleanup-{}-{attempt}.exe",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                drop(file);
                helper = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context("cannot create the cleanup helper"),
        }
    }
    let helper = helper.context("cannot allocate a cleanup helper path")?;
    fs::copy(&source, &helper).context("cannot copy the cleanup helper")?;
    let spawned = Command::new(&helper)
        .arg(super::CLEANUP_FLAG)
        .arg(destination)
        .arg(uninstaller)
        .creation_flags(CREATE_NO_WINDOW.0)
        .spawn();
    match spawned {
        Ok(_) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&helper);
            Err(error).context("cannot start the cleanup helper")
        }
    }
}

/// Asks Windows to delete `path` the next time the machine starts.
///
/// Writing the pending-rename list needs administrator rights, so this is the
/// fallback rather than the normal path.
fn schedule_removal_at_reboot(path: &Path) {
    unsafe {
        let _ = MoveFileExW(
            PCWSTR(wide(&path.display().to_string()).as_ptr()),
            PCWSTR::null(),
            MOVEFILE_DELAY_UNTIL_REBOOT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        preserved_data_paths, previous_install, register_uninstaller, registry_key_exists,
        registry_path, validate_destination, wide, Deployment, InstallArtifacts, PreviousInstall,
        MANIFEST_NAME,
    };
    use anyhow::Result;
    use std::path::{Path, PathBuf};
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegDeleteKeyW, HKEY_CURRENT_USER};

    struct TestRegistryKey(String);

    impl Drop for TestRegistryKey {
        fn drop(&mut self) {
            unsafe {
                let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(wide(&self.0).as_ptr()));
            }
        }
    }

    /// Config that points at the throwaway registry key the tests use.
    fn test_config() -> serde_json::Value {
        serde_json::json!({
            "registry": {"uninstall_key": "HKCU\\Software\\nano-installer-test"}
        })
    }

    fn write_payload(root: &Path, entries: &[(&str, &[u8])]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::with_capacity(entries.len());
        for (relative, contents) in entries {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("payload entry has a parent"))?;
            std::fs::write(&path, contents)?;
            files.push(PathBuf::from(relative));
        }
        Ok(files)
    }

    fn deploy(
        extracted: &Path,
        destination: &Path,
        files: &[PathBuf],
        previous: Option<&PreviousInstall>,
        backup_root: &Path,
        register: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        // An empty plan keeps the tests from touching the real desktop.
        let artifacts = InstallArtifacts::default();
        Deployment {
            extracted,
            destination,
            files,
            uninstaller_name: "uninst.exe",
            uninstaller: b"uninstaller",
            root: HKEY_CURRENT_USER,
            registry_path: "Software\\nano-installer-test",
            previous,
            backup_root,
            artifacts: &artifacts,
            registry_values: &[],
            registry_keys: &[],
        }
        .run(register)
    }

    fn manifest_files(destination: &Path) -> Result<Vec<String>> {
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(destination.join(MANIFEST_NAME))?)?;
        Ok(manifest["files"]
            .as_array()
            .expect("manifest file list")
            .iter()
            .map(|item| {
                item.as_str()
                    .expect("manifest path is a string")
                    .to_string()
            })
            .collect())
    }

    #[test]
    fn registry_path_normalizes_legacy_escaped_separators() {
        let (_, path) = registry_path("HKCU\\\\Software\\\\Microsoft\\Uninstall\\App").unwrap();
        assert_eq!(path, "Software\\Microsoft\\Uninstall\\App");
        assert!(registry_path("HKCR\\Somewhere").is_err());
    }

    #[test]
    fn refuses_relative_or_root_installation() {
        assert!(validate_destination(Path::new("relative\\path")).is_err());
        assert!(validate_destination(Path::new("C:\\")).is_err());
        assert!(validate_destination(Path::new("C:\\Program Files\\Example")).is_ok());
    }

    #[test]
    fn installs_a_fresh_directory_and_records_the_manifest() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let extracted = temp.path().join("extracted");
        let files = write_payload(
            &extracted,
            &[("TapTap.exe", b"app"), ("assets/config.ini", b"config")],
        )?;
        let destination = temp.path().join("installed");
        deploy(
            &extracted,
            &destination,
            &files,
            None,
            &temp.path().join("rollback"),
            || Ok(()),
        )?;
        assert_eq!(std::fs::read(destination.join("TapTap.exe"))?, b"app");
        assert_eq!(
            std::fs::read(destination.join("assets/config.ini"))?,
            b"config"
        );
        assert_eq!(
            manifest_files(&destination)?,
            vec!["TapTap.exe".to_string(), "assets/config.ini".to_string()]
        );
        assert!(destination.join("uninst.exe").is_file());
        Ok(())
    }

    #[test]
    fn refuses_to_install_over_a_directory_it_did_not_create() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let destination = temp.path().join("existing");
        std::fs::create_dir_all(&destination)?;
        std::fs::write(destination.join("user-data.txt"), b"keep me")?;
        assert!(previous_install(&destination, &test_config()).is_err());
        let extracted = temp.path().join("extracted");
        let files = write_payload(&extracted, &[("TapTap.exe", b"app")])?;
        assert!(deploy(
            &extracted,
            &destination,
            &files,
            None,
            &temp.path().join("rollback"),
            || Ok(()),
        )
        .is_err());
        assert_eq!(
            std::fs::read(destination.join("user-data.txt"))?,
            b"keep me"
        );
        assert!(!destination.join(MANIFEST_NAME).exists());
        Ok(())
    }

    #[test]
    fn refuses_a_foreign_manifest_at_the_destination() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let destination = temp.path().join("other-app");
        std::fs::create_dir_all(&destination)?;
        std::fs::write(
            destination.join(MANIFEST_NAME),
            serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "registry_root": "HKCU",
                "registry_path": "Software\\Other Vendor\\Uninstall\\Other App",
                "files": ["Other.exe"]
            }))?,
        )?;
        assert!(previous_install(&destination, &test_config()).is_err());
        Ok(())
    }

    #[test]
    fn an_upgrade_replaces_the_previous_version_and_drops_stale_files() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let rollback = temp.path().join("rollback");
        let destination = temp.path().join("installed");
        let first = temp.path().join("first");
        let files = write_payload(&first, &[("TapTap.exe", b"v1"), ("stale.txt", b"stale")])?;
        deploy(&first, &destination, &files, None, &rollback, || Ok(()))?;
        let previous = previous_install(&destination, &test_config())?.expect("previous install");

        let second = temp.path().join("second");
        let files = write_payload(&second, &[("TapTap.exe", b"v2"), ("fresh.txt", b"fresh")])?;
        deploy(
            &second,
            &destination,
            &files,
            Some(&previous),
            &rollback,
            || Ok(()),
        )?;
        assert_eq!(std::fs::read(destination.join("TapTap.exe"))?, b"v2");
        assert_eq!(std::fs::read(destination.join("fresh.txt"))?, b"fresh");
        assert!(!destination.join("stale.txt").exists());
        assert_eq!(
            manifest_files(&destination)?,
            vec!["TapTap.exe".to_string(), "fresh.txt".to_string()]
        );
        Ok(())
    }

    #[test]
    fn a_failed_upgrade_restores_the_previous_version() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let rollback = temp.path().join("rollback");
        let destination = temp.path().join("installed");
        let first = temp.path().join("first");
        let files = write_payload(&first, &[("TapTap.exe", b"v1"), ("old.txt", b"old")])?;
        deploy(&first, &destination, &files, None, &rollback, || Ok(()))?;
        let previous = previous_install(&destination, &test_config())?.expect("previous install");

        let second = temp.path().join("second");
        let files = write_payload(&second, &[("TapTap.exe", b"v2"), ("new.txt", b"new")])?;
        assert!(deploy(
            &second,
            &destination,
            &files,
            Some(&previous),
            &rollback,
            || anyhow::bail!("registry unavailable"),
        )
        .is_err());
        assert_eq!(std::fs::read(destination.join("TapTap.exe"))?, b"v1");
        assert_eq!(std::fs::read(destination.join("old.txt"))?, b"old");
        assert!(!destination.join("new.txt").exists());
        assert_eq!(
            manifest_files(&destination)?,
            vec!["TapTap.exe".to_string(), "old.txt".to_string()]
        );
        Ok(())
    }

    #[test]
    fn removes_a_fresh_install_whose_registration_fails() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let extracted = temp.path().join("extracted");
        let files = write_payload(&extracted, &[("TapTap.exe", b"app")])?;
        let destination = temp.path().join("installed");
        assert!(deploy(
            &extracted,
            &destination,
            &files,
            None,
            &temp.path().join("rollback"),
            || anyhow::bail!("registry unavailable"),
        )
        .is_err());
        assert!(!destination.exists());
        Ok(())
    }

    #[test]
    fn only_expands_data_paths_inside_a_user_profile() -> Result<()> {
        let appdata = std::env::var_os("APPDATA").expect("APPDATA is set on Windows");
        let appdata = PathBuf::from(appdata);
        let config = serde_json::json!({
            "uninstall": {
                "data_paths": [
                    "%APPDATA%\\NanoInstallerTest",
                    "%APPDATA%",
                    "%SystemRoot%",
                    "relative\\path",
                ]
            }
        });
        assert_eq!(
            preserved_data_paths(&config)?,
            vec![appdata.join("NanoInstallerTest")]
        );
        Ok(())
    }

    #[test]
    fn removes_recorded_shortcuts_and_only_their_empty_folder() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let start_menu = temp.path().join("Programs");
        let product = start_menu.join("TapTapTest");
        std::fs::create_dir_all(&product)?;
        let link = product.join("TapTap.lnk");
        std::fs::write(&link, b"")?;
        std::fs::write(product.join("kept.txt"), b"user file")?;
        let manifest = serde_json::json!({
            "shortcuts": [link.to_string_lossy()],
            "shortcut_dirs": [product.to_string_lossy()],
        });

        InstallArtifacts::remove_recorded(&manifest);

        assert!(!link.exists());
        // The folder still holds a file this installer never wrote.
        assert!(product.is_dir());
        assert!(start_menu.is_dir());
        Ok(())
    }

    #[test]
    fn drops_the_shortcut_folder_once_it_is_empty() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let start_menu = temp.path().join("Programs");
        let product = start_menu.join("TapTapTest");
        std::fs::create_dir_all(&product)?;
        let link = product.join("TapTap.lnk");
        std::fs::write(&link, b"")?;
        let manifest = serde_json::json!({
            "shortcuts": [link.to_string_lossy()],
            "shortcut_dirs": [product.to_string_lossy()],
        });

        InstallArtifacts::remove_recorded(&manifest);

        assert!(!link.exists());
        assert!(!product.exists());
        // The shared Start Menu root is never removed.
        assert!(start_menu.is_dir());
        Ok(())
    }

    #[test]
    fn ignores_data_paths_when_the_project_declares_none() -> Result<()> {
        let config = serde_json::json!({ "uninstall": {} });
        assert!(preserved_data_paths(&config)?.is_empty());
        Ok(())
    }

    #[test]
    #[ignore = "requires HKCU registry write access in an isolated Windows test environment"]
    fn registers_and_cleans_up_scoped_uninstall_key() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let key = TestRegistryKey(format!(
            "Software\\nano-installer-registry-test-{}",
            temp.path()
                .file_name()
                .expect("unique temporary directory")
                .to_string_lossy()
        ));
        let config = serde_json::json!({
            "project": {"name": "Temporary Test App", "version": "1.0", "publisher": "Test"}
        });
        assert!(!registry_key_exists(HKEY_CURRENT_USER, &key.0)?);
        register_uninstaller(
            HKEY_CURRENT_USER,
            &key.0,
            temp.path(),
            "uninst.exe",
            &config,
            false,
        )?;
        assert!(registry_key_exists(HKEY_CURRENT_USER, &key.0)?);
        assert!(register_uninstaller(
            HKEY_CURRENT_USER,
            &key.0,
            temp.path(),
            "uninst.exe",
            &config,
            false,
        )
        .is_err());
        register_uninstaller(
            HKEY_CURRENT_USER,
            &key.0,
            temp.path(),
            "uninst.exe",
            &config,
            true,
        )?;
        Ok(())
    }
}
