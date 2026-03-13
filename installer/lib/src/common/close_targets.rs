use crate::common::process::ProcessDetector;
use crate::common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloseTargetKind {
    Process,
    Service,
}

fn default_close_target_kind() -> CloseTargetKind {
    CloseTargetKind::Process
}

fn default_force_close() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloseTarget {
    pub name: String,
    #[serde(default = "default_close_target_kind")]
    pub kind: CloseTargetKind,
    #[serde(default)]
    pub detect_on_install: bool,
    #[serde(default)]
    pub close_on_install: bool,
    #[serde(default)]
    pub close_on_uninstall: bool,
    #[serde(default = "default_force_close")]
    pub force: bool,
}

impl CloseTarget {
    pub fn legacy_process(
        name: impl Into<String>,
        detect_on_install: bool,
        close_on_install: bool,
        close_on_uninstall: bool,
    ) -> Option<Self> {
        let name = name.into();
        if name.trim().is_empty() || !(detect_on_install || close_on_install || close_on_uninstall)
        {
            return None;
        }

        Some(Self {
            name,
            kind: CloseTargetKind::Process,
            detect_on_install,
            close_on_install,
            close_on_uninstall,
            force: true,
        })
    }

    pub fn should_check_on_install(&self) -> bool {
        self.detect_on_install || self.close_on_install
    }

    pub fn should_close_on_uninstall(&self) -> bool {
        self.close_on_uninstall
    }

