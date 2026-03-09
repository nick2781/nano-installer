//! Registry API — read, write, delete registry keys and values

use rhai::Engine;
use super::context::ScriptContext;

pub fn register(engine: &mut Engine, _ctx: ScriptContext) {
    // reg_write_string("HKCU\\Software\\MyApp", "InstallPath", "C:\\...")
    engine.register_fn("reg_write_string", |key: &str, name: &str, value: &str| -> bool {
        #[cfg(windows)]
        {
            let (hive, subkey) = match parse_reg_key(key) {
                Some(v) => v,
                None => return false,
            };
            match hive.create_subkey(subkey) {
                Ok((reg_key, _)) => {
                    match reg_key.set_value(name, &value) {
                        Ok(()) => true,
                        Err(e) => {
                            tracing::error!("[script] reg_write_string failed: {}", e);
                            false
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("[script] reg_write_string create key failed: {}", e);
                    false
                }
            }
        }
        #[cfg(not(windows))]
        { true }
    });

    // reg_write_dword("HKLM\\...", "NoModify", 1)
    engine.register_fn("reg_write_dword", |key: &str, name: &str, value: i64| -> bool {
        #[cfg(windows)]
        {
            let (hive, subkey) = match parse_reg_key(key) {
                Some(v) => v,
                None => return false,
            };
            match hive.create_subkey(subkey) {
                Ok((reg_key, _)) => {
                    match reg_key.set_value(name, &(value as u32)) {
                        Ok(()) => true,
                        Err(e) => {
                            tracing::error!("[script] reg_write_dword failed: {}", e);
                            false
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("[script] reg_write_dword create key failed: {}", e);
                    false
                }
            }
        }
        #[cfg(not(windows))]
        { true }
    });

    // reg_read("HKCU\\Software\\MyApp", "InstallPath") -> String
    engine.register_fn("reg_read", |key: &str, name: &str| -> String {
        #[cfg(windows)]
        {
            let (hive, subkey) = match parse_reg_key(key) {
                Some(v) => v,
                None => return String::new(),
            };
            match hive.open_subkey(subkey) {
                Ok(reg_key) => reg_key.get_value::<String, _>(name).unwrap_or_default(),
                Err(_) => String::new(),
            }
        }
        #[cfg(not(windows))]
        { String::new() }
    });

    // reg_delete_key("HKCU\\Software\\MyApp")
    engine.register_fn("reg_delete_key", |key: &str| -> bool {
        #[cfg(windows)]
        {
            let (hive, subkey) = match parse_reg_key(key) {
                Some(v) => v,
                None => return false,
            };
            hive.delete_subkey_all(subkey).is_ok()
        }
        #[cfg(not(windows))]
        { true }
    });

    // reg_delete_value("HKCU\\Software\\MyApp", "SomeValue")
    engine.register_fn("reg_delete_value", |key: &str, name: &str| -> bool {
        #[cfg(windows)]
        {
            let (hive, subkey) = match parse_reg_key(key) {
                Some(v) => v,
                None => return false,
            };
            match hive.open_subkey_with_flags(subkey, winreg::enums::KEY_WRITE) {
                Ok(reg_key) => reg_key.delete_value(name).is_ok(),
                Err(_) => false,
            }
        }
        #[cfg(not(windows))]
        { true }
    });

    // reg_key_exists("HKCU\\Software\\MyApp") -> bool
    engine.register_fn("reg_key_exists", |key: &str| -> bool {
        #[cfg(windows)]
        {
            let (hive, subkey) = match parse_reg_key(key) {
                Some(v) => v,
                None => return false,
            };
            hive.open_subkey(subkey).is_ok()
        }
        #[cfg(not(windows))]
        { false }
    });
}

/// Parse "HKCU\\Software\\MyApp" → (RegKey, "Software\\MyApp")
#[cfg(windows)]
fn parse_reg_key(key: &str) -> Option<(winreg::RegKey, &str)> {
    use winreg::enums::*;
    use winreg::RegKey;

    let (hive_str, subkey) = key.split_once('\\')?;
    let hive = match hive_str.to_uppercase().as_str() {
        "HKCU" | "HKEY_CURRENT_USER" => RegKey::predef(HKEY_CURRENT_USER),
        "HKLM" | "HKEY_LOCAL_MACHINE" => RegKey::predef(HKEY_LOCAL_MACHINE),
        "HKCR" | "HKEY_CLASSES_ROOT" => RegKey::predef(HKEY_CLASSES_ROOT),
        _ => {
            tracing::error!("[script] Unknown registry hive: {}", hive_str);
            return None;
        }
    };
    Some((hive, subkey))
}
