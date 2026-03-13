//! System API — drives, env, shell notify, URI scheme, config access

use super::context::ScriptContext;
use rhai::{Dynamic, Engine};

pub fn register(engine: &mut Engine, ctx: ScriptContext) {
    // get_env("APPDATA") -> String
    engine.register_fn("get_env", |name: &str| -> String {
        std::env::var(name).unwrap_or_default()
    });

    // get_drives() -> ["C:\\", "D:\\", ...]  (HDD only)
    engine.register_fn("get_drives", || -> rhai::Array {
        let mut drives = rhai::Array::new();
        #[cfg(windows)]
        {
            for letter in b'A'..=b'Z' {
                let drive = format!("{}:\\", letter as char);
                let wide: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0)).collect();
                let drive_type = unsafe {
                    windows::Win32::Storage::FileSystem::GetDriveTypeW(windows::core::PCWSTR(
                        wide.as_ptr(),
                    ))
                };
                // 3 = DRIVE_FIXED (HDD/SSD)
                if drive_type == 3 {
                    drives.push(Dynamic::from(drive));
                }
            }
        }
        drives
    });

    // shell_notify() — refresh desktop icons after install/uninstall
    engine.register_fn("shell_notify", || {
        #[cfg(windows)]
        {
            unsafe {
                use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
                SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
            }
            tracing::info!("[script] Shell notification sent");
        }
    });

    // run_tracked_uninstall(55.0, 95.0) — execute manifest-driven core uninstall within a script
    let c = ctx.clone();
    engine.register_fn(
        "run_tracked_uninstall",
        move |start_pct: f64, end_pct: f64| -> bool {
            match c.run_manifest_uninstall(start_pct as f32, end_pct as f32) {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("[script] run_tracked_uninstall failed: {}", e);
                    false
                }
            }
        },
    );

    // register_uri_scheme("taptap", "C:\\path\\to\\app.exe")
    engine.register_fn(
        "register_uri_scheme",
        |scheme: &str, exe_path: &str| -> bool {
            #[cfg(windows)]
            {
                use winreg::enums::*;
                use winreg::RegKey;
                let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
                match hkcr.create_subkey(scheme) {
                    Ok((key, _)) => {
                        let _ = key.set_value("", &format!("URL:{} Protocol", scheme));
                        let _ = key.set_value("URL Protocol", &"");
                        if let Ok((cmd_key, _)) = key.create_subkey("shell\\open\\command") {
                            let _ = cmd_key.set_value("", &format!("\"{}\" \"%1\"", exe_path));
                        }
                        tracing::info!("[script] Registered URI scheme: {}://", scheme);
                        true
                    }
                    Err(e) => {
                        tracing::error!("[script] register_uri_scheme failed: {}", e);
                        false
                    }
                }
            }
            #[cfg(not(windows))]
            {
                true
            }
        },
    );

    // delete_uri_scheme("taptap")
    engine.register_fn("delete_uri_scheme", |scheme: &str| -> bool {
        #[cfg(windows)]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
            let _ = hkcr.delete_subkey_all(scheme);
            tracing::info!("[script] Deleted URI scheme: {}://", scheme);
            true
        }
        #[cfg(not(windows))]
        {
            true
        }
    });

    // get_config_value("project.name") -> Dynamic
    let c = ctx.clone();
    engine.register_fn("get_config_value", move |path: &str| -> Dynamic {
        let mut current = c.config_json.as_ref();
        for key in path.split('.') {
            match current.get(key) {
                Some(v) => current = v,
                None => return Dynamic::UNIT,
            }
        }
        json_to_dynamic(current)
    });

    // get_drive_space("C:\\") -> [free_bytes, total_bytes]
    engine.register_fn("get_drive_space", |drive: &str| -> rhai::Array {
        let mut result = rhai::Array::new();
        #[cfg(windows)]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = OsStr::new(drive)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let mut free: u64 = 0;
            let mut total: u64 = 0;
            unsafe {
                let _ = windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                    windows::core::PCWSTR(wide.as_ptr()),
                    Some(&mut free as *mut u64),
                    Some(&mut total as *mut u64),
                    None,
                );
            }
            result.push(Dynamic::from(free as i64));
            result.push(Dynamic::from(total as i64));
        }
        result
    });

    // get_current_exe() -> String
    engine.register_fn("get_current_exe", || -> String {
        std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    // get_exe_dir() -> String (parent directory of current exe)
    engine.register_fn("get_exe_dir", || -> String {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_default()
    });

    // show_message(title, message) — blocking message box
    engine.register_fn("show_message", |title: &str, message: &str| {
        #[cfg(windows)]
        {
            use windows::core::HSTRING;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
            unsafe {
                MessageBoxW(
                    None,
                    &HSTRING::from(message),
                    &HSTRING::from(title),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
        }
    });

    // show_error(title, message) — blocking error dialog
    engine.register_fn("show_error", |title: &str, message: &str| {
        #[cfg(windows)]
        {
            use windows::core::HSTRING;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
            unsafe {
                MessageBoxW(
                    None,
                    &HSTRING::from(message),
                    &HSTRING::from(title),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
    });

    // ask_yes_no(title, message) -> bool
    engine.register_fn("ask_yes_no", |title: &str, message: &str| -> bool {
        #[cfg(windows)]
        {
            use windows::core::HSTRING;
            use windows::Win32::UI::WindowsAndMessaging::*;
            let result = unsafe {
                MessageBoxW(
                    None,
                    &HSTRING::from(message),
                    &HSTRING::from(title),
                    MB_YESNO | MB_ICONQUESTION,
                )
            };
            return result == IDYES;
        }
        #[cfg(not(windows))]
        {
            true
        }
    });

    // is_elevated() -> bool
    engine.register_fn("is_elevated", || -> bool {
        #[cfg(windows)]
        {
            crate::common::platform::is_elevated().unwrap_or(false)
        }
        #[cfg(not(windows))]
        {
            false
        }
    });
}

/// Convert serde_json::Value to rhai::Dynamic
fn json_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let rhai_arr: rhai::Array = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(rhai_arr)
        }
        serde_json::Value::Object(map) => {
            let mut rhai_map = rhai::Map::new();
            for (k, v) in map {
                rhai_map.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(rhai_map)
        }
    }
}
