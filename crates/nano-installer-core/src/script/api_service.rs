//! Services: the background program a product installs alongside its files.
//!
//! A service is not a file of the installation but an entry the machine keeps
//! pointing at one, so every call here needs an elevated process and the refusal
//! a plain run gets is reported as it came. What a script installed is recorded
//! on the run: the uninstall deletes what the manifest names, and a failure
//! deletes what the script had installed before it gave up.

use rhai::Engine;
use std::path::PathBuf;

use super::context::{log, ScriptContext};
use crate::service::{self, StartType};

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn(
        "service_install",
        move |name: &str, display_name: &str, binary: &str, arguments: &str| -> bool {
            install(&c, name, display_name, binary, arguments)
        },
    );

    engine.register_fn("service_exists", |name: &str| -> bool {
        answer("service_exists", name, service::exists(name))
    });

    engine.register_fn("service_running", |name: &str| -> bool {
        answer("service_running", name, service::running(name))
    });

    engine.register_fn("service_start", |name: &str| -> bool {
        done("service_start", name, service::start(name))
    });

    engine.register_fn("service_stop", |name: &str| -> bool {
        done("service_stop", name, service::stop(name))
    });

    engine.register_fn("service_set_start_type", |name: &str, kind: &str| -> bool {
        set_start_type(name, kind)
    });

    let c = context.clone();
    engine.register_fn("service_delete", move |name: &str| -> bool {
        delete(&c, name)
    });
}

/// Installs the service and records it as this installation's own.
///
/// The service is set to start with the machine. Installing it does not start
/// it -- that is `service_start` -- so a product whose service cannot start
/// still installs, and the failure is reported where the script asks for it.
fn install(
    context: &ScriptContext,
    name: &str,
    display_name: &str,
    binary: &str,
    arguments: &str,
) -> bool {
    let Some(program) = program(context, binary) else {
        return false;
    };
    match service::install(
        name,
        display_name,
        &program,
        arguments,
        StartType::Automatic,
    ) {
        Ok(()) => {
            context.record_service(name);
            true
        }
        Err(error) => {
            log(
                "error",
                &format!("service_install {name} failed: {error:#}"),
            );
            false
        }
    }
}

/// The program a service runs.
///
/// A path with no directory of its own names a file of the installation, which
/// is where the program a product ships lives; anything else is used the way
/// the script wrote it.
fn program(context: &ScriptContext, binary: &str) -> Option<PathBuf> {
    let binary = binary.trim();
    if binary.is_empty() {
        log(
            "error",
            "service_install needs the program the service runs",
        );
        return None;
    }
    let path = PathBuf::from(binary);
    Some(if path.is_absolute() {
        path
    } else {
        context.install_path().join(path)
    })
}

/// Changes how the service starts.
fn set_start_type(name: &str, kind: &str) -> bool {
    let Some(start) = StartType::parse(kind) else {
        log(
            "error",
            &format!(
                "service_set_start_type does not know the start kind {kind}: \
                 use auto, delayed, manual or disabled"
            ),
        );
        return false;
    };
    done(
        "service_set_start_type",
        name,
        service::set_start_type(name, start),
    )
}

/// Deletes the service and stops treating it as this installation's own.
fn delete(context: &ScriptContext, name: &str) -> bool {
    if !done("service_delete", name, service::delete(name)) {
        return false;
    }
    context.forget_service(name);
    true
}

/// Reports what a question about a service answered, or what stopped it.
fn answer(call: &str, name: &str, result: anyhow::Result<bool>) -> bool {
    match result {
        Ok(value) => value,
        Err(error) => {
            log("error", &format!("{call} {name} failed: {error:#}"));
            false
        }
    }
}

/// Reports whether a call that changes a service went through.
fn done(call: &str, name: &str, result: anyhow::Result<()>) -> bool {
    match result {
        Ok(()) => true,
        Err(error) => {
            log("error", &format!("{call} {name} failed: {error:#}"));
            false
        }
    }
}
