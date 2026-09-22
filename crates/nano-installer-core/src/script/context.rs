//! State the Rhai primitives and the script driver share.
//!
//! A project script decides the step order, so it also decides which files,
//! shortcuts, and registry entries the installation owns. Everything it
//! creates is recorded here while it runs; the driver turns that record into
//! the manifest the uninstaller replays, or into a rollback when it fails.

use anyhow::{bail, Result};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use super::Mode;
use crate::install::{parse_registry_key, Cancellation, PreviousInstall, RollbackJournal};
use crate::BundleIndex;

/// The paths below one root, kept relative so an installation stays relocatable.
#[derive(Default)]
pub(super) struct Snapshot {
    files: BTreeSet<PathBuf>,
    directories: BTreeSet<PathBuf>,
}

impl Snapshot {
    /// Walks `root`. A root that does not exist yet is an empty snapshot.
    pub(super) fn take(root: &Path) -> Self {
        let mut snapshot = Self::default();
        walk(root, root, &mut snapshot);
        snapshot
    }

    /// Files that did not exist when `earlier` was taken.
    pub(super) fn files_added_since(&self, earlier: &Self) -> Vec<PathBuf> {
        self.files.difference(&earlier.files).cloned().collect()
    }

    /// Directories that did not exist when `earlier` was taken.
    fn directories_added_since(&self, earlier: &Self) -> Vec<PathBuf> {
        self.directories
            .difference(&earlier.directories)
            .cloned()
            .collect()
    }
}

fn walk(root: &Path, current: &Path, snapshot: &mut Snapshot) {
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => {
                snapshot.directories.insert(relative.to_path_buf());
                walk(root, &path, snapshot);
            }
            Ok(_) => {
                snapshot.files.insert(relative.to_path_buf());
            }
            Err(_) => {}
        }
    }
}

/// What the running script has changed so far.
pub(super) struct ScriptState {
    /// Every path a script overwrote or created, so a failure can be undone.
    pub(super) journal: RollbackJournal,
    /// The destination as it looked before the script ran.
    pub(super) before: Snapshot,
    pub(super) shortcuts: Vec<PathBuf>,
    pub(super) shortcut_dirs: Vec<PathBuf>,
    pub(super) registry_values: Vec<(String, String)>,
    pub(super) registry_keys: Vec<String>,
    /// Services a script installed, by the name the machine knows them by.
    pub(super) services: Vec<String>,
    /// Files an update package verified on the machine and left where they
    /// were, which the installation still owns.
    pub(super) kept: Vec<PathBuf>,
    /// Set once the script replayed the manifest removal itself.
    pub(super) tracked_uninstall: bool,
    /// The directory the bundled tools were unpacked into, once a script asked.
    pub(super) tools: Option<PathBuf>,
}

struct Inner {
    mode: Mode,
    /// The setup or uninstaller image the script runs inside; its bundle
    /// supplies the payload and the uninstaller to deploy.
    setup: PathBuf,
    /// Scratch directory for streamed payload archives.
    stage: PathBuf,
    config: Value,
    install_path: PathBuf,
    checkboxes: HashMap<String, bool>,
    /// What the page's text fields held, by the id the layout gave each one.
    texts: HashMap<String, String>,
    /// What the page's choice controls held, by the id that owns the choice.
    choices: HashMap<String, String>,
    /// The components this run installs; empty while uninstalling.
    components: Vec<String>,
    keep_data: bool,
    /// The manifest the uninstaller replays; `Null` while installing.
    manifest: Value,
    /// The installation this run replaces, when it is an upgrade.
    previous: Option<PreviousInstall>,
    bundle: BundleIndex,
    /// Set once the user has asked this task to stop.
    cancel: Cancellation,
}

/// Handle the Rhai primitives share with the driver.
#[derive(Clone)]
pub(super) struct ScriptContext {
    inner: Arc<Inner>,
    state: Arc<Mutex<ScriptState>>,
}

