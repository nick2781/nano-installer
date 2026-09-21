//! Registry primitives, backed by the same key type the built-in flow uses so
//! both paths share their error handling.
//!
//! A key may name the view it is read and written in, which a 64-bit Windows
//! keeps two of: `HKLM32\SOFTWARE\...` is the copy a 32-bit program sees,
//! `HKLM64\...` the one a 64-bit program sees, and a bare `HKLM` is whichever
//! this process is subject to.

use rhai::{Array, Dynamic, Engine};
use windows::Win32::System::Registry::{
    REG_BINARY, REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ, REG_NONE, REG_QWORD, REG_SZ, REG_VALUE_TYPE,
};

use super::context::{log, ScriptContext};
use crate::install::{parse_registry_key, utf16_text, RegistryKey};

/// Splits `HKCU\Software\...` into the key a primitive reads and writes.
pub(super) fn split(key: &str) -> Option<RegistryKey> {
    match parse_registry_key(key) {
        Ok(key) => Some(key),
        Err(error) => {
            log("error", &format!("invalid registry key {key}: {error:#}"));
            None
        }
    }
}

/// The name Windows gives a value type, as `reg_read_type` reports it.
fn type_name(kind: REG_VALUE_TYPE) -> &'static str {
    match kind {
        REG_NONE => "REG_NONE",
        REG_SZ => "REG_SZ",
        REG_EXPAND_SZ => "REG_EXPAND_SZ",
        REG_BINARY => "REG_BINARY",
        REG_DWORD => "REG_DWORD",
        REG_MULTI_SZ => "REG_MULTI_SZ",
        REG_QWORD => "REG_QWORD",
        _ => "REG_OTHER",
    }
}

/// A text value's bytes, its terminator included.
fn text_bytes(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// A `REG_MULTI_SZ` value's bytes: every string terminated, and the list closed
/// by one more terminator, which is also all an empty list is.
///
/// An empty string is dropped rather than written: it would end the list where it
/// stands, so a script could not tell the rest of its own array from what Windows
/// reads back.
fn multi_string_bytes(values: &[String]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in values.iter().filter(|value| !value.is_empty()) {
        bytes.extend(text_bytes(value));
    }
    bytes.extend([0, 0]);
    bytes
}

/// A `REG_BINARY` value's bytes, or why the array is not one.
fn binary_bytes(values: &Array) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::with_capacity(values.len());
    for value in values {
        let number = value
            .as_int()
            .map_err(|_| "every byte must be a whole number".to_string())?;
        bytes.push(u8::try_from(number).map_err(|_| format!("{number} is not a byte"))?);
    }
    Ok(bytes)
}

/// The strings of a rhai array, ignoring anything that is not one.
fn strings(values: Array) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| value.into_string().ok())
        .collect()
}

/// Writes through `write` and records it, so the uninstall takes it back.
fn write_with(
    context: &ScriptContext,
    primitive: &str,
    key: &str,
    name: &str,
    write: impl FnOnce(&RegistryKey) -> anyhow::Result<()>,
) -> bool {
    let Some(target) = split(key) else {
        return false;
    };
    match write(&target) {
        Ok(()) => {
            context.record_registry_write(key, name);
            true
        }
        Err(error) => {
            log(
                "error",
                &format!("{primitive} {key}\\{name} failed: {error:#}"),
            );
            false
        }
    }
}

/// Writes one value of the given type.
fn write_value(
    context: &ScriptContext,
    primitive: &str,
    key: &str,
    name: &str,
    kind: REG_VALUE_TYPE,
    bytes: &[u8],
) -> bool {
    write_with(context, primitive, key, name, |target| {
        target.write(name, kind, bytes)
    })
}

/// The value stored under `key\name`, or `None` when the key, the value, or the
/// read itself says nothing is there.
fn read_value(primitive: &str, key: &str, name: &str) -> Option<(REG_VALUE_TYPE, Vec<u8>)> {
    let target = split(key)?;
    match target.read_raw(name) {
        Ok(value) => value,
        Err(error) => {
            log(
                "error",
                &format!("{primitive} {key}\\{name} failed: {error:#}"),
            );
            None
        }
    }
}

