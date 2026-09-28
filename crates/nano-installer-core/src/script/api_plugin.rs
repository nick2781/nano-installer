//! Plugin primitives.
//!
//! A project ships plugins in the directory its configuration names as
//! `resources.plugins_dir`, and a script calls one as `dll::function`. The
//! contract a plugin author writes against is `include/nano_plugin.h`; this
//! module is where a script reaches it, and where the host services a plugin
//! gets are implemented over the same code the script primitives use.
//!
//! A call that cannot be made fails the script. That is deliberately not the
//! shape the other primitives have -- they answer with a value and leave the
//! decision to the script -- because those answer about this machine, while a
//! plugin call is third-party code the script asked for by name: a call that
//! did not happen is not an answer, and a script that wants to carry on anyway
//! has `try`.

use rhai::{Array, Dynamic, Engine, EvalAltResult, Position};
use std::path::{Path, PathBuf};

use super::api_file::write as write_into_installation;
use super::api_registry::write_value;
use super::context::{log, ScriptContext};
use crate::plugin::{self, Services};

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn(
        "plugin_call",
        move |spec: &str, arguments: Array| -> Result<String, Box<EvalAltResult>> {
            let values = call(&c, spec, arguments)?;
            Ok(values.into_iter().next().unwrap_or_default())
        },
    );

    let c = context.clone();
    engine.register_fn(
        "plugin_values",
        move |spec: &str, arguments: Array| -> Result<Array, Box<EvalAltResult>> {
            let values = call(&c, spec, arguments)?;
            Ok(values.into_iter().map(Dynamic::from).collect())
        },
    );
}

/// Calls one plugin function and answers with what it pushed.
fn call(
    context: &ScriptContext,
    spec: &str,
    arguments: Array,
) -> Result<Vec<String>, Box<EvalAltResult>> {
    let failed = |error: anyhow::Error| -> Box<EvalAltResult> {
        Box::new(EvalAltResult::ErrorRuntime(
            Dynamic::from(format!("{error:#}")),
            Position::NONE,
        ))
    };

    let (name, function) = plugin::split_call(spec).map_err(failed)?;
    let directory = plugins_directory(context).map_err(|error| {
        log("error", &format!("plugin_call {spec} failed: {error:#}"));
        failed(error)
    })?;
    let file = directory.join(format!("{name}.dll"));
    if !file.is_file() {
        let error = anyhow::anyhow!(
            "the setup ships no plugin called `{name}`, so `{spec}` cannot be called; a plugin is a DLL in the project's plugins directory"
        );
        log("error", &format!("{error:#}"));
        return Err(failed(error));
    }

    // The library is taken and the state lock released before the call: the
    // plugin's services reach back into this same state, and holding it across
    // the call would deadlock the first time a plugin wrote anything.
    let library = {
        let mut state = context.state();
        state.libraries.library(&name, &file).map_err(|error| {
            log("error", &format!("plugin_call {spec} failed: {error:#}"));
            failed(error)
        })?
    };

    let text: Vec<String> = arguments.iter().map(|value| value.to_string()).collect();
    let mut services = HostServices {
        context: context.clone(),
    };
    library
        .call(&function, &text, &mut services)
        .map_err(|error| {
            log("error", &format!("plugin_call {spec} failed: {error:#}"));
            failed(error)
        })
}

/// Unpacks the plugins a project bundled and returns the directory holding them.
///
/// The bytes come out of the bundle, which checks every entry against the digest
/// the build recorded, so what gets loaded is what the project built and not
/// what the file on this machine happens to be.
fn plugins_directory(context: &ScriptContext) -> anyhow::Result<PathBuf> {
    if let Some(directory) = context.plugins_directory() {
        return Ok(directory);
    }
    let stored = context.config()["resources"]["plugins_dir"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| {
            anyhow::anyhow!("the project bundles no resources.plugins_dir, so it ships no plugins")
        })?;
    let entries = context.bundle().read_directory(&stored)?;
    if entries.is_empty() {
        anyhow::bail!("the bundle carries nothing under {stored}");
    }
    let directory = context.stage().join("plugins");
    let prefix = format!("{}/", stored.trim_end_matches(['/', '\\']));
    for (name, contents) in &entries {
        let target = directory.join(name.strip_prefix(&prefix).unwrap_or(name));
        let Some(parent) = target.parent() else {
            continue;
        };
        std::fs::create_dir_all(parent)
            .and_then(|()| std::fs::write(&target, contents))
            .map_err(|error| {
                anyhow::anyhow!("cannot write the plugin {}: {error}", target.display())
            })?;
    }
    context.set_plugins_directory(directory.clone());
    Ok(directory)
}