/// Everything the driver hands over before a script can run.
pub(super) struct ScriptEnvironment {
    pub(super) mode: Mode,
    pub(super) setup: PathBuf,
    pub(super) config: Value,
    pub(super) install_path: PathBuf,
    pub(super) checkboxes: HashMap<String, bool>,
    /// The values the page held when the user started the task, which a script
    /// reads through `get_text_value` and `get_choice_value`.
    pub(super) texts: HashMap<String, String>,
    pub(super) choices: HashMap<String, String>,
    /// The components this run installs, which a script reads through
    /// `is_component_selected` and `selected_components`.
    pub(super) components: Vec<String>,
    pub(super) keep_data: bool,
    pub(super) manifest: Value,
    pub(super) previous: Option<PreviousInstall>,
    pub(super) bundle: BundleIndex,
    /// Scratch directory the caller owns for the duration of the script.
    pub(super) stage: PathBuf,
    /// The task this script is part of, so it can be asked to stop.
    pub(super) cancel: Cancellation,
    /// Filled in by `super::begin`, which creates the destination.
    pub(super) journal: Option<RollbackJournal>,
}

/// Everything a page hook can read.
///
/// A hook runs between two pages rather than as part of a task, so it is given
/// the values the wizard holds and nothing that belongs to an installation: no
/// previous version, no manifest, and no cancellation to ask about.
pub(crate) struct PageEnvironment {
    pub(crate) mode: Mode,
    pub(crate) config: Value,
    pub(crate) install_path: PathBuf,
    pub(crate) checkboxes: HashMap<String, bool>,
    pub(crate) texts: HashMap<String, String>,
    pub(crate) choices: HashMap<String, String>,
    /// The components the wizard's current answers say a run would install.
    pub(crate) components: Vec<String>,
}

impl ScriptContext {
    pub(super) fn new(environment: ScriptEnvironment) -> Self {
        let journal = environment
            .journal
            .expect("the script driver creates the rollback journal first");
        let inner = Inner {
            mode: environment.mode,
            setup: environment.setup,
            stage: environment.stage,
            config: environment.config,
            install_path: environment.install_path,
            checkboxes: environment.checkboxes,
            texts: environment.texts,
            choices: environment.choices,
            components: environment.components,
            keep_data: environment.keep_data,
            manifest: environment.manifest,
            previous: environment.previous,
            bundle: environment.bundle,
            cancel: environment.cancel,
        };
        Self::assemble(inner, journal)
    }

    /// The context a page hook runs in.
    ///
    /// Only the primitives that read are registered against it, so the journal,
    /// the staging directory, the bundle, and the cancellation below are never
    /// reached: they are here because every primitive shares one context type,
    /// and a hook that changed the machine would be an installation step nobody
    /// declared.
    pub(super) fn for_page(environment: PageEnvironment) -> Self {
        let inner = Inner {
            mode: environment.mode,
            setup: std::env::current_exe().unwrap_or_default(),
            stage: std::env::temp_dir(),
            config: environment.config,
            install_path: environment.install_path,
            checkboxes: environment.checkboxes,
            texts: environment.texts,
            choices: environment.choices,
            components: environment.components,
            keep_data: false,
            manifest: Value::Null,
            previous: None,
            bundle: crate::BundleIndex::empty(),
            cancel: Cancellation::default(),
        };
        let journal =
            RollbackJournal::new(std::env::temp_dir().join("nano-installer-page-hook"), None);
        Self::assemble(inner, journal)
    }

    fn assemble(inner: Inner, journal: RollbackJournal) -> Self {
        Self {
            inner: Arc::new(inner),
            state: Arc::new(Mutex::new(ScriptState {
                journal,
                before: Snapshot::default(),
                shortcuts: Vec::new(),
                shortcut_dirs: Vec::new(),
                registry_values: Vec::new(),
                registry_keys: Vec::new(),
                services: Vec::new(),
                kept: Vec::new(),
                tracked_uninstall: false,
                tools: None,
            })),
        }
    }

