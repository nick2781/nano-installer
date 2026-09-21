//! Process primitives: detect and stop a running product, and run a program a
//! project's own step needs.

use rhai::{Array, Dynamic, Engine, Map};

use super::context::{log, ScriptContext};
use crate::shell;

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
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
        match shell::hidden_command(command).spawn() {
            Ok(_) => true,
            Err(error) => {
                log("error", &format!("run_detached {command} failed: {error}"));
                false
            }
        }
    });

    let c = context.clone();
    engine.register_fn("run_command", move |command: &str, args: Array| -> i64 {
        let args = arguments(args);
        let cancelled = c.cancellation().clone();
        match shell::run_captured(command, &args, &|| cancelled.requested()) {
            Ok(output) => output.code,
            Err(error) => {
                log("error", &format!("run_command {command} failed: {error}"));
                -1
            }
        }
    });

    let c = context.clone();
    engine.register_fn(
        "run_command_output",
        move |command: &str, args: Array| -> Map {
            let args = arguments(args);
            let cancelled = c.cancellation().clone();
            match shell::run_captured(command, &args, &|| cancelled.requested()) {
                Ok(output) => report(output.code, &output.stdout, &output.stderr),
                Err(error) => {
                    log(
                        "error",
                        &format!("run_command_output {command} failed: {error}"),
                    );
                    // The program could not be started at all, which is the one
                    // failure a script has to be able to tell from a program
                    // that ran and said no: it carries no exit code, and the
                    // reason is what a person needs to read.
                    report(-1, "", &format!("{error:#}"))
                }
            }
        },
    );
}

/// What `run_command_output` hands back: the code, and what the program wrote.
fn report(code: i64, stdout: &str, stderr: &str) -> Map {
    let mut output = Map::new();
    output.insert("code".into(), Dynamic::from(code));
    output.insert("stdout".into(), Dynamic::from(stdout.to_string()));
    output.insert("stderr".into(), Dynamic::from(stderr.to_string()));
    output
}

/// The arguments a script passed, as the list a command takes.
fn arguments(args: Array) -> Vec<String> {
    args.into_iter()
        .map(|arg| arg.into_string().unwrap_or_default())
        .collect()
}
