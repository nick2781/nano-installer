use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND};
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegDeleteTreeW, RegDeleteValueW, RegOpenKeyExW,
    RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE,
    KEY_READ, KEY_SET_VALUE, KEY_WOW64_32KEY, KEY_WOW64_64KEY, KEY_WRITE, REG_CREATED_NEW_KEY,
    REG_CREATE_KEY_DISPOSITION, REG_DWORD, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_QWORD,
    REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
};

use super::delta::UpdatePlan;
use super::{dependency, install_log, script, shell, BundleIndex, RuntimeMode, UI};

/// What the installer UI held when the user started the task.
///
/// A project script decides the steps, so the values the user left on the page
/// are as much its input as the directory to install into: it reads them
/// through `get_text_value` and `get_choice_value`.
#[derive(Default, Clone)]
pub(super) struct InstallSelection {
    destination: Option<String>,
    checkboxes: std::collections::HashMap<String, bool>,
    /// What each text field held, under the id the layout gave it.
    texts: std::collections::HashMap<String, String>,
    /// What each choice control held, under the id of the control that owns the
    /// choice: a select's own id, or the group a radio button belongs to.
    choices: std::collections::HashMap<String, String>,
}

impl InstallSelection {
    /// The checkbox states the installer UI held when the user started the task.
    pub(super) fn checkboxes(&self) -> &std::collections::HashMap<String, bool> {
        &self.checkboxes
    }

    /// What the page's text fields held when the user started the task.
    pub(super) fn texts(&self) -> &std::collections::HashMap<String, String> {
        &self.texts
    }

    /// What the page's choice controls held when the user started the task.
    pub(super) fn choices(&self) -> &std::collections::HashMap<String, String> {
        &self.choices
    }

    /// A selection carrying the values a page handed over, for a caller that
    /// drives the script without a window -- which is what a test does.
    #[cfg(test)]
    pub(super) fn from_values(
        texts: std::collections::HashMap<String, String>,
        choices: std::collections::HashMap<String, String>,
        checkboxes: std::collections::HashMap<String, bool>,
    ) -> Self {
        Self {
            texts,
            choices,
            checkboxes,
            ..Self::default()
        }
    }
}

pub(super) const MANIFEST_NAME: &str = "nano-installer-manifest.json";
static BUSY: AtomicBool = AtomicBool::new(false);
/// The task the window is watching, so a click can reach it.
static TASK: Mutex<Option<Cancellation>> = Mutex::new(None);
static STAGING_ID: AtomicU64 = AtomicU64::new(0);

/// A request to stop the task that is running.
///
/// The window asks and the worker reads, so the two share this one flag. The
/// wizard keeps the handle of the task it started, and the same handle travels
/// down the flow to the script that asks about it through `is_cancelled`.
#[derive(Clone, Default)]
pub(super) struct Cancellation(Arc<AtomicBool>);

impl Cancellation {
    /// True once the user has asked for this task to stop.
    pub(super) fn requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    /// Asks this task to stop at its next checkpoint.
    pub(super) fn request(&self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Asks the task that is running to stop.
///
/// The task is never killed where it stands. It reads the request at the
/// checkpoints between its steps -- and a project's own script reads it through
/// `is_cancelled` -- so whatever it has written can be undone before it gives
/// up.
pub(super) fn request_cancel() {
    if let Ok(slot) = TASK.lock() {
        if let Some(task) = slot.as_ref() {
            task.request();
        }
    }
}

/// The error a task reports when the user stopped it.
///
/// It carries its own type so the wizard can say the run was cancelled instead
/// of showing a failure the product never had.
#[derive(Debug)]
pub(super) struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "cancelled by the user")
    }
}

impl std::error::Error for Cancelled {}

/// Gives up when the user asked for it.
///
/// Every caller is a point between two steps of a task, which is what keeps a
/// cancellation from landing in the middle of one.
pub(super) fn check_cancelled(task: &Cancellation) -> Result<()> {
    if task.requested() {
        return Err(Cancelled.into());
    }
    Ok(())
}

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
                texts: state.interaction.text_input_values.clone(),
                choices: state.interaction.choices.clone(),
            })
        });
    let Some(selection) = selection else {
        show_result(
            Err(anyhow::anyhow!("the installer window is not ready")),
            None,
        );
        return;
    };
    run_worker(move |task| {
        let setup = std::env::current_exe()?;
        begin_log(install_log::RunKind::Install, None);
        let bundle = BundleIndex::read(&setup)?.context("installer resource bundle missing")?;
        let config = bundle.read_config()?;
        let destination =
            resolve_install_destination(&config, selection.destination.as_deref().map(Path::new))?;
        install_setup(&setup, &destination, &selection, task)
    });
}

/// Whether the page left the checkbox `id` ticked, with `fallback` standing in
/// for a page that carries no such checkbox at all.
impl InstallSelection {
    pub(super) fn checked(&self, id: &str, fallback: bool) -> bool {
        self.checkboxes.get(id).copied().unwrap_or(fallback)
    }
}

/// The components a run installs, in the order the project declares them.
///
/// A component is chosen by the checkbox that carries its id: the page the user
/// answered decides. A page that carries no such checkbox, and a silent run,
/// which has no page at all, leave the project's own `default` to decide, and a
/// component the project marked `required` is installed whatever the page says.
pub(super) fn selected_components(
    config: &Value,
    checked: impl Fn(&str, bool) -> bool,
) -> Vec<String> {
    let Some(items) = config["components"]["items"].as_array() else {
        return Vec::new();
    };
    let mut selected = Vec::new();
    for item in items {
        let Some(id) = item["id"].as_str() else {
            continue;
        };
        let default = item["default"].as_bool().unwrap_or(false);
        let required = item["required"].as_bool().unwrap_or(false);
        if required || checked(id, default) {
            selected.push(id.to_string());
        }
    }
    selected
}

/// The archives an install unfolds, with what to call each one when something is
/// wrong with it: the payload the project declares, then the payload of every
/// component this run installs.
fn payload_archives<'a>(
    config: &'a Value,
    components: &[String],
) -> Result<Vec<(&'a str, String)>> {
    let base = config["resources"]["payload_file"]
        .as_str()
        .context("resources.payload_file is required")?;
    let mut archives = vec![(base, "the payload".to_string())];
    let items = config["components"]["items"].as_array();
    for id in components {
        let payload = items
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item["id"].as_str() == Some(id.as_str()))
            })
            .and_then(|item| item["payload"].as_str())
            .with_context(|| format!("component {id} is not one this project declares"))?;
        archives.push((payload, format!("component {id}")));
    }
    Ok(archives)
}