    /// Records the destination as it looked before the script ran.
    pub(super) fn set_before(&self, before: Snapshot) {
        self.state().before = before;
    }

    pub(super) fn mode(&self) -> Mode {
        self.inner.mode
    }

    pub(super) fn setup(&self) -> &Path {
        &self.inner.setup
    }

    pub(super) fn stage(&self) -> &Path {
        &self.inner.stage
    }

    pub(super) fn config(&self) -> &Value {
        &self.inner.config
    }

    pub(super) fn bundle(&self) -> &BundleIndex {
        &self.inner.bundle
    }

    /// Where the bundled tools were unpacked, once a script asked for them.
    /// Unpacking again would only repeat the writes to the same place.
    pub(super) fn tools_directory(&self) -> Option<PathBuf> {
        self.state().tools.clone()
    }

    pub(super) fn set_tools_directory(&self, directory: PathBuf) {
        self.state().tools = Some(directory);
    }

    /// The task this script is part of, so `is_cancelled` can ask it.
    pub(super) fn cancellation(&self) -> &Cancellation {
        &self.inner.cancel
    }

    pub(super) fn install_path(&self) -> PathBuf {
        self.inner.install_path.clone()
    }

    pub(super) fn install_path_text(&self) -> String {
        self.inner.install_path.to_string_lossy().to_string()
    }

    pub(super) fn keep_data(&self) -> bool {
        self.inner.keep_data
    }

    pub(super) fn manifest(&self) -> &Value {
        &self.inner.manifest
    }

    /// The installation this run replaces, when it is an upgrade.
    pub(super) fn previous(&self) -> Option<&PreviousInstall> {
        self.inner.previous.as_ref()
    }

    /// Looks up a dotted `config.json` path, as `get_config_value` exposes it.
    pub(super) fn config_value(&self, path: &str) -> Option<&Value> {
        let mut current = &self.inner.config;
        for key in path.split('.') {
            current = current.get(key)?;
        }
        Some(current)
    }

    pub(super) fn checkbox(&self, id: &str) -> bool {
        self.inner.checkboxes.get(id).copied().unwrap_or(false)
    }

    /// What the page's field `id` held. A layout that declares no such field
    /// reads as empty text, so a script can ask for a value the page it runs on
    /// does not carry.
    pub(super) fn text_value(&self, id: &str) -> String {
        self.inner.texts.get(id).cloned().unwrap_or_default()
    }

    /// What the page's choice `id` held, empty when the page offers no such
    /// choice.
    pub(super) fn choice_value(&self, id: &str) -> String {
        self.inner.choices.get(id).cloned().unwrap_or_default()
    }

    /// The components this run installs, in the order the project declares them.
    /// An uninstall installs nothing, so the list is empty there.
    pub(super) fn components(&self) -> &[String] {
        &self.inner.components
    }

    /// Whether the component `id` is one of them.
    pub(super) fn component_selected(&self, id: &str) -> bool {
        self.inner.components.iter().any(|chosen| chosen == id)
    }