/// A `REG_DWORD` or `REG_QWORD` payload as a number, `-1` when the bytes are of
/// another width.
fn number(bytes: &[u8], width: usize) -> i64 {
    if bytes.len() < width {
        return -1;
    }
    let mut padded = [0u8; 8];
    padded[..width].copy_from_slice(&bytes[..width]);
    i64::from_le_bytes(padded)
}

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn(
        "reg_write_string",
        move |key: &str, name: &str, value: &str| -> bool {
            write_value(
                &c,
                "reg_write_string",
                key,
                name,
                REG_SZ,
                &text_bytes(value),
            )
        },
    );

    let c = context.clone();
    engine.register_fn(
        "reg_write_expand_string",
        move |key: &str, name: &str, value: &str| -> bool {
            write_value(
                &c,
                "reg_write_expand_string",
                key,
                name,
                REG_EXPAND_SZ,
                &text_bytes(value),
            )
        },
    );

    let c = context.clone();
    engine.register_fn(
        "reg_write_multi_string",
        move |key: &str, name: &str, values: Array| -> bool {
            write_value(
                &c,
                "reg_write_multi_string",
                key,
                name,
                REG_MULTI_SZ,
                &multi_string_bytes(&strings(values)),
            )
        },
    );

    let c = context.clone();
    engine.register_fn(
        "reg_write_dword",
        move |key: &str, name: &str, value: i64| -> bool {
            let Ok(value) = u32::try_from(value) else {
                log(
                    "error",
                    &format!("reg_write_dword: {value} is not a dword (0 to 4294967295)"),
                );
                return false;
            };
            write_with(&c, "reg_write_dword", key, name, |target| {
                target.write_dword(name, value)
            })
        },
    );

    let c = context.clone();
    engine.register_fn(
        "reg_write_qword",
        move |key: &str, name: &str, value: i64| -> bool {
            write_value(
                &c,
                "reg_write_qword",
                key,
                name,
                REG_QWORD,
                &(value as u64).to_le_bytes(),
            )
        },
    );

    let c = context.clone();
    engine.register_fn(
        "reg_write_binary",
        move |key: &str, name: &str, values: Array| -> bool {
            let bytes = match binary_bytes(&values) {
                Ok(bytes) => bytes,
                Err(reason) => {
                    log("error", &format!("reg_write_binary: {reason}"));
                    return false;
                }
            };
            write_value(&c, "reg_write_binary", key, name, REG_BINARY, &bytes)
        },
    );

    engine.register_fn("reg_read", |key: &str, name: &str| -> String {
        let Some(target) = split(key) else {
            return String::new();
        };
        match target.read_string(name) {
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

    engine.register_fn("reg_read_dword", |key: &str, name: &str| -> i64 {
        match read_value("reg_read_dword", key, name) {
            Some((kind, bytes)) if kind == REG_DWORD => number(&bytes, 4),
            _ => -1,
        }
    });

    engine.register_fn("reg_read_qword", |key: &str, name: &str| -> i64 {
        match read_value("reg_read_qword", key, name) {
            Some((kind, bytes)) if kind == REG_QWORD => number(&bytes, 8),
            _ => -1,
        }
    });

    engine.register_fn(
        "reg_read_expand_string",
        |key: &str, name: &str| -> String {
            let Some((kind, bytes)) = read_value("reg_read_expand_string", key, name) else {
                return String::new();
            };
            if kind != REG_EXPAND_SZ {
                return String::new();
            }
            let text = utf16_text(&bytes);
            match crate::shell::expand_environment(&text) {
                Ok(expanded) => expanded,
                Err(error) => {
                    log(
                        "warn",
                        &format!("reg_read_expand_string {key}\\{name}: {error:#}"),
                    );
                    text
                }
            }
        },
    );

    engine.register_fn("reg_read_multi_string", |key: &str, name: &str| -> Array {
        let Some((kind, bytes)) = read_value("reg_read_multi_string", key, name) else {
            return Array::new();
        };
        if kind != REG_MULTI_SZ {
            return Array::new();
        }
        utf16_strings(&bytes)
            .into_iter()
            .map(Dynamic::from)
            .collect()
    });

    engine.register_fn("reg_read_binary", |key: &str, name: &str| -> Array {
        let Some((kind, bytes)) = read_value("reg_read_binary", key, name) else {
            return Array::new();
        };
        if kind != REG_BINARY {
            return Array::new();
        }
        bytes
            .into_iter()
            .map(|byte| Dynamic::from(i64::from(byte)))
            .collect()
    });

    engine.register_fn("reg_read_type", |key: &str, name: &str| -> String {
        match read_value("reg_read_type", key, name) {
            Some((kind, _)) => type_name(kind).to_string(),
            None => String::new(),
        }
    });

    engine.register_fn("reg_value_exists", |key: &str, name: &str| -> bool {
        read_value("reg_value_exists", key, name).is_some()
    });

    engine.register_fn("reg_key_exists", |key: &str| -> bool {
        let Some(target) = split(key) else {
            return false;
        };
        target.exists().unwrap_or(false)
    });

    engine.register_fn("reg_delete_value", |key: &str, name: &str| -> bool {
        let Some(target) = split(key) else {
            return false;
        };
        target.delete_value(name).is_ok()
    });

    let c = context.clone();
    engine.register_fn("reg_delete_key", move |key: &str| -> bool {
        let Some(target) = split(key) else {
            return false;
        };
        if let Err(error) = target.delete_key() {
            log("error", &format!("reg_delete_key {key} failed: {error:#}"));
            return false;
        }
        // The uninstaller must not try to delete it again.
        c.forget_registry_key(key);
        true
    });
}

/// The strings of a `REG_MULTI_SZ` payload, which ends at the first empty one.
fn utf16_strings(bytes: &[u8]) -> Vec<String> {
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    units
        .split(|unit| *unit == 0)
        .take_while(|part| !part.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}