/// The directory an install writes into.
///
/// A silent run can state one with `--dir`, which wins over the project's
/// `install.default_path`; the wizard passes whatever the path field holds.
/// Either way the value is expanded, because a configured path normally names
/// environment variables such as `%LOCALAPPDATA%\Product`, and an unexpanded
/// percent path is not absolute, so every install would be refused.
pub(super) fn resolve_install_destination(
    config: &Value,
    explicit: Option<&Path>,
) -> Result<PathBuf> {
    let raw = match explicit {
        Some(path) => path.to_path_buf(),
        None => {
            let configured = config["install"]["default_path"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .context(
                    "no installation directory: pass --dir, or set install.default_path in the project",
                )?;
            PathBuf::from(configured)
        }
    };
    let expanded = shell::expand_environment(&raw.to_string_lossy())?;
    let path = PathBuf::from(expanded.trim());
    ensure!(
        !path.as_os_str().is_empty(),
        "the installation directory is empty"
    );
    Ok(path)
}

/// The flags a windowless run takes.
///
/// A windowless run has no page and no message box, so its command line is its
/// only channel: it names where the product goes and where the record of the
/// run is left.
#[derive(Debug, Default, PartialEq, Eq)]
struct SilentOptions {
    /// The directory this run installs into, when the caller named one.
    directory: Option<PathBuf>,
    /// The file this run logs to, when the caller named one. Without it the log
    /// goes to the temporary directory, and the run leaves no other trace than
    /// its exit code.
    log: Option<PathBuf>,
}

/// Reads the flags a silent install accepts.
///
/// Only the installation directory and the log can be overridden. Anything else
/// is rejected rather than ignored, so a mistyped option cannot quietly install
/// somewhere nobody asked for.
fn parse_silent_arguments(arguments: &[std::ffi::OsString]) -> Result<SilentOptions> {
    let mut options = SilentOptions::default();
    let mut index = 0;
    while index < arguments.len() {
        let argument = arguments[index].to_string_lossy().to_string();
        let slot = match argument.as_str() {
            "--dir" => &mut options.directory,
            "--log" => &mut options.log,
            other => bail!("unsupported silent option: {other}"),
        };
        let value = arguments
            .get(index + 1)
            .with_context(|| format!("{argument} needs the path it names"))?;
        *slot = Some(PathBuf::from(value));
        index += 2;
    }
    Ok(options)
}

/// Opens the log a run writes to, for a machine that still gets the product
/// when it cannot take one.
///
/// Nothing about an installation depends on its log, so a setup that cannot
/// create the file carries on without it: the wizard keeps its own tail of the
/// run in memory, and a windowless run reports through its exit code.
fn begin_log(kind: install_log::RunKind, at: Option<PathBuf>) {
    if let Err(error) = install_log::begin(kind, at) {
        eprintln!("cannot write the log of this run: {error:#}");
    }
}

/// Records how a windowless run ended and reports it the way its caller reads
/// it: nothing on success, and on failure the error followed by the file that
/// holds the whole run.
fn finish_run(result: Result<()>) -> Result<()> {
    let outcome = match result.as_ref() {
        Ok(()) => install_log::Outcome::Finished,
        Err(error) => install_log::Outcome::Failed(error),
    };
    match install_log::end(outcome) {
        Some(line) => result.map_err(|error| anyhow::anyhow!("{error:#}\n{line}")),
        None => result,
    }
}

/// Records which product this run is about and what it does with it.
///
/// Written once the project's own configuration has been read, which is the
/// first moment the run knows the name a support reader would recognize.
fn describe_run(config: &Value, directory: &Path, what: &str) {
    let name = config["project"]["name"].as_str().unwrap_or("unnamed");
    let version = config["project"]["version"]
        .as_str()
        .filter(|version| !version.is_empty())
        .map(|version| format!(" {version}"))
        .unwrap_or_default();
    install_log::describe(&[
        ("product", format!("{name}{version}")),
        (what, directory.display().to_string()),
    ]);
}

/// Refuses a windowless run for a project that never declared one.
///
/// `advanced.silent_mode_support` and `advanced.uninstall_mode_support` are the
/// project's own statement that its flow works without a window, so a flag its
/// author did not agree to cannot install or remove a product unattended.
fn require_silent_support(config: &Value, key: &str) -> Result<()> {
    if config["advanced"][key].as_bool() == Some(true) {
        return Ok(());
    }
    bail!(
        "this project does not support a windowless run: set advanced.{key} to true in installer_config.json"
    )
}

/// Runs an install with no window, for a project that declares silent support.
///
/// The flags after `--silent` are the only input such a run has, so the image
/// is read once here to answer the two questions that have to be settled before
/// anything is written: whether the project agreed to a windowless run, and
/// where it wants to be installed. The read is the bundle index alone, a footer
/// and a small table; the payload stays on disk and is streamed later.
pub(super) fn run_silent_install(arguments: &[std::ffi::OsString]) -> Result<()> {
    let mut options = parse_silent_arguments(arguments)?;
    let setup = std::env::current_exe()?;
    // The log opens before the image is read, so a run that is refused for what
    // the image says still leaves a record of why.
    begin_log(install_log::RunKind::Install, options.log.take());
    finish_run(install_silently(&setup, &options))
}

fn install_silently(setup: &Path, options: &SilentOptions) -> Result<()> {
    let bundle = BundleIndex::read(setup)?.context("installer resource bundle missing")?;
    let config = bundle.read_config()?;
    require_silent_support(&config, "silent_mode_support")?;
    let destination = resolve_install_destination(&config, options.directory.as_deref())?;
    // No window means no checkbox and no field is there to be read, so every
    // shortcut, autostart and value decision falls back to what the project
    // configured.
    let selection = InstallSelection::default();
    install_setup(setup, &destination, &selection, &Cancellation::default())
}

/// Runs an uninstall with no window, for a project that declares silent support.
///
/// User data is kept: a silent run has no keep-data checkbox to clear, and the
/// runtime never destroys data the user did not ask to remove.
pub(super) fn run_silent_uninstall(arguments: &[std::ffi::OsString]) -> Result<()> {
    let options = parse_silent_arguments(arguments)?;
    ensure!(
        options.directory.is_none(),
        "the uninstaller accepts no option that names a directory: the product is removed from where it is"
    );
    let uninstaller = std::env::current_exe()?;
    begin_log(install_log::RunKind::Uninstall, options.log);
    finish_run(uninstall_removals(&uninstaller))
}

fn uninstall_removals(uninstaller: &Path) -> Result<()> {
    let bundle = BundleIndex::read(uninstaller)?.context("uninstaller bundle missing")?;
    let config = bundle.read_config()?;
    require_silent_support(&config, "uninstall_mode_support")?;
    uninstall(uninstaller, true, &Cancellation::default())
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
    run_worker(move |task| {
        let uninstaller = std::env::current_exe()?;
        begin_log(install_log::RunKind::Uninstall, None);
        uninstall(&uninstaller, keep_data, task)
    });
}

fn run_worker(work: impl FnOnce(&Cancellation) -> Result<()> + Send + 'static) {
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    // The wizard keeps this handle while the task runs, which is how a click
    // reaches it; the next task gets one of its own.
    let task = Cancellation::default();
    if let Ok(mut slot) = TASK.lock() {
        *slot = Some(task.clone());
    }
    // The task reports on the progress page and ends on the completion page. A
    // project that orders its pages differently names those two with `role`;
    // otherwise the second page and the last one are them, and a list with a
    // single page leaves the wizard where it is.
    let progress = super::page_index_for_role("progress");
    let completion = super::page_index_for_role("finish");
    if let Some(index) = progress {
        let _ = super::show_page(index);
    }
    std::thread::spawn(move || {
        let result = work(&task);
        if let Ok(mut slot) = TASK.lock() {
            *slot = None;
        }
        BUSY.store(false, Ordering::Release);
        let stopped = result
            .as_ref()
            .err()
            .is_some_and(|error| error.downcast_ref::<Cancelled>().is_some());
        // A stop is the user's own doing, so it is not a failure the log has to
        // explain; anything else leaves the run's log as the report of it.
        let log_line = match (&result, stopped) {
            (Ok(()), _) => {
                install_log::end(install_log::Outcome::Finished);
                None
            }
            (Err(_), true) => {
                install_log::end(install_log::Outcome::Stopped);
                None
            }
            (Err(error), false) => install_log::end(install_log::Outcome::Failed(error)),
        };
        match &result {
            // The completion page reports the outcome and owns the next action.
            Ok(()) => match completion {
                Some(index) => {
                    let _ = super::show_page(index);
                }
                None => show_result(Ok(()), None),
            },
            // Stopping is the user's own doing, so the wizard returns to the
            // page the task was started from instead of reporting a failure.
            Err(_) if stopped => {
                let _ = super::show_page(0);
                super::show_notice("Operation cancelled");
            }
            Err(error) => {
                let _ = super::show_page(0);
                show_result(Err(anyhow::anyhow!("{error:#}")), log_line);
            }
        }
    });
}

/// Reports how the task ended.
///
/// A project with a completion page shows the outcome there. A project without
/// one still has to be told, and that notice is drawn inside the installer
/// window so it carries the product's skin rather than the system default.
fn show_result(result: Result<()>, log_line: Option<String>) {
    super::show_notice(&result_notice(result, log_line.as_deref()));
}

/// The words the wizard shows when a task ends.
///
/// A failure also names the file the run was written to, which is the one thing
/// a user can hand over when something goes wrong on a machine nobody here can
/// reach.
fn result_notice(result: Result<()>, log_line: Option<&str>) -> String {
    match result {
        Ok(()) => "Operation complete".to_string(),
        Err(error) => match log_line {
            Some(line) => format!("Operation failed:\n{error:#}\n\n{line}"),
            None => format!("Operation failed:\n{error:#}"),
        },
    }
}

fn install_setup(
    setup: &Path,
    destination: &Path,
    selection: &InstallSelection,
    task: &Cancellation,
) -> Result<()> {
    validate_destination(destination)?;
    super::report_progress(5, "status.preparing")?;
    let bundle = BundleIndex::read(setup)?.context("installer resource bundle missing")?;
    let config = bundle.read_config()?;
    describe_run(&config, destination, "install into");
    install_log::note("info", "preparing the installation");
    let prep = prepare_install(&bundle, &config, destination)?;

    let stage = StagingDirectory::create()?;
    let extracted = stage.0.join("files");
    let backups = stage.0.join("rollback");
    // What this run installs, decided once: the built-in flow unfolds these
    // payloads, and a project script reads the same answer.
    let components = selected_components(&config, |id, default| selection.checked(id, default));
    if bundle.contains(script::INSTALL_SCRIPT) {
        install_log::note("info", "running the project's own install script");
        return script::run_install(script::InstallRequest {
            setup: setup.to_path_buf(),
            bundle,
            config,
            destination: destination.to_path_buf(),
            components,
            selection: selection.clone(),
            stage: stage.0.clone(),
            prep,
            cancel: task.clone(),
        });
    }
    // What the product needs from the machine comes before what the product is:
    // a machine that cannot be given it is a machine the product would not
    // start on, and saying so costs nothing while the destination is still
    // untouched.
    dependency::install_missing(&config, &bundle, &stage.0, task)?;
    super::report_progress(15, "status.extracting")?;
    install_log::note("info", "unpacking the payload");
    let files = extract_payload(PayloadRequest {
        setup,
        bundle: &bundle,
        config: &config,
        destination,
        scratch: &stage.0,
        target: &extracted,
        components: &components,
        task,
    })?;
    // A cancel is most likely to arrive while the payload unpacks, and this is
    // the last moment it costs nothing: nothing outside the staging directory
    // has been written yet.
    check_cancelled(task)?;
    super::report_progress(50, "status.deploying")?;
    // What an update leaves where it is says as much about the run as what it
    // writes, and a silent run has nothing else to report it through.
    let (deployed, kept) = files.counts();
    if files.update {
        install_log::note(
            "info",
            &format!("update package: {kept} file(s) verified in place, {deployed} deployed"),
        );
    }
    install_log::note("info", &format!("deploying {deployed} files"));

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
    // What the installation will hold, which is what the uninstall entry
    // reports the size of.
    let installed = files.all();
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
        services: &[],
        task,
    }
    .run(|| {
        register_uninstaller(
            prep.root,
            &prep.registry_path,
            destination,
            &prep.uninstaller_name,
            &config,
            prep.upgrade,
            &installed,
        )
    })?;
    super::report_progress(95, "status.finishing")?;
    install_log::note("info", "the product is deployed");
    super::record_installed_app(destination.join(exe_name))?;
    super::report_progress(100, "status.install_complete")?;
    install_log::note("info", "the installation is complete");
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
    let key = uninstall_registry_key(config)?;
    let previous = previous_install(destination, config)?;
    let upgrade = previous.is_some();
    if !upgrade && key.exists()? {
        bail!("uninstall registry key already exists; refusing to overwrite another installation");
    }
    if let Some(required_mb) = config["install"]["required_space_mb"].as_u64() {
        require_free_space(destination, required_mb)?;
    }
    Ok(InstallPrep {
        uninstaller_name,
        uninstaller,
        root: key.root,
        registry_path: key.path,
        previous,
        upgrade,
    })
}