    /// Recovers the state even if an earlier script panicked while holding it;
    /// leaving the installer stuck on a poisoned lock would be worse.
    pub(super) fn state(&self) -> MutexGuard<'_, ScriptState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub(super) fn progress(&self, percent: f64) {
        let percent = percent.clamp(0.0, 100.0) as u8;
        if let Err(error) = crate::publish_progress(percent) {
            log("warn", &format!("cannot publish progress: {error:#}"));
        }
    }

    pub(super) fn status(&self, text: &str) {
        if let Err(error) = crate::publish_status_text(text) {
            log("warn", &format!("cannot publish status: {error:#}"));
        }
    }

    pub(super) fn status_key(&self, key: &str) {
        if let Err(error) = crate::publish_status_key(key) {
            log("warn", &format!("cannot publish status: {error:#}"));
        }
    }

    /// Records `target` before a script creates or overwrites it.
    pub(super) fn record_write(&self, target: &Path) -> Result<()> {
        self.state().journal.track(target)
    }

    /// Records the files an update package verified and left where they were.
    ///
    /// They are named in the manifest like the ones the run deployed: an
    /// uninstall that did not know about them would leave them behind.
    pub(super) fn record_kept(&self, paths: &[PathBuf]) {
        let mut state = self.state();
        for path in paths {
            if !state.kept.iter().any(|recorded| recorded == path) {
                state.kept.push(path.clone());
            }
        }
    }

    pub(super) fn record_shortcut(&self, link: &Path) {
        let mut state = self.state();
        if !state.shortcuts.iter().any(|recorded| recorded == link) {
            state.shortcuts.push(link.to_path_buf());
        }
    }

    pub(super) fn record_shortcut_dir(&self, directory: &Path) {
        let mut state = self.state();
        if !state
            .shortcut_dirs
            .iter()
            .any(|recorded| recorded == directory)
        {
            state.shortcut_dirs.push(directory.to_path_buf());
        }
    }

    /// Records a registry write so the uninstaller replays it.
    ///
    /// A value written into a container key is replayed as a value deletion,
    /// because the key belongs to Windows or to another product. Anywhere else
    /// the key is this product's own, so it is removed whole.
    pub(super) fn record_registry_write(&self, key: &str, name: &str) {
        let mut state = self.state();
        if !state
            .registry_values
            .iter()
            .any(|(recorded_key, recorded_name)| recorded_key == key && recorded_name == name)
        {
            state
                .registry_values
                .push((key.to_string(), name.to_string()));
        }
        if !is_shared_registry_key(key) {
            push_unique(&mut state.registry_keys, key);
        }
    }

    /// Forgets a key a script removed, so the uninstaller does not replay it.
    ///
    /// A key that now belongs to another product is forgotten without being
    /// removed: the uninstall must leave what it no longer owns alone.
    pub(super) fn forget_registry_key(&self, key: &str) {
        let mut state = self.state();
        state.registry_keys.retain(|recorded| recorded != key);
        state
            .registry_values
            .retain(|(recorded, _)| recorded.as_str() != key);
    }

    /// Records a service the installation now owns, so the uninstall takes it
    /// away again.
    ///
    /// A script that installs the same service twice -- which is what an
    /// upgrade does -- records one entry for it.
    pub(super) fn record_service(&self, name: &str) {
        let name = name.trim();
        let mut state = self.state();
        if !state.services.iter().any(|recorded| recorded == name) {
            state.services.push(name.to_string());
        }
    }

    /// Forgets a service the script deleted, so the uninstall does not replay
    /// a deletion of something that is already gone.
    pub(super) fn forget_service(&self, name: &str) {
        let name = name.trim();
        self.state()
            .services
            .retain(|recorded| recorded.as_str() != name);
    }
}

fn push_unique(keys: &mut Vec<String>, key: &str) {
    if !keys.iter().any(|recorded| recorded == key) {
        keys.push(key.to_string());
    }
}

/// Container keys that Windows and other products also write into.
///
/// A script that writes one value under such a leaf owns that value, not the
/// key, so the uninstaller must not remove the key itself.
const SHARED_REGISTRY_LEAVES: [&str; 9] = [
    "classes",
    "currentversion",
    "environment",
    "explorer",
    "microsoft",
    "policies",
    "run",
    "runonce",
    "software",
];

fn is_shared_registry_key(key: &str) -> bool {
    let leaf = key
        .rsplit('\\')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    SHARED_REGISTRY_LEAVES.contains(&leaf.as_str())
}

