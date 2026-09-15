//! Process primitives: detect and stop a running product.

use rhai::Engine;

use super::context::log;
use crate::shell;

pub(super) fn register(engine: &mut Engine) {
    engine.register_fn("is_process_running", |name: &str| -> bool {
        match shell::process_running(name) {
            Ok(running) => running,
            Err(error) => {
                log("warn", &format!("cannot enumerate processes: {error:#}"));
                false
            }
        }
    });

    engine.register_fn("kill_process", |name: &str| -> bool {
        match shell::kill_processes(name) {
            Ok(_) => true,
            Err(error) => {
                log("error", &format!("kill_process {name} failed: {error:#}"));
                false
            }
        }
    });

    engine.register_fn("run_detached", |command: &str| -> bool {
        match std::process::Command::new(command).spawn() {
            Ok(_) => true,
            Err(error) => {
                log("error", &format!("run_detached {command} failed: {error}"));
                false
            }
        }
    });

    engine.register_fn("run_command", |command: &str, args: rhai::Array| -> i64 {
        let args = args
            .into_iter()
            .map(|arg| arg.into_string().unwrap_or_default())
            .collect::<Vec<_>>();
        match std::process::Command::new(command).args(&args).output() {
            Ok(output) => output.status.code().unwrap_or(-1) as i64,
            Err(error) => {
                log("error", &format!("run_command {command} failed: {error}"));
                -1
            }
        }
    });
}