/// Refuses the install before anything is written when the destination drive holds less free
/// space than `install.required_space_mb` asks for.
fn require_free_space(destination: &Path, required_mb: u64) -> Result<()> {
    let drive = super::disk_root(destination).context("cannot determine installation drive")?;
    let free = super::query_disk_free_bytes(&drive)?;
    let required = required_mb.saturating_mul(1024 * 1024);
    if free < required {
        bail!(
            "not enough disk space: {required_mb} MiB required on {}",
            drive.display()
        );
    }
    Ok(())
}

/// What an install writes, and what an update package leaves where it is.
///
/// An update package carries only the files whose bytes changed, so the rest of
/// the release is already on disk; those files are not deployed, and they still
/// belong to the installation the manifest describes.
pub(super) struct PayloadFiles {
    /// Files this run writes out of the payloads it unpacked.
    pub(super) deployed: Vec<PathBuf>,
    /// Files an update package found on disk, verified, and left alone.
    pub(super) kept: Vec<PathBuf>,
    /// Whether the setup that carried the payload was an update package, which
    /// is what tells a deployment that an unchanged target is already right.
    pub(super) update: bool,
}

impl PayloadFiles {
    /// Every file the installation holds once the run is done, which is what
    /// the manifest records and what an uninstall has to take back.
    pub(super) fn all(&self) -> Vec<PathBuf> {
        let mut files = self.deployed.clone();
        files.extend(self.kept.iter().cloned());
        files.sort();
        files
    }

    /// How many files the run deploys, and how many an update kept in place.
    pub(super) fn counts(&self) -> (usize, usize) {
        (self.deployed.len(), self.kept.len())
    }
}

/// Everything one payload extraction needs from the run that asked for it.
///
/// The built-in install and a project script both unpack the same payloads, so
/// they hand the same answer to the same call rather than each carrying a long
/// argument list of their own.
pub(super) struct PayloadRequest<'a> {
    /// The setup or uninstaller image whose bundle holds the payloads.
    pub(super) setup: &'a Path,
    pub(super) bundle: &'a BundleIndex,
    pub(super) config: &'a Value,
    /// Where the product is being installed, which is what an update package is
    /// checked against.
    pub(super) destination: &'a Path,
    /// The scratch directory the payload archives are streamed into.
    pub(super) scratch: &'a Path,
    /// Where the expanded files are assembled before they are deployed.
    pub(super) target: &'a Path,
    pub(super) components: &'a [String],
    pub(super) task: &'a Cancellation,
}

/// Streams the payload out of the setup image and expands it into `target`.
///
/// The payload is copied to disk first and expanded by the stub that matches
/// its format, so a large archive never has to fit in the process address
/// space. The expanded file list is validated before it is deployed.
///
/// An update package is checked here rather than later: what it expects to find
/// on disk is verified before the first file is written, so a machine that holds
/// another release is told so instead of ending up with half of each version.
pub(super) fn extract_payload(request: PayloadRequest<'_>) -> Result<PayloadFiles> {
    let PayloadRequest {
        setup,
        bundle,
        config,
        destination,
        scratch,
        target,
        components,
        task,
    } = request;
    let uninstaller_name = config["output"]["uninstaller_name"]
        .as_str()
        .unwrap_or("uninst.exe");
    let product = config["project"]["name"]
        .as_str()
        .or_else(|| config["project"]["output_name"].as_str())
        .unwrap_or("this product");
    let mut kept: Vec<PathBuf> = Vec::new();
    let mut update = false;
    if let Some(plan) = UpdatePlan::read(bundle)? {
        plan.requires_installed_product(destination, product)?;
        plan.verify(destination, product)?;
        kept = plan.kept_paths();
        update = true;
    }
    // Every part is unpacked on its own, so the parts can be compared before they
    // meet: two archives that carry one path would otherwise take turns silently
    // overwriting each other, and which file ends up installed would depend on the
    // order. The payload the project declares unpacks straight into the staging
    // directory, which is the whole job for a project that declares no components.
    let mut files: Vec<PathBuf> = Vec::new();
    for (index, (payload_name, label)) in payload_archives(config, components)?
        .into_iter()
        .enumerate()
    {
        if !bundle.contains(payload_name) {
            bail!("{label} is missing: {payload_name}");
        }
        let archive = scratch.join(format!("payload-{index}.archive"));
        bundle.copy_file_to(payload_name, &archive)?;
        let part = if index == 0 {
            target.to_path_buf()
        } else {
            let part = scratch.join(format!("payload-{index}"));
            fs::create_dir_all(&part)?;
            part
        };
        extract_archive(setup, &archive, &part, task)?;
        let unpacked = collect_staged_files(&part)?;
        if unpacked.is_empty() {
            bail!("{label} carries no files")
        }
        if let Some(path) = unpacked.iter().find(|path| files.contains(path)) {
            bail!(
                "{label} carries {}, which is already installed by another payload",
                path.display()
            );
        }
        if let Some(path) = unpacked.iter().find(|path| kept.contains(path)) {
            bail!(
                "{label} carries {}, which this update expects to find on the machine and not to \
                 install",
                path.display()
            );
        }
        merge_staged(&part, target, &unpacked)?;
        files.extend(unpacked);
        files.sort();
    }
    let mut installed = files.clone();
    installed.extend(kept.iter().cloned());
    installed.sort();
    if installed
        .iter()
        .any(|path| path == Path::new(uninstaller_name) || path == Path::new(MANIFEST_NAME))
    {
        bail!("payload conflicts with installer-managed files")
    }
    let exe_name = config["install"]["exe_name"]
        .as_str()
        .context("install.exe_name is required")?;
    super::validate_output_filename(exe_name, "install.exe_name")?;
    if !installed.iter().any(|path| path == Path::new(exe_name)) {
        bail!("payload does not contain the configured application executable: {exe_name}")
    }
    Ok(PayloadFiles {
        deployed: files,
        kept,
        update,
    })
}