/// Puts the destination back the way the script found it.
///
/// Best effort throughout: a partially restored directory still beats a
/// half-written installation.
pub(super) fn undo(context: &ScriptContext) {
    let install_path = context.install_path();
    let state = context.state();
    state.journal.rollback();
    // A service runs a program inside the installation, so it goes before the
    // files it would otherwise start from.
    for service in &state.services {
        let _ = crate::service::delete(service);
    }
    for link in &state.shortcuts {
        let _ = std::fs::remove_file(link);
    }
    for directory in &state.shortcut_dirs {
        let _ = std::fs::remove_dir(directory);
    }
    for (key, name) in &state.registry_values {
        if let Ok(key) = parse_registry_key(key) {
            let _ = key.delete_value(name);
        }
    }
    for key in &state.registry_keys {
        if let Ok(key) = parse_registry_key(key) {
            let _ = key.delete_key();
        }
    }
    let after = Snapshot::take(&install_path);
    let mut created = after.directories_added_since(&state.before);
    // Deepest first, so a parent is only removed once its children are gone.
    created.sort_by_key(|directory| std::cmp::Reverse(directory.components().count()));
    for directory in created {
        let _ = std::fs::remove_dir(install_path.join(directory));
    }
}

/// Converts a `config.json` value into the shape Rhai scripts see.
pub(super) fn json_to_dynamic(value: &Value) -> rhai::Dynamic {
    match value {
        Value::Null => rhai::Dynamic::UNIT,
        Value::Bool(flag) => rhai::Dynamic::from(*flag),
        Value::Number(number) => match (number.as_i64(), number.as_f64()) {
            (Some(integer), _) => rhai::Dynamic::from(integer),
            (None, Some(float)) => rhai::Dynamic::from(float),
            (None, None) => rhai::Dynamic::UNIT,
        },
        Value::String(text) => rhai::Dynamic::from(text.clone()),
        Value::Array(items) => {
            rhai::Dynamic::from(items.iter().map(json_to_dynamic).collect::<rhai::Array>())
        }
        Value::Object(fields) => {
            let mut map = rhai::Map::new();
            for (key, item) in fields {
                map.insert(key.as_str().into(), json_to_dynamic(item));
            }
            rhai::Dynamic::from(map)
        }
    }
}

/// Refuses a deletion target that is not a real path below a drive root.
pub(super) fn checked_delete_target(path: &str) -> Result<PathBuf> {
    let target = PathBuf::from(path);
    if !target.is_absolute() {
        bail!("{path} is not an absolute path");
    }
    if target.parent().is_none() {
        bail!("refusing to delete the drive root {path}");
    }
    Ok(target)
}

/// The most recent script messages, kept so a failure can explain itself.
///
/// An installer runs without a console, so `log_info` and friends would
/// otherwise go nowhere. The tail of this buffer is attached to the error the
/// wizard reports, which is where a project author will look.
///
/// The buffer belongs to the run rather than to the process. A run happens on
/// the one thread that drives the script, and a process that runs more than one
/// of them — every test binary does, and nothing stops an application from
/// doing the same — would otherwise report whichever run logged last against a
/// failure that never wrote those lines.
const LOG_LIMIT: usize = 32;
thread_local! {
    static LOG: RefCell<VecDeque<String>> = const { RefCell::new(VecDeque::new()) };
}

/// The buffer and this function belong to the run rather than to the script, so
/// the built-in step that installs a project's dependencies writes into the
/// same log a failing script reports.
///
/// The lines also go to the run's log on disk, which is the copy that outlives
/// the process: the buffer here is never longer than the notice the wizard can
/// show, and a run that succeeds never shows one at all.
pub(crate) fn log(level: &str, message: &str) {
    LOG.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        if buffer.len() == LOG_LIMIT {
            buffer.pop_front();
        }
        buffer.push_back(format!("{level}: {message}"));
    });
    crate::install_log::note(level, message);
}

/// The script log, oldest line first, for reporting alongside a failure.
pub(super) fn log_tail() -> String {
    LOG.with(|buffer| {
        buffer
            .borrow()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// Clears the buffer, so one run never reports another run's messages.
pub(super) fn reset_log() {
    LOG.with(|buffer| buffer.borrow_mut().clear());
}

/// Reads a script entry and checks that it is text.
pub(super) fn read_script(bundle: &BundleIndex, name: &str) -> Result<String> {
    String::from_utf8(bundle.read_file(name)?)
        .map_err(|_| anyhow::anyhow!("{name} is not valid UTF-8"))
}
