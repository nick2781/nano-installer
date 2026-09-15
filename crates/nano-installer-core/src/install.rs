use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, HWND};
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_CREATED_NEW_KEY, REG_CREATE_KEY_DISPOSITION,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, PostMessageW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, WM_CLOSE,
};

use super::{BundleIndex, RuntimeMode, UI};

const MANIFEST_NAME: &str = "nano-installer-manifest.json";
static BUSY: AtomicBool = AtomicBool::new(false);
static STAGING_ID: AtomicU64 = AtomicU64::new(0);

pub(super) fn busy() -> bool {
    BUSY.load(Ordering::Acquire)
}

struct StagingDirectory(PathBuf);

impl StagingDirectory {
    fn create() -> Result<Self> {
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

pub(super) fn start_install(window: HWND) {
    let path = UI
        .get()
        .and_then(|runtime| runtime.lock().ok())
        .and_then(|state| {
            (matches!(state.mode, RuntimeMode::Installer))
                .then(|| state.interaction.text_input_values.get("editDir").cloned())
                .flatten()
        });
    let Some(path) = path else {
        show_result(
            window,
            Err(anyhow::anyhow!("installation directory is not configured")),
        );
        return;
    };
    run_worker(window, move || {
        let setup = std::env::current_exe()?;
        install_setup(&setup, Path::new(&path))
    });
}

pub(super) fn start_uninstall(window: HWND) {
    run_worker(window, move || {
        let uninstaller = std::env::current_exe()?;
        uninstall(&uninstaller)
    });
}

fn run_worker(window: HWND, work: impl FnOnce() -> Result<()> + Send + 'static) {
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    let window_handle = window.0 as usize;
    std::thread::spawn(move || {
        let window = HWND(window_handle as *mut _);
        let result = work();
        BUSY.store(false, Ordering::Release);
        let succeeded = result.is_ok();
        show_result(window, result);
        if succeeded {
            unsafe {
                let _ = PostMessageW(window, WM_CLOSE, None, None);
            }
        }
    });
}

fn show_result(window: HWND, result: Result<()>) {
    let (text, icon) = match result {
        Ok(()) => ("Operation complete".to_string(), MB_ICONINFORMATION),
        Err(error) => (format!("Operation failed:\n{error:#}"), MB_ICONERROR),
    };
    unsafe {
        let _ = MessageBoxW(
            window,
            &HSTRING::from(text),
            &HSTRING::from("nano-installer"),
            MB_OK | icon,
        );
    }
}

fn install_setup(setup: &Path, destination: &Path) -> Result<()> {
    validate_destination(destination)?;
    let bundle = BundleIndex::read(setup)?.context("installer resource bundle missing")?;
    let config = bundle.read_config()?;
    let payload_name = config["resources"]["payload_file"]
        .as_str()
        .context("resources.payload_file is required")?;
    if !bundle.contains(payload_name) {
        bail!("payload missing: {payload_name}");
    }
    let uninstaller_name = config["output"]["uninstaller_name"]
        .as_str()
        .unwrap_or("uninst.exe");
    super::validate_output_filename(uninstaller_name, "output.uninstaller_name")?;
    let uninstaller = bundle.read_file(&format!("runtime/{uninstaller_name}"))?;
    let (root, registry_path) = uninstall_registry_key(&config)?;
    // An installation of this project may already be present; that is an
    // upgrade, not an error. Anything else at the destination is left alone.
    let previous = previous_install(destination, &config)?;
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

    let stage = StagingDirectory::create()?;
    let archive = stage.0.join("payload.archive");
    let extracted = stage.0.join("files");
    let backups = stage.0.join("rollback");
    // The payload is streamed straight from the setup image; it never has to
    // fit in the process address space.
    bundle.copy_file_to(payload_name, &archive)?;
    let output = std::process::Command::new(setup)
        .arg("--extract")
        .arg(&archive)
        .arg(&extracted)
        .output()
        .context("failed to start installer extraction backend")?;
    if !output.status.success() {
        bail!(
            "payload extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let files = collect_staged_files(&extracted)?;
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
    Deployment {
        extracted: &extracted,
        destination,
        files: &files,
        uninstaller_name,
        uninstaller: &uninstaller,
        root,
        registry_path: &registry_path,
        previous: previous.as_ref(),
        backup_root: &backups,
    }
    .run(|| {
        register_uninstaller(
            root,
            &registry_path,
            destination,
            uninstaller_name,
            &config,
            upgrade,
        )
    })
}

/// A previous installation of this project found at the destination.
struct PreviousInstall {
    files: Vec<PathBuf>,
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
    }))
}

fn registry_root_name(root: windows::Win32::System::Registry::HKEY) -> &'static str {
    if root == HKEY_CURRENT_USER {
        "HKCU"
    } else {
        "HKLM"
    }
}

