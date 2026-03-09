//! Shortcut API — create/delete desktop and start menu shortcuts

use rhai::Engine;
use super::context::ScriptContext;
use std::path::PathBuf;

pub fn register(engine: &mut Engine, _ctx: ScriptContext) {
    // create_desktop_shortcut("MyApp", "C:\\path\\to\\app.exe")
    engine.register_fn("create_desktop_shortcut", |name: &str, exe_path: &str| -> bool {
        #[cfg(windows)]
        {
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                let desktop = PathBuf::from(userprofile).join("Desktop");
                return create_lnk(&desktop.join(format!("{}.lnk", name)), exe_path);
            }
            false
        }
        #[cfg(not(windows))]
        { true }
    });

    // create_start_menu_shortcut("MyApp", "C:\\path\\to\\app.exe", "MyApp")
    engine.register_fn("create_start_menu_shortcut", |name: &str, exe_path: &str, folder: &str| -> bool {
        #[cfg(windows)]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let folder_path = PathBuf::from(appdata)
                    .join("Microsoft\\Windows\\Start Menu\\Programs")
                    .join(folder);
                let _ = std::fs::create_dir_all(&folder_path);
                return create_lnk(&folder_path.join(format!("{}.lnk", name)), exe_path);
            }
            false
        }
        #[cfg(not(windows))]
        { true }
    });

    // create_uninstall_shortcut("Uninstall MyApp", "C:\\path\\to\\uninst.exe", "MyApp")
    engine.register_fn("create_uninstall_shortcut", |name: &str, exe_path: &str, folder: &str| -> bool {
        #[cfg(windows)]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let folder_path = PathBuf::from(appdata)
                    .join("Microsoft\\Windows\\Start Menu\\Programs")
                    .join(folder);
                let _ = std::fs::create_dir_all(&folder_path);
                return create_lnk(&folder_path.join(format!("{}.lnk", name)), exe_path);
            }
            false
        }
        #[cfg(not(windows))]
        { true }
    });

    // delete_desktop_shortcut("MyApp")
    engine.register_fn("delete_desktop_shortcut", |name: &str| -> bool {
        #[cfg(windows)]
        {
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                let shortcut = PathBuf::from(userprofile).join("Desktop").join(format!("{}.lnk", name));
                if shortcut.exists() {
                    return std::fs::remove_file(shortcut).is_ok();
                }
            }
            true
        }
        #[cfg(not(windows))]
        { true }
    });

    // delete_start_menu_folder("MyApp")
    engine.register_fn("delete_start_menu_folder", |folder: &str| -> bool {
        #[cfg(windows)]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let path = PathBuf::from(appdata)
                    .join("Microsoft\\Windows\\Start Menu\\Programs")
                    .join(folder);
                if path.exists() {
                    return std::fs::remove_dir_all(path).is_ok();
                }
            }
            true
        }
        #[cfg(not(windows))]
        { true }
    });
}

/// Create a Windows .lnk shortcut file using COM
#[cfg(windows)]
fn create_lnk(lnk_path: &std::path::Path, target: &str) -> bool {
    // Use powershell to create .lnk (simpler than COM interop)
    let ps_script = format!(
        "$ws = New-Object -ComObject WScript.Shell; \
         $s = $ws.CreateShortcut('{}'); \
         $s.TargetPath = '{}'; \
         $s.Save()",
        lnk_path.display(),
        target
    );
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    match std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &ps_script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                tracing::info!("[script] Created shortcut: {}", lnk_path.display());
                true
            } else {
                tracing::error!("[script] Shortcut creation failed: {}",
                    String::from_utf8_lossy(&output.stderr));
                false
            }
        }
        Err(e) => {
            tracing::error!("[script] powershell failed: {}", e);
            false
        }
    }
}
