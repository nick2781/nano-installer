//! Dependency primitives: what the machine already has, and putting there what
//! it does not.
//!
//! Both read the project's own `dependencies.items`, so a script decides *when*
//! a dependency is checked and installed while the project keeps saying *what*
//! it is and how it is recognized.

use rhai::Engine;

use super::context::{log, ScriptContext};
use super::Mode;
use crate::dependency;

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn("dependency_installed", move |id: &str| -> bool {
        match dependency::installed(c.config(), id) {
            Ok(installed) => installed,
            Err(error) => {
                log(
                    "error",
                    &format!("dependency_installed {id} failed: {error:#}"),
                );
                false
            }
        }
    });

    let c = context.clone();
    engine.register_fn("install_dependency", move |id: &str| -> bool {
        if c.mode() != Mode::Install {
            log(
                "error",
                "install_dependency is only available while installing",
            );
            return false;
        }
        // The script owns the progress the page shows, so the install reports
        // nothing of its own: what the page says while a dependency is being
        // put there is what the script says.
        let outcome = dependency::install(
            c.config(),
            id,
            c.bundle(),
            c.stage(),
            &mut |_| {},
            c.cancellation(),
        );
        match outcome {
            Ok(true) => true,
            Ok(false) => {
                log(
                    "error",
                    &format!(
                        "install_dependency {id}: the rule the project declares still says the machine is missing it"
                    ),
                );
                false
            }
            Err(error) => {
                log(
                    "error",
                    &format!("install_dependency {id} failed: {error:#}"),
                );
                false
            }
        }
    });
}
