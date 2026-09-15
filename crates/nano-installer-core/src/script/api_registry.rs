//! Registry primitives, backed by the same reader and writer the built-in
//! flow uses so both paths share their error handling.

use rhai::Engine;

use super::context::{log, ScriptContext};
use crate::install::{
    delete_registry_key, delete_registry_value, read_registry_string, registry_key_exists,
    registry_path, write_registry_dword, write_registry_string,
};

/// Splits `HKCU\Software\...` into a hive handle and a subkey path.
fn split(key: &str) -> Option<(windows::Win32::System::Registry::HKEY, String)> {
    match registry_path(key) {
        Ok(parts) => Some(parts),
        Err(error) => {
            log("error", &format!("invalid registry key {key}: {error:#}"));
            None
        }
    }
}

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn(
        "reg_write_string",
        move |key: &str, name: &str, value: &str| -> bool {
            let Some((root, path)) = split(key) else {
                return false;
            };
            match write_registry_string(root, &path, name, value) {
                Ok(()) => {
                    c.record_registry_write(key, name);
                    true
                }
                Err(error) => {
                    log(
                        "error",
                        &format!("reg_write_string {key}\\{name} failed: {error:#}"),
                    );
                    false
                }
            }
        },
    );

    let c = context.clone();
    engine.register_fn(
        "reg_write_dword",
        move |key: &str, name: &str, value: i64| -> bool {
            let Some((root, path)) = split(key) else {
                return false;
            };
            match write_registry_dword(root, &path, name, value as u32) {
                Ok(()) => {
                    c.record_registry_write(key, name);
                    true
                }
                Err(error) => {
                    log(
                        "error",
                        &format!("reg_write_dword {key}\\{name} failed: {error:#}"),
                    );
                    false
                }
            }
        },
    );

    engine.register_fn("reg_read", |key: &str, name: &str| -> String {
        let Some((root, path)) = split(key) else {
            return String::new();
        };
        match read_registry_string(root, &path, name) {
            Ok(Some(value)) => value,
            Ok(None) => String::new(),
            Err(error) => {
                log(
                    "error",
                    &format!("reg_read {key}\\{name} failed: {error:#}"),
                );
                String::new()
            }
        }
    });

    engine.register_fn("reg_key_exists", |key: &str| -> bool {
        let Some((root, path)) = split(key) else {
            return false;
        };
        registry_key_exists(root, &path).unwrap_or(false)
    });

    engine.register_fn("reg_delete_value", |key: &str, name: &str| -> bool {
        let Some((root, path)) = split(key) else {
            return false;
        };
        delete_registry_value(root, &path, name).is_ok()
    });

    let c = context.clone();
    engine.register_fn("reg_delete_key", move |key: &str| -> bool {
        let Some((root, path)) = split(key) else {
            return false;
        };
        if let Err(error) = delete_registry_key(root, &path) {
            log("error", &format!("reg_delete_key {key} failed: {error:#}"));
            return false;
        }
        // The uninstaller must not try to delete it again.
        let mut state = c.state();
        state.registry_keys.retain(|recorded| recorded != key);
        state
            .registry_values
            .retain(|(recorded, _)| recorded.as_str() != key);
        true
    });
}
