//! Process API — detect, kill, run processes

use super::context::ScriptContext;
use rhai::Engine;

pub fn register(engine: &mut Engine, _ctx: ScriptContext) {
    // is_process_running("TapTap.exe") -> bool
    engine.register_fn("is_process_running", |name: &str| -> bool {
        let detector = crate::common::process::ProcessDetector::new(vec![name.to_string()]);
        detector.is_target_running().unwrap_or(false)
    });

    // kill_process("TapTap.exe") -> bool
    engine.register_fn("kill_process", |name: &str| -> bool {
        let detector = crate::common::process::ProcessDetector::new(vec![name.to_string()]);
        if let Ok(true) = detector.is_target_running() {
            match detector.terminate_target_processes() {
                Ok(()) => {
                    tracing::info!("[script] Killed process: {}", name);
                    true
                }
                Err(e) => {
                    tracing::error!("[script] kill_process {} failed: {}", name, e);
                    false
                }
            }
        } else {
            true // Not running, so "kill" succeeded
        }
    });

    // run_command("cmd", ["/C", "echo hello"]) -> i64 exit code
    engine.register_fn("run_command", |exe: &str, args: rhai::Array| -> i64 {
        let args_str: Vec<String> = args
            .into_iter()
            .map(|a| a.into_string().unwrap_or_default())
            .collect();
        match std::process::Command::new(exe).args(&args_str).output() {
            Ok(output) => output.status.code().unwrap_or(-1) as i64,
            Err(e) => {
                tracing::error!("[script] run_command {} failed: {}", exe, e);
                -1
            }
        }
    });

    // run_detached("C:\\path\\to\\app.exe") — fire and forget
    engine.register_fn("run_detached", |exe: &str| -> bool {
        match std::process::Command::new(exe).spawn() {
            Ok(_) => true,
            Err(e) => {
                tracing::error!("[script] run_detached {} failed: {}", exe, e);
                false
            }
        }
    });
}