/// Moves the files a part unpacked into the directory the payload deploys from.
///
/// The parts are unpacked side by side so they can be compared, and a rename
/// inside the staging directory costs nothing next to copying a product's worth
/// of files; the part that unpacked straight into `target` has nothing to move.
fn merge_staged(part: &Path, target: &Path, files: &[PathBuf]) -> Result<()> {
    if part == target {
        return Ok(());
    }
    for relative in files {
        let destination = target.join(relative);
        fs::create_dir_all(
            destination
                .parent()
                .context("payload target has no parent")?,
        )?;
        fs::rename(part.join(relative), &destination)?;
    }
    Ok(())
}

/// Unpacks the payload with the runtime embedded in the setup image.
///
/// The wait is a poll rather than a blocking call, because unpacking is the
/// longest step an install takes: a user who asks to stop has to be able to
/// stop it here, and what has been unpacked so far is in the staging directory,
/// which nobody keeps.
fn extract_archive(setup: &Path, archive: &Path, target: &Path, task: &Cancellation) -> Result<()> {
    let mut child = std::process::Command::new(setup)
        .arg("--extract")
        .arg(archive)
        .arg(target)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to start installer extraction backend")?;
    loop {
        if task.requested() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Cancelled.into());
        }
        match child.try_wait()? {
            Some(status) if status.success() => return Ok(()),
            Some(_) => {
                let mut message = String::new();
                if let Some(mut errors) = child.stderr.take() {
                    let _ = errors.read_to_string(&mut message);
                }
                bail!("payload extraction failed: {}", message.trim());
            }
            None => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    }
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
    let expected = uninstall_registry_key(config)?;
    let root_name = registry_root_name(expected.root);
    if manifest["registry_root"].as_str() != Some(root_name)
        || manifest["registry_path"].as_str() != Some(expected.path.as_str())
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
    files: &'a PayloadFiles,
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
    /// Services a project script installed, deleted by the uninstaller.
    services: &'a [String],
    /// The task asking for this deployment, so it can stop between two files.
    task: &'a Cancellation,
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
        // What the installation holds once the run is done: the files it wrote
        // plus the ones an update package verified and left where they were.
        let files = self.files.all();
        deploy_files(
            self.extracted,
            self.destination,
            self.files,
            self.previous,
            journal,
            self.task,
        )?;
        // The payload is on disk; a cancel that arrived while it was being
        // copied stops before the uninstaller and the manifest are written, so
        // nothing records a product the user never got.
        check_cancelled(self.task)?;
        let uninstaller = self.destination.join(self.uninstaller_name);
        journal.track(&uninstaller)?;
        fs::write(&uninstaller, self.uninstaller)?;

        self.artifacts.apply(journal)?;

        write_manifest(
            self.destination,
            journal,
            registry_root_name(self.root),
            self.registry_path,
            &files,
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
                services: self.services.to_vec(),
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
    /// Services a project script installed, by the name the machine knows each
    /// one by.
    pub(super) services: Vec<String>,
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
        "services": artifacts.services,
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
///
/// A target an update package already holds, byte for byte, is not rewritten:
/// the plan promises those bytes, and copying them again would only cost the
/// time the update saved. The files the plan kept are left where they are for
/// the same reason.
pub(super) fn deploy_files(
    extracted: &Path,
    destination: &Path,
    files: &PayloadFiles,
    previous: Option<&PreviousInstall>,
    journal: &mut RollbackJournal,
    task: &Cancellation,
) -> Result<()> {
    for relative in &files.deployed {
        // Between two files rather than halfway through one: the journal has
        // everything copied so far and puts it back when the deploy fails.
        check_cancelled(task)?;
        let source = extracted.join(relative);
        let target = destination.join(relative);
        if files.update && same_contents(&source, &target)? {
            continue;
        }
        fs::create_dir_all(target.parent().context("payload target has no parent")?)?;
        journal.track(&target)?;
        fs::copy(source, target)?;
    }
    if let Some(previous) = previous {
        let installed = files.all();
        for relative in &previous.files {
            if installed.iter().any(|path| path == relative) {
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

/// Whether two files hold the same bytes.
///
/// Read in chunks rather than in one piece: a payload file can be an
/// application's whole runtime, and comparing two of them must not depend on
/// how much memory is free.
fn same_contents(left: &Path, right: &Path) -> Result<bool> {
    let (Ok(left_meta), Ok(right_meta)) = (fs::metadata(left), fs::metadata(right)) else {
        return Ok(false);
    };
    if !left_meta.is_file() || left_meta.len() != right_meta.len() {
        return Ok(false);
    }
    let mut left = io::BufReader::new(fs::File::open(left)?);
    let mut right = io::BufReader::new(fs::File::open(right)?);
    let mut left_buffer = [0u8; 64 * 1024];
    let mut right_buffer = [0u8; 64 * 1024];
    loop {
        let left_len = read_chunk(&mut left, &mut left_buffer)?;
        let right_len = read_chunk(&mut right, &mut right_buffer)?;
        if left_len != right_len {
            return Ok(false);
        }
        if left_len == 0 {
            return Ok(true);
        }
        if left_buffer[..left_len] != right_buffer[..right_len] {
            return Ok(false);
        }
    }
}

/// Fills `buffer` unless the file ends first, and reports how much it filled.
fn read_chunk(reader: &mut impl io::Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
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

/// Which view of the registry a key belongs to.
///
/// A 64-bit Windows keeps a second copy of `HKLM\SOFTWARE` for 32-bit programs
/// and shows each process the copy it was built for. A project that has to reach
/// the other one names the view on the hive, the way an Inno Setup script does:
/// `HKLM32\SOFTWARE\...` is the copy a 32-bit program reads, `HKLM64\...` the one
/// a 64-bit program reads, and a bare `HKLM` is whichever this process is
/// subject to.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum RegistryView {
    /// The view this process itself is subject to.
    #[default]
    Native,
    /// The 32-bit view, `WOW6432Node` under a 64-bit Windows.
    Wow6432,
    /// The 64-bit view.
    Wow64,
}

impl RegistryView {
    /// What the registry calls take to reach this view: the native view is the
    /// absence of a flag rather than a third flag.
    fn flags(self) -> REG_SAM_FLAGS {
        match self {
            Self::Native => REG_SAM_FLAGS(0),
            Self::Wow6432 => KEY_WOW64_32KEY,
            Self::Wow64 => KEY_WOW64_64KEY,
        }
    }

    fn is_native(self) -> bool {
        self == Self::Native
    }
}

/// A registry key a project names: the hive it hangs from, the subkey below it,
/// and the view it is read and written in.
#[derive(Clone, Debug)]
pub(super) struct RegistryKey {
    pub(super) root: HKEY,
    pub(super) path: String,
    pub(super) view: RegistryView,
}

impl RegistryKey {
    /// A key in the view this process is subject to, which is where the built-in
    /// flow writes and what a project that names no view means.
    pub(super) fn new(root: HKEY, path: impl Into<String>) -> Self {
        Self {
            root,
            path: path.into(),
            view: RegistryView::Native,
        }
    }

    /// Opens the key for the access asked for, `None` when it is not there.
    fn open(&self, access: REG_SAM_FLAGS) -> Result<Option<HKEY>> {
        let mut key = Default::default();
        let opened = unsafe {
            RegOpenKeyExW(
                self.root,
                PCWSTR(wide(&self.path).as_ptr()),
                0,
                access | self.view.flags(),
                &mut key,
            )
        };
        if opened == ERROR_FILE_NOT_FOUND || opened == ERROR_PATH_NOT_FOUND {
            return Ok(None);
        }
        opened.ok()?;
        Ok(Some(key))
    }

    /// Opens the key for writing, creating it and its parents when needed.
    fn create(&self, access: REG_SAM_FLAGS) -> Result<HKEY> {
        let mut key = Default::default();
        unsafe {
            RegCreateKeyExW(
                self.root,
                PCWSTR(wide(&self.path).as_ptr()),
                0,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                access | self.view.flags(),
                None,
                &mut key,
                None,
            )
            .ok()?;
        }
        Ok(key)
    }

    /// Whether the key is there.
    pub(super) fn exists(&self) -> Result<bool> {
        let Some(key) = self.open(KEY_READ)? else {
            return Ok(false);
        };
        unsafe {
            RegCloseKey(key).ok()?;
        }
        Ok(true)
    }

    /// The value as the machine stores it: its type and its bytes.
    pub(super) fn read_raw(&self, name: &str) -> Result<Option<(REG_VALUE_TYPE, Vec<u8>)>> {
        let Some(key) = self.open(KEY_QUERY_VALUE)? else {
            return Ok(None);
        };
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
        if status.is_err() {
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
        Ok(Some((kind, buffer)))
    }

    /// Reads a single `REG_SZ` value: `None` when the value is absent or is of
    /// another type.
    pub(super) fn read_string(&self, name: &str) -> Result<Option<String>> {
        match self.read_raw(name)? {
            Some((kind, buffer)) if kind == REG_SZ => Ok(Some(utf16_text(&buffer))),
            _ => Ok(None),
        }
    }

    /// Reads a single value as text, whatever scalar type the machine stored it
    /// as.
    ///
    /// A dependency's detection rule names the value a product writes when it is
    /// installed, and vendors disagree about the type: the VC++ runtimes write a
    /// `REG_DWORD` of 1, the WebView2 runtime writes its version as `REG_SZ`,
    /// .NET Framework records a release number as a `REG_DWORD`, and a version
    /// has been seen as a `REG_QWORD`. All of them read as text here, which is
    /// what a comparison needs; a value of another type reads as absent rather
    /// than guessed at. `REG_EXPAND_SZ` is text with `%VAR%` references in it,
    /// and the caller compares it as written.
    pub(super) fn read_text(&self, name: &str) -> Result<Option<String>> {
        let Some((kind, buffer)) = self.read_raw(name)? else {
            return Ok(None);
        };
        if kind == REG_DWORD && buffer.len() >= 4 {
            return Ok(Some(
                u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]).to_string(),
            ));
        }
        if kind == REG_QWORD && buffer.len() >= 8 {
            let mut number = [0u8; 8];
            number.copy_from_slice(&buffer[..8]);
            return Ok(Some(u64::from_le_bytes(number).to_string()));
        }
        if kind == REG_SZ || kind == REG_EXPAND_SZ {
            return Ok(Some(utf16_text(&buffer)));
        }
        Ok(None)
    }

    /// Writes a single value of the given type, creating the key when needed.
    pub(super) fn write(&self, name: &str, kind: REG_VALUE_TYPE, bytes: &[u8]) -> Result<()> {
        let key = self.create(KEY_SET_VALUE)?;
        let status =
            unsafe { RegSetValueExW(key, PCWSTR(wide(name).as_ptr()), 0, kind, Some(bytes)) };
        unsafe {
            let _ = RegCloseKey(key);
        }
        status.ok()?;
        Ok(())
    }

    /// Writes a single `REG_SZ` value.
    pub(super) fn write_string(&self, name: &str, value: &str) -> Result<()> {
        let encoded = wide(value);
        let bytes =
            unsafe { std::slice::from_raw_parts(encoded.as_ptr().cast::<u8>(), encoded.len() * 2) };
        self.write(name, REG_SZ, bytes)
    }

    /// Writes a single `REG_DWORD` value.
    pub(super) fn write_dword(&self, name: &str, value: u32) -> Result<()> {
        self.write(name, REG_DWORD, &value.to_le_bytes())
    }

    /// Deletes a single value; a missing key or value is not an error.
    pub(super) fn delete_value(&self, name: &str) -> Result<()> {
        let Some(key) = self.open(KEY_SET_VALUE)? else {
            return Ok(());
        };
        let status = unsafe { RegDeleteValueW(key, PCWSTR(wide(name).as_ptr())) };
        unsafe {
            let _ = RegCloseKey(key);
        }
        if status != ERROR_FILE_NOT_FOUND && status != ERROR_PATH_NOT_FOUND {
            status.ok()?;
        }
        Ok(())
    }

    /// Deletes the key and everything below it; a key that is not there is not an
    /// error.
    ///
    /// The view belongs to a handle rather than to a call, so the tree comes down
    /// through a handle on the branch above it, opened in this key's view. A key
    /// with no branch above it is refused: `HKCU\Software` and its like belong to
    /// Windows and to every product on the machine.
    pub(super) fn delete_key(&self) -> Result<()> {
        let (parent, name) = self.path.rsplit_once('\\').with_context(|| {
            format!(
                "refusing to delete {} itself: name a subkey below it",
                self.path
            )
        })?;
        let parent = Self {
            root: self.root,
            path: parent.to_string(),
            view: self.view,
        };
        let Some(key) = parent.open(KEY_READ | KEY_WRITE)? else {
            return Ok(());
        };
        let status = unsafe { RegDeleteTreeW(key, PCWSTR(wide(name).as_ptr())) };
        unsafe {
            let _ = RegCloseKey(key);
        }
        if status != ERROR_FILE_NOT_FOUND && status != ERROR_PATH_NOT_FOUND {
            status.ok()?;
        }
        Ok(())
    }
}

/// Reads a single `REG_SZ` value, returning `None` when it is absent.
pub(super) fn read_registry_string(root: HKEY, path: &str, name: &str) -> Result<Option<String>> {
    RegistryKey::new(root, path).read_string(name)
}

/// Decodes a `REG_SZ` payload, which runs to the first NUL.
pub(super) fn utf16_text(buffer: &[u8]) -> String {
    let units = buffer
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|unit| *unit != 0)
        .collect::<Vec<_>>();
    String::from_utf16_lossy(&units)
}

/// Writes a single `REG_SZ` value, creating the key when needed.
pub(super) fn write_registry_string(root: HKEY, path: &str, name: &str, value: &str) -> Result<()> {
    RegistryKey::new(root, path).write_string(name, value)
}

/// Deletes a whole key and its subkeys; a missing key is not an error.
pub(super) fn delete_registry_key(root: HKEY, path: &str) -> Result<()> {
    RegistryKey::new(root, path).delete_key()
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
            if let Ok(key) = parse_registry_key(path) {
                let _ = key.delete_value(name);
            }
        }
    }
    if let Some(keys) = manifest["registry_keys"].as_array() {
        for key in keys.iter().filter_map(Value::as_str) {
            if let Ok(key) = parse_registry_key(key) {
                let _ = key.delete_key();
            }
        }
    }
}

/// Deletes a single value; a missing key or value is not an error.
pub(super) fn delete_registry_value(root: HKEY, path: &str, name: &str) -> Result<()> {
    RegistryKey::new(root, path).delete_value(name)
}

pub(super) fn uninstall_registry_key(config: &Value) -> Result<RegistryKey> {
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
    parse_registry_key(&configured)
}

/// Parses a key a project names, hive first.
///
/// The hive may carry the view to read it in: `HKLM32\SOFTWARE\...` is the copy
/// a 32-bit program sees, `HKLM64\...` the one a 64-bit program sees. A hive
/// without a suffix means the view this process itself is subject to.
pub(super) fn parse_registry_key(raw: &str) -> Result<RegistryKey> {
    let mut parts = raw.split('\\').filter(|part| !part.is_empty());
    let hive = parts.next().unwrap_or_default().to_ascii_uppercase();
    let (root, view) = match hive.as_str() {
        "HKCU" | "HKEY_CURRENT_USER" => (HKEY_CURRENT_USER, RegistryView::Native),
        "HKLM" | "HKEY_LOCAL_MACHINE" => (HKEY_LOCAL_MACHINE, RegistryView::Native),
        "HKCU32" | "HKEY_CURRENT_USER32" => (HKEY_CURRENT_USER, RegistryView::Wow6432),
        "HKLM32" | "HKEY_LOCAL_MACHINE32" => (HKEY_LOCAL_MACHINE, RegistryView::Wow6432),
        "HKCU64" | "HKEY_CURRENT_USER64" => (HKEY_CURRENT_USER, RegistryView::Wow64),
        "HKLM64" | "HKEY_LOCAL_MACHINE64" => (HKEY_LOCAL_MACHINE, RegistryView::Wow64),
        _ => bail!("registry key must start with HKCU or HKLM, optionally with a 32 or 64 suffix"),
    };
    let path = parts.collect::<Vec<_>>().join("\\");
    if path.is_empty() || path.contains("..") {
        bail!("invalid registry key: {raw}")
    }
    Ok(RegistryKey { root, path, view })
}

/// Splits a key that cannot name a view.
///
/// The uninstall registration and the autostart entry are written by one process
/// and removed by another, and what travels between them is the hive and the
/// subkey alone, so a view here would be recorded and then forgotten.
pub(super) fn registry_path(raw: &str) -> Result<(HKEY, String)> {
    let key = parse_registry_key(raw)?;
    if !key.view.is_native() {
        bail!(
            "{raw} names a registry view, which only a project script's own key and a dependency's detection rule can use"
        );
    }
    Ok((key.root, key.path))
}

pub(super) fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(super) fn register_uninstaller(
    root: windows::Win32::System::Registry::HKEY,
    path: &str,
    destination: &Path,
    uninstaller_name: &str,
    config: &Value,
    upgrade: bool,
    files: &[PathBuf],
) -> Result<()> {
    write_uninstall_registration(
        root,
        path,
        destination,
        uninstaller_name,
        config,
        upgrade,
        files,
    )
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
    files: &[PathBuf],
) -> Result<()> {
    write_uninstall_registration(
        root,
        path,
        destination,
        uninstaller_name,
        config,
        true,
        files,
    )
}

/// One value of the uninstall entry, with the type Windows reads it as.
///
/// Windows lists a product under installed programs and offers the actions the
/// entry claims, so the size and the two flags that stand beside the strings are
/// numbers rather than text.
enum UninstallField {
    Text(String),
    Dword(u32),
}

/// The size of an installation, in KiB, which is how Windows shows it.
///
/// Every file the installation owns is counted, the uninstaller included: what
/// the directory takes is what the number is about, and the manifest leaves the
/// uninstaller out of its own list because removing it is its own step. Windows
/// reads the value in kilobytes, so a product smaller than one reports one:
/// zero there reads as "size unknown".
///
/// The field is a `REG_DWORD`, so an installation past four tebibytes reports
/// the largest value the type can hold rather than failing the install.
fn installed_size_kib(destination: &Path, files: &[PathBuf], uninstaller_name: &str) -> u32 {
    let bytes = files
        .iter()
        .map(|relative| destination.join(relative))
        .chain(std::iter::once(destination.join(uninstaller_name)))
        .filter_map(|path| fs::metadata(path).ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum::<u64>();
    u32::try_from(bytes / 1024).unwrap_or(u32::MAX).max(1)
}

fn write_uninstall_registration(
    root: windows::Win32::System::Registry::HKEY,
    path: &str,
    destination: &Path,
    uninstaller_name: &str,
    config: &Value,
    upgrade: bool,
    files: &[PathBuf],
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
    // What the entry says the product is, where it went, and how to remove it
    // without a window. It is the set a Windows installation list expects, and
    // the two flags are what keep it from offering actions this installer does
    // not have: there is no separate repair or modify step, and a button that
    // leads nowhere is worse than no button.
    let fields = [
        (
            "DisplayName",
            UninstallField::Text(
                config["project"]["name"]
                    .as_str()
                    .unwrap_or("nano-installer")
                    .to_string(),
            ),
        ),
        (
            "DisplayVersion",
            UninstallField::Text(
                config["project"]["version"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            ),
        ),
        (
            "Publisher",
            UninstallField::Text(
                config["project"]["publisher"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            ),
        ),
        (
            "InstallLocation",
            UninstallField::Text(destination.display().to_string()),
        ),
        (
            "UninstallString",
            UninstallField::Text(format!("\"{}\"", uninstaller.display())),
        ),
        (
            // The one Windows runs when a script or an administrator asks for
            // the removal without a window.
            "QuietUninstallString",
            UninstallField::Text(format!(
                "\"{}\" {}",
                uninstaller.display(),
                super::SILENT_FLAG
            )),
        ),
        (
            "DisplayIcon",
            UninstallField::Text(uninstaller.display().to_string()),
        ),
        (
            "EstimatedSize",
            UninstallField::Dword(installed_size_kib(destination, files, uninstaller_name)),
        ),
        ("NoModify", UninstallField::Dword(1)),
        ("NoRepair", UninstallField::Dword(1)),
    ];
    let result = fields
        .into_iter()
        .try_for_each(|(name, field)| -> Result<()> {
            match field {
                UninstallField::Text(value) => {
                    let encoded = wide(&value);
                    let bytes = unsafe {
                        std::slice::from_raw_parts(encoded.as_ptr().cast::<u8>(), encoded.len() * 2)
                    };
                    unsafe {
                        RegSetValueExW(key, PCWSTR(wide(name).as_ptr()), 0, REG_SZ, Some(bytes))
                            .ok()?;
                    }
                }
                UninstallField::Dword(value) => {
                    let bytes = value.to_le_bytes();
                    unsafe {
                        RegSetValueExW(
                            key,
                            PCWSTR(wide(name).as_ptr()),
                            0,
                            REG_DWORD,
                            Some(&bytes),
                        )
                        .ok()?;
                    }
                }
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

fn uninstall(uninstaller: &Path, keep_data: bool, task: &Cancellation) -> Result<()> {
    let destination = uninstaller
        .parent()
        .context("uninstaller has no parent directory")?;
    validate_destination(destination)?;
    let manifest_file = destination.join(MANIFEST_NAME);
    install_log::note("info", "reading the installation's manifest");
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
    describe_run(&config, destination, "remove from");
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
    let expected = uninstall_registry_key(&config)?;
    if manifest_root != registry_root_name(expected.root) || manifest_path != expected.path {
        bail!("installation manifest registry target does not match the project")
    }
    let exe_name = config["install"]["exe_name"]
        .as_str()
        .context("install.exe_name is required")?;
    // Nothing has been removed yet, so this is the last moment an uninstall
    // can be called off: once the removals start they run to their end, because
    // a product left half-removed is worse than one removed a second time.
    check_cancelled(task)?;
    if bundle.contains(script::UNINSTALL_SCRIPT) {
        let stage = StagingDirectory::create()?;
        install_log::note("info", "running the project's own uninstall script");
        return script::run_uninstall(script::UninstallRequest {
            uninstaller: uninstaller.to_path_buf(),
            bundle,
            config,
            destination: destination.to_path_buf(),
            manifest,
            keep_data,
            stage: stage.0.clone(),
            root: expected.root,
            registry_path: expected.path.clone(),
            cancel: task.clone(),
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
    install_log::note(
        "info",
        "removing the entries the installation left outside itself",
    );
    remove_recorded_artifacts(&manifest);
    if !keep_data {
        super::report_progress(60, "uninstall.status.removing_user_data")?;
        remove_user_data(&config)?;
    }
    super::report_progress(75, "uninstall.status.removing_files")?;
    install_log::note("info", "removing the installed files");
    remove_installed_files(destination, &manifest)?;
    super::report_progress(90, "uninstall.status.finishing")?;
    finish_uninstall(destination, uninstaller, expected.root, &expected.path)?;
    shell::notify_shell();
    super::report_progress(100, "uninstall.status.complete")?;
    install_log::note("info", "the product is removed");
    Ok(())
}

/// Removes the shortcut, autostart, service, and registry entries the manifest
/// records.
pub(super) fn remove_recorded_artifacts(manifest: &Value) {
    // Services go first: each one runs a program inside the installation, and a
    // service left behind while its program is deleted starts into nothing.
    remove_recorded_services(manifest);
    InstallArtifacts::remove_recorded(manifest);
    remove_recorded_registry(manifest);
}

/// Deletes the services a project script installed.
///
/// A service the script already deleted, or one an earlier uninstall took away,
/// is not an error: the manifest records what the installation owned, and the
/// uninstall deletes whatever of it is still there.
fn remove_recorded_services(manifest: &Value) {
    let Some(services) = manifest["services"].as_array() else {
        return;
    };
    for name in services.iter().filter_map(Value::as_str) {
        let _ = crate::service::delete(name);
    }
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
    use std::process::{Command, Stdio};
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
        // The helper has nothing to say: it runs after the uninstall is over and
        // reports no error of its own, so inherited handles would only put its
        // output in front of whoever started the uninstall.
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
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
        begin_deployment, check_cancelled, deploy_files, installed_size_kib, parse_registry_key,
        parse_silent_arguments, preserved_data_paths, previous_install, register_uninstaller,
        registry_path, require_free_space, require_silent_support, resolve_install_destination,
        result_notice, same_contents, selected_components, validate_destination, wide,
        Cancellation, Cancelled, Deployment, InstallArtifacts, PayloadFiles, PreviousInstall,
        RegistryKey, RegistryView, SilentOptions, MANIFEST_NAME,
    };
    use anyhow::Result;
    use std::path::{Path, PathBuf};
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegDeleteKeyW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    struct TestRegistryKey(String);

    impl Drop for TestRegistryKey {
        fn drop(&mut self) {
            unsafe {
                let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(wide(&self.0).as_ptr()));
            }
        }
    }

    /// A key may name the view it is read in, and that view is what reaches the
    /// registry call.
    ///
    /// The proof is the one name whose two copies differ: a 64-bit Windows keeps
    /// the 32-bit view of `HKLM\SOFTWARE` under `WOW6432Node`, so the name is
    /// there in the 64-bit view and nowhere in the 32-bit one. A 32-bit Windows
    /// has one view of the registry, and this case asks it for nothing.
    #[test]
    fn a_registry_key_names_the_view_it_is_read_in() -> Result<()> {
        let key = parse_registry_key(r"HKLM64\SOFTWARE\Microsoft")?;
        assert_eq!(key.root, HKEY_LOCAL_MACHINE);
        assert_eq!(key.path, r"SOFTWARE\Microsoft");
        assert_eq!(key.view, RegistryView::Wow64);

        let key = parse_registry_key(r"HKCU32\Software\Classes")?;
        assert_eq!(key.root, HKEY_CURRENT_USER);
        assert_eq!(key.view, RegistryView::Wow6432);

        let plain = parse_registry_key(r"HKCU\Software")?;
        assert_eq!(plain.view, RegistryView::Native);
        // A suffix that is neither view is not a hive, and a key that travels
        // between two processes as a hive and a subkey cannot carry one at all.
        assert!(parse_registry_key(r"HKLM65\SOFTWARE").is_err());
        assert!(registry_path(r"HKLM64\SOFTWARE").is_err());

        // Where the machine keeps a branch only a 32-bit product registered,
        // the two views are told apart by it: the name is in the copy a 32-bit
        // program reads and in no other. A machine that keeps no such product,
        // or a 32-bit Windows, has one view and nothing to tell apart.
        if let Some(marker) = a_key_only_the_32_bit_view_has() {
            let thirty_two = parse_registry_key(&format!(r"HKLM32\{marker}"))?;
            let sixty_four = parse_registry_key(&format!(r"HKLM64\{marker}"))?;
            assert!(thirty_two.exists()?, "the 32-bit view was not reached");
            assert!(
                !sixty_four.exists()?,
                "the 32-bit view read the 64-bit copy of {marker}"
            );
        }

        // A key written in a named view reads back through the same name. Both
        // views of `HKCU\Software` are the same place -- Windows redirects only
        // some of the branch -- so this holds the write and the read together
        // rather than telling the two copies apart.
        let name = format!("nano-installer-view-test-{}", std::process::id());
        let _guard = TestRegistryKey(format!(r"Software\{name}"));
        let key = parse_registry_key(&format!(r"HKCU64\Software\{name}"))?;
        key.write_string("View", "64")?;
        assert_eq!(key.read_string("View")?, Some("64".to_string()));
        key.delete_key()?;
        assert!(!key.exists()?);
        Ok(())
    }

    /// A branch below `HKLM\SOFTWARE` that only the 32-bit view of it has,
    /// where the machine keeps one.
    ///
    /// A 64-bit Windows keeps the whole branch a 32-bit program reads as a copy
    /// of `HKLM\SOFTWARE`, and what is in one copy and not the other is what a
    /// 32-bit product left there. Which product that is depends on the machine,
    /// so the names are tried rather than assumed.
    fn a_key_only_the_32_bit_view_has() -> Option<&'static str> {
        const CANDIDATES: [&str; 4] = [
            r"SOFTWARE\Microsoft\EdgeUpdate",
            r"SOFTWARE\Microsoft\EdgeWebView",
            r"SOFTWARE\Google\Update",
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
        ];
        CANDIDATES.into_iter().find(|marker| {
            let thirty_two = parse_registry_key(&format!(r"HKLM32\{marker}"));
            let sixty_four = parse_registry_key(&format!(r"HKLM64\{marker}"));
            match (thirty_two, sixty_four) {
                (Ok(thirty_two), Ok(sixty_four)) => {
                    thirty_two.exists().unwrap_or(false) && !sixty_four.exists().unwrap_or(true)
                }
                _ => false,
            }
        })
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

    /// What a full setup deploys: every file it unpacks, nothing left on disk.
    fn full(files: &[PathBuf]) -> PayloadFiles {
        PayloadFiles {
            deployed: files.to_vec(),
            kept: Vec::new(),
            update: false,
        }
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
        let task = Cancellation::default();
        let payload = full(files);
        Deployment {
            extracted,
            destination,
            files: &payload,
            uninstaller_name: "uninst.exe",
            uninstaller: b"uninstaller",
            root: HKEY_CURRENT_USER,
            registry_path: "Software\\nano-installer-test",
            previous,
            backup_root,
            artifacts: &artifacts,
            registry_values: &[],
            registry_keys: &[],
            services: &[],
            task: &task,
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
    fn a_cancel_request_stops_the_checkpoints_that_follow_it() {
        let task = Cancellation::default();
        assert!(!task.requested());
        assert!(check_cancelled(&task).is_ok());

        task.request();
        assert!(task.requested());
        let error = check_cancelled(&task).expect_err("the checkpoint gives up");
        assert!(error.is::<Cancelled>(), "{error:#}");
        assert_eq!(format!("{error}"), "cancelled by the user");

        // Stopping one task does not touch the next one: every task gets a
        // handle of its own.
        assert!(check_cancelled(&Cancellation::default()).is_ok());
    }

    #[test]
    fn a_cancelled_deployment_writes_nothing() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let extracted = temp.path().join("extracted");
        let files = write_payload(&extracted, &[("TapTap.exe", b"app")])?;
        let destination = temp.path().join("installed");
        let mut journal = begin_deployment(&destination, &temp.path().join("rollback"), None)?;
        let task = Cancellation::default();
        task.request();

        assert!(
            deploy_files(
                &extracted,
                &destination,
                &full(&files),
                None,
                &mut journal,
                &task
            )
            .is_err(),
            "a cancelled deployment copied files anyway"
        );
        journal.rollback();
        assert!(
            !destination.exists(),
            "the directory a cancelled install created is still there"
        );
        Ok(())
    }

    /// An update deploys over a target that already holds exactly its bytes by
    /// not touching it: a read-only file is the proof, because a copy onto one
    /// fails.
    #[test]
    fn an_update_leaves_a_target_that_already_holds_the_bytes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let extracted = temp.path().join("extracted");
        let files = write_payload(&extracted, &[("E2eProbe.exe", b"second release")])?;
        let destination = temp.path().join("installed");
        let mut journal = begin_deployment(&destination, &temp.path().join("rollback"), None)?;
        // The machine already holds the bytes this payload carries, and the file
        // is read-only: a copy onto it fails, so coming through the deployment
        // untouched is what says the file was left alone.
        let target = destination.join("E2eProbe.exe");
        std::fs::write(&target, b"second release")?;
        let mut permissions = std::fs::metadata(&target)?.permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&target, permissions.clone())?;

        let payload = PayloadFiles {
            deployed: files,
            kept: Vec::new(),
            update: true,
        };
        let task = Cancellation::default();
        let deployed = deploy_files(
            &extracted,
            &destination,
            &payload,
            None,
            &mut journal,
            &task,
        );
        // This crate is built for Windows only, where the read-only attribute is
        // what the deployment has to leave alone rather than write through.
        #[allow(clippy::permissions_set_readonly_false)]
        {
            permissions.set_readonly(false);
            std::fs::set_permissions(&target, permissions)?;
        }
        deployed?;
        assert_eq!(std::fs::read(&target)?, b"second release");

        // Bytes that differ are written, which is what makes the skip above a
        // comparison rather than something every update does.
        std::fs::write(&target, b"first release")?;
        deploy_files(
            &extracted,
            &destination,
            &payload,
            None,
            &mut journal,
            &task,
        )?;
        assert_eq!(std::fs::read(&target)?, b"second release");
        Ok(())
    }

    /// Two files are compared by their bytes, in chunks, however large they are.
    #[test]
    fn files_are_compared_by_their_bytes_in_chunks() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let left = temp.path().join("left.bin");
        let right = temp.path().join("right.bin");
        // Larger than one chunk, so the comparison has to walk both files.
        let large = (0..200_000u32)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        std::fs::write(&left, &large)?;
        std::fs::write(&right, &large)?;
        assert!(same_contents(&left, &right)?);

        // The last byte is enough to tell them apart.
        let mut other = large.clone();
        other[199_999] = other[199_999].wrapping_add(1);
        std::fs::write(&right, &other)?;
        assert!(!same_contents(&left, &right)?);

        // A size that differs is answered without reading either file.
        std::fs::write(&right, b"short")?;
        assert!(!same_contents(&left, &right)?);

        // A target that is not there does not hold the bytes.
        assert!(!same_contents(&left, &temp.path().join("missing.bin"))?);
        Ok(())
    }

    #[test]
    fn refuses_an_install_when_the_drive_holds_less_space_than_the_project_asks_for() -> Result<()>
    {
        let temp = tempfile::tempdir()?;
        // No drive on this machine holds an exabyte, so the ask can only be refused.
        let error = require_free_space(temp.path(), 1_000_000_000_000)
            .expect_err("a drive this small cannot satisfy the ask");
        assert!(
            format!("{error:#}").contains("not enough disk space"),
            "{error:#}"
        );
        require_free_space(temp.path(), 0)?;
        Ok(())
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
        assert!(!RegistryKey::new(HKEY_CURRENT_USER, &key.0).exists()?);
        register_uninstaller(
            HKEY_CURRENT_USER,
            &key.0,
            temp.path(),
            "uninst.exe",
            &config,
            false,
            &[],
        )?;
        assert!(RegistryKey::new(HKEY_CURRENT_USER, &key.0).exists()?);
        assert!(register_uninstaller(
            HKEY_CURRENT_USER,
            &key.0,
            temp.path(),
            "uninst.exe",
            &config,
            false,
            &[],
        )
        .is_err());
        register_uninstaller(
            HKEY_CURRENT_USER,
            &key.0,
            temp.path(),
            "uninst.exe",
            &config,
            true,
            &[],
        )?;
        Ok(())
    }

    /// Windows shows the size of an installation from the bytes on disk, and a
    /// product smaller than a kilobyte still has to report something: zero
    /// there means "unknown", which is why the result floors at one.
    #[test]
    fn the_reported_size_counts_every_owned_file() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let destination = temp.path();
        std::fs::create_dir_all(destination.join("bin"))?;
        std::fs::write(destination.join("bin/app.exe"), vec![0u8; 3 * 1024])?;
        std::fs::write(destination.join("readme.txt"), b"short")?;
        // Not a file, so it is not counted even when it is named.
        std::fs::create_dir_all(destination.join("empty"))?;
        let files = vec![PathBuf::from("bin/app.exe"), PathBuf::from("readme.txt")];
        assert_eq!(installed_size_kib(destination, &files, "uninst.exe"), 3);
        std::fs::write(destination.join("uninst.exe"), vec![0u8; 2 * 1024])?;
        assert_eq!(installed_size_kib(destination, &files, "uninst.exe"), 5);
        assert_eq!(
            installed_size_kib(destination, &[PathBuf::from("empty")], "absent.exe"),
            1
        );
        Ok(())
    }

    /// A configured default path names environment variables, and an
    /// unexpanded `%LOCALAPPDATA%\\Product` is not absolute, so an install that
    /// skipped this step would be refused before it wrote anything.
    #[test]
    fn configured_install_paths_are_expanded() -> Result<()> {
        let config = serde_json::json!({
            "install": {"default_path": "%LOCALAPPDATA%\\nano-installer-path-test"}
        });
        let resolved = resolve_install_destination(&config, None)?;
        assert!(resolved.is_absolute(), "{resolved:?} is not absolute");
        assert!(
            !resolved.to_string_lossy().contains('%'),
            "{resolved:?} still holds an unexpanded variable"
        );
        assert_eq!(
            resolved.file_name().and_then(|name| name.to_str()),
            Some("nano-installer-path-test")
        );
        Ok(())
    }

    /// The page the user answered decides which components a run installs. A box
    /// the page carries answers for the component of that id; a component the page
    /// carries no box for keeps the project's default; a required component is
    /// installed whatever the page says.
    #[test]
    fn the_page_and_the_project_decide_which_components_install() {
        let config = serde_json::json!({
            "components": { "items": [
                { "id": "core", "payload": "payload/core.7z", "required": true },
                { "id": "docs", "payload": "payload/docs.7z" },
                { "id": "samples", "payload": "payload/samples.7z", "default": true }
            ] }
        });
        // The page answered for docs alone, so docs installs and samples keeps the
        // default the project gave it.
        let ticked = selected_components(&config, |id, default| id == "docs" || default);
        assert_eq!(ticked, ["core", "docs", "samples"]);
        // A silent run has no page, which is the same as a page that carries none of
        // these boxes: the project decides, except where it said required.
        let silent = selected_components(&config, |_, default| default);
        assert_eq!(silent, ["core", "samples"]);
        // The user can clear a default the project chose, and cannot clear a required
        // component.
        let cleared = selected_components(&config, |_, _| false);
        assert_eq!(cleared, ["core"]);
    }

    /// A project that declares no components installs its payload and nothing else,
    /// however the page answers.
    #[test]
    fn a_project_without_components_installs_none_of_them() {
        let config = serde_json::json!({ "resources": { "payload_file": "payload/app.7z" } });
        assert!(selected_components(&config, |_, _| true).is_empty());
    }

    /// An explicit directory wins over the configured one, which is what lets a
    /// silent run choose where a product lands without editing the project.
    #[test]
    fn an_explicit_install_path_wins_over_the_configured_one() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let chosen = temp.path().join("chosen");
        let config = serde_json::json!({
            "install": {"default_path": "C:\\Somewhere\\Else"}
        });
        let resolved = resolve_install_destination(&config, Some(&chosen))?;
        assert_eq!(resolved, chosen);
        Ok(())
    }

    /// A failure the wizard reports names the file the run was written to.
    ///
    /// This is the notice that stands between a user whose install failed and
    /// whoever has to explain why: the error says what went wrong, and the line
    /// under it is the one thing the user can send on. A run that succeeded is
    /// never told about a log, because there is nothing to report.
    #[test]
    fn a_failure_notice_points_at_the_log_of_the_run() {
        let notice = result_notice(
            Err(anyhow::anyhow!("cannot write files")),
            Some("A log of this run is at:\nC:\\Users\\u\\Temp\\setup.log"),
        );
        assert!(
            notice.starts_with("Operation failed:\ncannot write files"),
            "{notice}"
        );
        assert!(notice.contains("C:\\Users\\u\\Temp\\setup.log"), "{notice}");

        // Without a log there is nothing to name, and the notice says only what
        // went wrong.
        let notice = result_notice(Err(anyhow::anyhow!("cannot write files")), None);
        assert!(!notice.contains("log"), "{notice}");

        assert_eq!(
            result_notice(Ok(()), Some("A log of this run is at:\nX")),
            "Operation complete"
        );
    }

    /// Neither source is a directory to guess at, so the run stops with a
    /// message that names both ways to supply one.
    #[test]
    fn a_run_without_any_install_path_is_refused() {
        let config = serde_json::json!({"install": {}});
        let error = resolve_install_destination(&config, None).expect_err("no directory");
        let text = format!("{error:#}");
        assert!(
            text.contains("--dir"),
            "the error should name --dir: {text}"
        );
        assert!(
            text.contains("install.default_path"),
            "the error should name the config key: {text}"
        );
    }

    #[test]
    fn silent_arguments_read_the_directory_and_reject_anything_else() -> Result<()> {
        use std::ffi::OsString;

        let empty: Vec<OsString> = Vec::new();
        assert_eq!(parse_silent_arguments(&empty)?, SilentOptions::default());

        let given = vec![OsString::from("--dir"), OsString::from("C:\\Install\\Here")];
        assert_eq!(
            parse_silent_arguments(&given)?.directory,
            Some(PathBuf::from("C:\\Install\\Here"))
        );

        // A windowless run has no notice to read, so it can be told where to
        // leave the record of itself. Both flags may appear in one command line,
        // in either order.
        let logged = vec![
            OsString::from("--log"),
            OsString::from("C:\\Logs\\setup.log"),
            OsString::from("--dir"),
            OsString::from("C:\\Install\\Here"),
        ];
        let options = parse_silent_arguments(&logged)?;
        assert_eq!(options.log, Some(PathBuf::from("C:\\Logs\\setup.log")));
        assert_eq!(options.directory, Some(PathBuf::from("C:\\Install\\Here")));
        assert_eq!(
            parse_silent_arguments(&logged[2..])?.log,
            None,
            "a --dir run keeps the default log"
        );

        // A mistyped option has to stop the run: quietly installing into the
        // configured default instead would put files somewhere nobody asked for.
        let unknown = vec![OsString::from("--silnet")];
        assert!(parse_silent_arguments(&unknown).is_err());

        // So do --dir and --log with nothing after them.
        for dangling in [vec![OsString::from("--dir")], vec![OsString::from("--log")]] {
            assert!(parse_silent_arguments(&dangling).is_err());
        }
        Ok(())
    }

    /// A windowless run is the project's own decision, so a project that never
    /// declared one cannot be installed or removed unattended.
    #[test]
    fn a_project_that_did_not_opt_in_refuses_a_windowless_run() {
        let opted_in = serde_json::json!({"advanced": {"silent_mode_support": true}});
        assert!(require_silent_support(&opted_in, "silent_mode_support").is_ok());

        for config in [
            serde_json::json!({}),
            serde_json::json!({"advanced": {}}),
            serde_json::json!({"advanced": {"silent_mode_support": false}}),
        ] {
            let error = require_silent_support(&config, "silent_mode_support")
                .expect_err("the project did not opt in");
            assert!(
                format!("{error:#}").contains("silent_mode_support"),
                "the error should name the switch to set"
            );
        }
    }
}