    pub fn display_name(&self) -> String {
        match self.kind {
            CloseTargetKind::Process => format!("process {}", self.name),
            CloseTargetKind::Service => format!("service {}", self.name),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CloseExecutionReport {
    pub blocking_targets: Vec<String>,
    pub closed_targets: Vec<String>,
}

impl CloseExecutionReport {
    pub fn blocking_message(&self) -> Option<String> {
        if self.blocking_targets.is_empty() {
            None
        } else {
            Some(format!(
                "The following targets are still running and must be closed first: {}",
                self.blocking_targets.join(", ")
            ))
        }
    }
}

pub fn effective_close_targets(
    explicit_targets: &[CloseTarget],
    legacy_exe_name: &str,
    legacy_detect_on_install: bool,
    legacy_close_on_install: bool,
    legacy_close_on_uninstall: bool,
) -> Vec<CloseTarget> {
    if !explicit_targets.is_empty() {
        return explicit_targets.to_vec();
    }

    CloseTarget::legacy_process(
        legacy_exe_name,
        legacy_detect_on_install,
        legacy_close_on_install,
        legacy_close_on_uninstall,
    )
    .into_iter()
    .collect()
}

pub fn prepare_install_close_targets(targets: &[CloseTarget]) -> Result<CloseExecutionReport> {
    let mut report = CloseExecutionReport::default();

    for target in targets
        .iter()
        .filter(|target| target.should_check_on_install())
    {
        if !is_target_running(target)? {
            continue;
        }

        if target.close_on_install {
            close_target(target)?;
            report.closed_targets.push(target.display_name());
        } else {
            report.blocking_targets.push(target.display_name());
        }
    }

    Ok(report)
}

pub fn close_uninstall_targets(targets: &[CloseTarget]) -> Result<CloseExecutionReport> {
    let mut report = CloseExecutionReport::default();

    for target in targets
        .iter()
        .filter(|target| target.should_close_on_uninstall())
    {
        if !is_target_running(target)? {
            continue;
        }

        close_target(target)?;
        report.closed_targets.push(target.display_name());
    }

    Ok(report)
}

fn close_target(target: &CloseTarget) -> Result<()> {
    match target.kind {
        CloseTargetKind::Process => close_process_target(target),
        CloseTargetKind::Service => close_service_target(target),
    }
}

fn is_target_running(target: &CloseTarget) -> Result<bool> {
    match target.kind {
        CloseTargetKind::Process => {
            let detector = ProcessDetector::new(vec![target.name.clone()]);
            detector.is_target_running()
        }
        CloseTargetKind::Service => is_service_running(&target.name),
    }
}

fn close_process_target(target: &CloseTarget) -> Result<()> {
    let detector = ProcessDetector::new(vec![target.name.clone()]);
    if target.force {
        detector.terminate_target_processes()?;
    } else {
        detector.request_target_processes_exit()?;
    }
    wait_until_stopped(target, Duration::from_secs(5))
}

fn close_service_target(target: &CloseTarget) -> Result<()> {
    stop_service(&target.name)?;
    wait_until_stopped(target, Duration::from_secs(15))
}

fn wait_until_stopped(target: &CloseTarget, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !is_target_running(target)? {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }

    Err(Error::ProcessTerminationFailed(format!(
        "Timed out while stopping {}",
        target.display_name()
    )))
}

#[cfg(windows)]
fn is_service_running(name: &str) -> Result<bool> {
    let output = Command::new("sc")
        .args(["query", name])
        .output()
        .map_err(|e| {
            Error::ProcessDetectionFailed(format!("Failed to query service {}: {}", name, e))
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr).to_ascii_uppercase();

    if !output.status.success() {
        if combined.contains("FAILED 1060") || combined.contains("DOES NOT EXIST") {
            return Ok(false);
        }
        return Err(Error::ProcessDetectionFailed(format!(
            "Failed to query service {}: {}",
            name,
            combined.trim()
        )));
    }

    Ok(combined.contains("STATE") && combined.contains("RUNNING"))
}

#[cfg(not(windows))]
fn is_service_running(name: &str) -> Result<bool> {
    Err(Error::ProcessDetectionFailed(format!(
        "Service detection is not supported on this platform: {}",
        name
    )))
}

#[cfg(windows)]
fn stop_service(name: &str) -> Result<()> {
    let output = Command::new("sc")
        .args(["stop", name])
        .output()
        .map_err(|e| {
            Error::ProcessTerminationFailed(format!("Failed to stop service {}: {}", name, e))
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr).to_ascii_uppercase();

    if !output.status.success()
        && !combined.contains("FAILED 1062")
        && !combined.contains("NOT_STARTED")
        && !combined.contains("FAILED 1060")
        && !combined.contains("DOES NOT EXIST")
    {
        return Err(Error::ProcessTerminationFailed(format!(
            "Failed to stop service {}: {}",
            name,
            combined.trim()
        )));
    }

    Ok(())
}

#[cfg(not(windows))]
fn stop_service(name: &str) -> Result<()> {
    Err(Error::ProcessTerminationFailed(format!(
        "Service control is not supported on this platform: {}",
        name
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_targets_override_legacy_flags() {
        let explicit = vec![CloseTarget {
            name: "TapTapService".to_string(),
            kind: CloseTargetKind::Service,
            detect_on_install: true,
            close_on_install: true,
            close_on_uninstall: true,
            force: true,
        }];

        let targets = effective_close_targets(&explicit, "TapTap.exe", true, false, true);
        assert_eq!(targets, explicit);
    }

    #[test]
    fn legacy_process_target_is_generated_when_needed() {
        let targets = effective_close_targets(&[], "TapTap.exe", true, false, true);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "TapTap.exe");
        assert_eq!(targets[0].kind, CloseTargetKind::Process);
        assert!(targets[0].detect_on_install);
        assert!(!targets[0].close_on_install);
        assert!(targets[0].close_on_uninstall);
    }

    #[cfg(windows)]
    #[test]
    fn install_close_targets_can_terminate_a_real_process() {
        use tempfile::tempdir;

        let temp = tempdir().unwrap();
        let exe_path = temp.path().join("nano_close_target_test.exe");
        std::fs::copy(r"C:\Windows\System32\cmd.exe", &exe_path).unwrap();

        let mut child = Command::new(&exe_path)
            .args(["/C", "ping", "127.0.0.1", "-n", "30"])
            .spawn()
            .unwrap();

        thread::sleep(Duration::from_millis(600));

        let report = prepare_install_close_targets(&[CloseTarget {
            name: "nano_close_target_test.exe".to_string(),
            kind: CloseTargetKind::Process,
            detect_on_install: true,
            close_on_install: true,
            close_on_uninstall: false,
            force: true,
        }])
        .unwrap();

        assert!(report.blocking_targets.is_empty());
        assert_eq!(
            report.closed_targets,
            vec!["process nano_close_target_test.exe".to_string()]
        );

        for _ in 0..20 {
            if child.try_wait().unwrap().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(200));
        }

        let _ = child.kill();
        panic!("close target test process was not terminated");
    }
}
