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

use super::{read_embedded_bundle, RuntimeMode, UI};

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
    if destination.exists() {
        bail!(
            "installation directory already exists; refusing to overwrite: {}",
            destination.display()
        );
    }
    let bundle = read_embedded_bundle(setup)?.context("installer resource bundle missing")?;
    let config = project_config(&bundle)?;
    let payload_name = config["resources"]["payload_file"]
        .as_str()
        .context("resources.payload_file is required")?;
    let payload = bundle
        .get(payload_name)
        .with_context(|| format!("payload missing: {payload_name}"))?;
    let uninstaller_name = config["output"]["uninstaller_name"]
        .as_str()
        .unwrap_or("uninst.exe");
    super::validate_output_filename(uninstaller_name, "output.uninstaller_name")?;
    let uninstaller = bundle
        .get(&format!("runtime/{uninstaller_name}"))
        .context("self-contained uninstaller missing")?;
    let (root, registry_path) = uninstall_registry_key(&config)?;
    if registry_key_exists(root, &registry_path)? {
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
    fs::write(&archive, payload)?;
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
    deploy_staged(
        &extracted,
        destination,
        &files,
        (uninstaller_name, uninstaller),
        (root, &registry_path),
        || register_uninstaller(root, &registry_path, destination, uninstaller_name, &config),
    )
}

fn deploy_staged(
    extracted: &Path,
    destination: &Path,
    files: &[PathBuf],
    (uninstaller_name, uninstaller): (&str, &[u8]),
    (root, registry_path): (windows::Win32::System::Registry::HKEY, &str),
    register: impl FnOnce() -> Result<()>,
) -> Result<()> {
    validate_destination(destination)?;
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
    let result = (|| -> Result<()> {
        for relative in files {
            let source = extracted.join(relative);
            let target = destination.join(relative);
            fs::create_dir_all(target.parent().context("payload target has no parent")?)?;
            fs::copy(source, target)?;
        }
        fs::write(destination.join(uninstaller_name), uninstaller)?;
        let manifest = json!({
            "version": 1,
            "registry_root": if root == HKEY_CURRENT_USER { "HKCU" } else { "HKLM" },
            "registry_path": registry_path,
            "files": files.iter().map(|path| path.to_string_lossy().to_string()).collect::<Vec<_>>()
        });
        fs::write(
            destination.join(MANIFEST_NAME),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        register()
    })();
    if result.is_err() {
        // The destination did not exist before this run, so it contains only files from this attempt.
        let _ = fs::remove_dir_all(destination);
    }
    result
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

fn project_config(bundle: &std::collections::HashMap<String, Vec<u8>>) -> Result<Value> {
    serde_json::from_slice(
        bundle
            .get("installer_config.json")
            .context("project configuration missing")?,
    )
    .context("invalid installer configuration")
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
    if disposition != REG_CREATED_NEW_KEY {
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
    let config =
        project_config(&read_embedded_bundle(uninstaller)?.context("uninstaller bundle missing")?)?;
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
    if manifest_root
        != if root == HKEY_CURRENT_USER {
            "HKCU"
        } else {
            "HKLM"
        }
        || manifest_path != expected_path
    {
        bail!("installation manifest registry target does not match the project")
    }
    let files = manifest["files"]
        .as_array()
        .context("manifest file list missing")?;
    let mut paths = Vec::new();
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
        paths.push(destination.join(path));
    }
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
        deploy_staged, register_uninstaller, registry_key_exists, registry_path,
        validate_destination, wide, MANIFEST_NAME,
    };
    use anyhow::Result;
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
    #[test]
    fn registry_path_normalizes_legacy_escaped_separators() {
        let (_, path) = registry_path("HKCU\\\\Software\\\\Microsoft\\Uninstall\\App").unwrap();
        assert_eq!(path, "Software\\Microsoft\\Uninstall\\App");
        assert!(registry_path("HKCR\\Somewhere").is_err());
    }
    #[test]
    fn refuses_relative_or_root_installation() {
        assert!(validate_destination(std::path::Path::new("relative\\path")).is_err());
        assert!(validate_destination(std::path::Path::new("C:\\")).is_err());
        assert!(validate_destination(std::path::Path::new("C:\\Program Files\\Example")).is_ok());
    }

    #[test]
    fn deployment_records_files_and_rolls_back_on_failure() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let extracted = temp.path().join("extracted");
        std::fs::create_dir_all(extracted.join("assets"))?;
        std::fs::write(extracted.join("TapTap.exe"), b"app")?;
        std::fs::write(extracted.join("assets/config.ini"), b"config")?;
        let files = ["TapTap.exe", "assets/config.ini"].map(std::path::PathBuf::from);
        let destination = temp.path().join("installed");
        let registry_path = "Software\\nano-installer-test";
        deploy_staged(
            &extracted,
            &destination,
            &files,
            ("uninst.exe", b"uninstaller"),
            (HKEY_CURRENT_USER, registry_path),
            || Ok(()),
        )?;
        assert_eq!(std::fs::read(destination.join("TapTap.exe"))?, b"app");
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(destination.join(MANIFEST_NAME))?)?;
        assert_eq!(manifest["files"].as_array().unwrap().len(), 2);
        assert!(deploy_staged(
            &extracted,
            &destination,
            &files,
            ("uninst.exe", b"uninstaller"),
            (HKEY_CURRENT_USER, registry_path),
            || Ok(())
        )
        .is_err());
        assert_eq!(std::fs::read(destination.join("TapTap.exe"))?, b"app");
        let failed = temp.path().join("failed");
        assert!(deploy_staged(
            &extracted,
            &failed,
            &files,
            ("uninst.exe", b"uninstaller"),
            (HKEY_CURRENT_USER, registry_path),
            || anyhow::bail!("registry unavailable")
        )
        .is_err());
        assert!(!failed.exists());
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
        )?;
        assert!(registry_key_exists(HKEY_CURRENT_USER, &key.0)?);
        assert!(register_uninstaller(
            HKEY_CURRENT_USER,
            &key.0,
            temp.path(),
            "uninst.exe",
            &config
        )
        .is_err());
        Ok(())
    }
}