/// What one call can do for a plugin, over this script's own context.
struct HostServices {
    context: ScriptContext,
}

impl Services for HostServices {
    fn log(&mut self, level: i32, message: &str) {
        let level = match level {
            plugin::LOG_ERROR => "error",
            plugin::LOG_WARN => "warn",
            plugin::LOG_INFO => "info",
            _ => "info",
        };
        log(level, &format!("plugin: {message}"));
    }

    fn window(&mut self) -> *mut std::ffi::c_void {
        crate::runtime_window()
            .map(|window| window.0)
            .unwrap_or(std::ptr::null_mut())
    }

    fn cancelled(&mut self) -> bool {
        self.context.cancellation().requested()
    }

    fn install_dir(&mut self) -> String {
        self.context.install_path_text()
    }

    fn write_file(&mut self, path: &str, text: &str) -> anyhow::Result<()> {
        let target = inside_installation(&self.context, path)?;
        write_into_installation(&self.context, &target, text.as_bytes())
    }

    fn write_registry(
        &mut self,
        key: &str,
        name: &str,
        kind: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        let (kind, bytes) = registry_value(kind, value)?;
        if write_value(
            &self.context,
            "plugin write_registry",
            key,
            name,
            kind,
            &bytes,
        ) {
            Ok(())
        } else {
            anyhow::bail!("{key}\\{name} could not be written")
        }
    }
}

/// The file a plugin asked to write, refused unless the installation owns it.
///
/// Everything below the installation directory ends up in the manifest by the
/// same snapshot the install takes, so uninstalling removes it. Anywhere else
/// would be a file this product leaves on the machine with nothing recording it,
/// which is exactly what writing through the host exists to prevent.
fn inside_installation(context: &ScriptContext, path: &str) -> anyhow::Result<PathBuf> {
    let installation = context.install_path();
    let requested = Path::new(path);
    let target = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        installation.join(requested)
    };
    let inside = target
        .to_string_lossy()
        .to_ascii_lowercase()
        .starts_with(&installation.to_string_lossy().to_ascii_lowercase());
    if !inside {
        anyhow::bail!(
            "{} is outside the installation directory {}; a plugin writes what the uninstaller will take back",
            target.display(),
            installation.display()
        );
    }
    Ok(target)
}

/// One registry value of the kind the ABI names, as the bytes to store.
fn registry_value(
    kind: &str,
    value: &str,
) -> anyhow::Result<(windows::Win32::System::Registry::REG_VALUE_TYPE, Vec<u8>)> {
    use windows::Win32::System::Registry::{REG_DWORD, REG_EXPAND_SZ, REG_QWORD, REG_SZ};

    match kind.trim().to_ascii_lowercase().as_str() {
        "string" | "sz" => Ok((REG_SZ, super::api_registry::text_bytes(value))),
        "expand" | "expand_sz" => Ok((REG_EXPAND_SZ, super::api_registry::text_bytes(value))),
        "dword" => {
            let number: u32 = value
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("`{value}` is not a number a DWORD can hold"))?;
            Ok((REG_DWORD, number.to_le_bytes().to_vec()))
        }
        "qword" => {
            let number: u64 = value
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("`{value}` is not a number a QWORD can hold"))?;
            Ok((REG_QWORD, number.to_le_bytes().to_vec()))
        }
        other => anyhow::bail!(
            "`{other}` is not a kind this ABI writes: use string, expand, dword or qword"
        ),
    }
}