/// Validates the manifest file list and returns the stored relative paths.
fn manifest_file_paths(manifest: &Value) -> Result<Vec<PathBuf>> {
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
}

impl Deployment<'_> {
    /// Applies the deployment. On failure the destination is restored to its
    /// previous state: a fresh install is removed, an upgrade is rolled back
    /// from the journal.
    fn run(self, register: impl FnOnce() -> Result<()>) -> Result<()> {
        validate_destination(self.destination)?;
        if self.previous.is_some() {
            fs::create_dir_all(self.destination)?;
        } else {
            fs::create_dir_all(
                self.destination
                    .parent()
                    .context("installation directory has no parent")?,
            )?;
            fs::create_dir(self.destination).with_context(|| {
                format!(
                    "installation directory must be new: {}",
                    self.destination.display()
                )
            })?;
        }
        let fresh = self
            .previous
            .is_none()
            .then(|| self.destination.to_path_buf());
        let mut journal = RollbackJournal::new(self.backup_root.to_path_buf(), fresh);
        let result = self.deploy(&mut journal, register);
        if result.is_err() {
            journal.rollback();
        }
        result
    }

    fn deploy(
        &self,
        journal: &mut RollbackJournal,
        register: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        for relative in self.files {
            let source = self.extracted.join(relative);
            let target = self.destination.join(relative);
            fs::create_dir_all(target.parent().context("payload target has no parent")?)?;
            journal.track(&target)?;
            fs::copy(source, target)?;
        }
        let uninstaller = self.destination.join(self.uninstaller_name);
        journal.track(&uninstaller)?;
        fs::write(&uninstaller, self.uninstaller)?;

        // Drop files the previous version shipped that the new payload no
        // longer contains. Files this installer never wrote are left alone.
        if let Some(previous) = self.previous {
            for relative in &previous.files {
                if self.files.iter().any(|path| path == relative) {
                    continue;
                }
                let target = self.destination.join(relative);
                if !target.is_file() {
                    continue;
                }
                journal.track(&target)?;
                fs::remove_file(&target)?;
            }
        }

        journal.track(&self.destination.join(MANIFEST_NAME))?;
        let manifest = json!({
            "version": 1,
            "registry_root": registry_root_name(self.root),
            "registry_path": self.registry_path,
            "files": self.files.iter().map(|path| path.to_string_lossy().to_string()).collect::<Vec<_>>()
        });
        fs::write(
            self.destination.join(MANIFEST_NAME),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        register()
    }
}

/// Records every file a deployment is about to change so it can be undone.
struct RollbackJournal {
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
    fn new(backup_root: PathBuf, fresh_destination: Option<PathBuf>) -> Self {
        Self {
            backup_root,
            counter: 0,
            entries: Vec::new(),
            fresh_destination,
        }
    }

    /// Records `target` before it is created, overwritten, or removed.
    fn track(&mut self, target: &Path) -> Result<()> {
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
    fn rollback(&self) {
        if let Some(destination) = &self.fresh_destination {
            let _ = fs::remove_dir_all(destination);
            return;
        }
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

fn validate_destination(destination: &Path) -> Result<()> {
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

fn collect_staged_files(root: &Path) -> Result<Vec<PathBuf>> {
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

fn uninstall_registry_key(
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

fn registry_path(raw: &str) -> Result<(windows::Win32::System::Registry::HKEY, String)> {
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

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn registry_key_exists(root: windows::Win32::System::Registry::HKEY, path: &str) -> Result<bool> {
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

fn register_uninstaller(
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

fn uninstall(uninstaller: &Path) -> Result<()> {
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
    let config = BundleIndex::read(uninstaller)?
        .context("uninstaller bundle missing")?
        .read_config()?;
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
    let paths = manifest_file_paths(&manifest)?
        .into_iter()
        .map(|relative| destination.join(relative))
        .collect::<Vec<_>>();
    for path in paths {
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
    // The running executable cannot be removed directly; schedule only that file for reboot.
    unsafe {
        MoveFileExW(
            PCWSTR(wide(&uninstaller.display().to_string()).as_ptr()),
            PCWSTR::null(),
            MOVEFILE_DELAY_UNTIL_REBOOT,
        )?;
    }
    unsafe {
        let status = RegDeleteKeyW(root, PCWSTR(wide(&expected_path).as_ptr()));
        if status != ERROR_FILE_NOT_FOUND && status != ERROR_PATH_NOT_FOUND {
            status.ok()?;
        }
    }
    fs::remove_file(manifest_file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        previous_install, register_uninstaller, registry_key_exists, registry_path,
        validate_destination, wide, Deployment, PreviousInstall, MANIFEST_NAME,
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
