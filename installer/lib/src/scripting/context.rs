//! Script execution context — shared state between Rhai functions and the UI thread

use crate::common::{Error, Result};
use crate::config::InstallerConfig;
use crate::installer::artifacts::{snapshot_install_tree, InstallArtifacts};
use crate::installer::state::{InstallProgress, InstallState};
use parking_lot::RwLock;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Shared context accessible by all script API functions.
/// Cloneable (all fields are Arc-wrapped or Clone), safe to capture in closures.
#[derive(Clone)]
pub struct ScriptContext {
    /// Install state (progress, cancel, preferences — Arc<RwLock> internally)
    pub install_state: Option<InstallState>,
    /// Installer configuration (read-only during script execution)
    pub config: InstallerConfig,
    /// Installation path
    pub install_path: Arc<RwLock<String>>,
    /// Config as JSON value (for get_config_value traversal)
    pub config_json: Arc<serde_json::Value>,
    /// Uninstall mode progress (shared with UI thread)
    pub uninstall_progress: Arc<RwLock<f32>>,
    pub uninstall_status: Arc<RwLock<String>>,
    pub uninstall_finished: Arc<AtomicBool>,
    /// Checkbox values (populated from UI before script runs)
    pub checkbox_values: Arc<RwLock<std::collections::HashMap<String, bool>>>,
    /// Tracked install artifacts for manifest/rollback
    install_artifacts: Arc<RwLock<InstallArtifacts>>,
    /// Baseline tree before script install starts
    install_tree_before: Arc<(BTreeSet<PathBuf>, BTreeSet<PathBuf>)>,
    /// Whether manifest-driven uninstall was already invoked explicitly
    tracked_uninstall_invoked: Arc<AtomicBool>,
}

impl ScriptContext {
    /// Create context for install mode
    pub fn for_install(state: InstallState, config: InstallerConfig) -> Self {
        let install_path = state.install_path().to_string();
        let config_json = serde_json::to_value(&config).unwrap_or_default();
        let baseline = snapshot_install_tree(Path::new(&install_path)).unwrap_or_default();
        Self {
            install_artifacts: {
                let mut artifacts = InstallArtifacts::default();
                let _ = artifacts.capture_existing_install(Path::new(&install_path));
                Arc::new(RwLock::new(artifacts))
            },
            install_state: Some(state),
            config,
            install_path: Arc::new(RwLock::new(install_path)),
            config_json: Arc::new(config_json),
            uninstall_progress: Arc::new(RwLock::new(0.0)),
            uninstall_status: Arc::new(RwLock::new(String::new())),
            uninstall_finished: Arc::new(AtomicBool::new(false)),
            checkbox_values: Arc::new(RwLock::new(std::collections::HashMap::new())),
            install_tree_before: Arc::new(baseline),
            tracked_uninstall_invoked: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create context for uninstall mode
    pub fn for_uninstall(
        config: InstallerConfig,
        install_path: String,
        progress: Arc<RwLock<f32>>,
        status: Arc<RwLock<String>>,
        finished: Arc<AtomicBool>,
        reserve_data: bool,
    ) -> Self {
        let config_json = serde_json::to_value(&config).unwrap_or_default();
        let mut checkboxes = std::collections::HashMap::new();
        checkboxes.insert("keep_data".to_string(), reserve_data);
        Self {
            install_state: None,
            config,
            install_path: Arc::new(RwLock::new(install_path)),
            config_json: Arc::new(config_json),
            uninstall_progress: progress,
            uninstall_status: status,
            uninstall_finished: finished,
            checkbox_values: Arc::new(RwLock::new(checkboxes)),
            install_artifacts: Arc::new(RwLock::new(InstallArtifacts::default())),
            install_tree_before: Arc::new(Default::default()),
            tracked_uninstall_invoked: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Update progress (works for both install and uninstall)
    pub fn set_progress(&self, percentage: f32) {
        if let Some(ref state) = self.install_state {
            state.update_progress(InstallProgress {
                percentage,
                current_step: self.uninstall_status.read().clone(),
                ..Default::default()
            });
        } else {
            *self.uninstall_progress.write() = percentage / 100.0;
        }
    }

    /// Update status text
    pub fn set_status(&self, text: &str) {
        if let Some(ref state) = self.install_state {
            let progress = state.progress();
            state.update_progress(InstallProgress {
                percentage: progress.percentage,
                current_step: text.to_string(),
                ..Default::default()
            });
        }
        *self.uninstall_status.write() = text.to_string();
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.install_state
            .as_ref()
            .map_or(false, |s| s.is_cancelled())
    }

    /// Get install path
    pub fn get_install_path(&self) -> String {
        self.install_path.read().clone()
    }

    pub fn record_install_tree_delta(&self) -> Result<()> {
        let install_path = self.get_install_path();
        let root = Path::new(&install_path);
        let (after_files, after_dirs) = snapshot_install_tree(root)?;
        let mut artifacts = self.install_artifacts.write();

        for file in after_files.difference(&self.install_tree_before.0) {
            artifacts.record_file(file.clone());
        }
        for dir in after_dirs.difference(&self.install_tree_before.1) {
            artifacts.record_directory(dir.clone());
        }

        Ok(())
    }

    pub fn record_file(&self, path: impl Into<PathBuf>) {
        self.install_artifacts.write().record_file(path.into());
    }

    pub fn record_directory(&self, path: impl Into<PathBuf>) {
        self.install_artifacts.write().record_directory(path.into());
    }

    pub fn record_shortcut(&self, path: impl Into<PathBuf>) {
        self.install_artifacts.write().record_shortcut(path.into());
    }

    pub fn record_registry_key(&self, path: impl Into<String>) {
        self.install_artifacts.write().record_registry_key(path);
    }

    pub fn record_registry_value(
        &self,
        key_path: impl Into<String>,
        value_name: impl Into<String>,
    ) {
        self.install_artifacts
            .write()
            .record_registry_value(key_path, value_name);
    }

    pub fn finalize_install(&self) -> Result<()> {
        let state = self.install_state.as_ref().ok_or_else(|| {
            Error::InstallationFailed("Script install context missing install state".to_string())
        })?;
        self.record_install_tree_delta()?;
        self.install_artifacts
            .write()
            .write_uninstall_manifest(&self.config, state)
    }

    pub fn rollback_install(&self) -> Result<()> {
        let state = self.install_state.as_ref().ok_or_else(|| {
            Error::InstallationFailed("Script install context missing install state".to_string())
        })?;
        let mut artifacts = self.install_artifacts.write();
        let result = artifacts.rollback(Path::new(&state.install_path()));
        artifacts.cleanup_backup();
        result
    }

    pub fn should_track_install_path(&self, path: &Path) -> bool {
        let install_root = PathBuf::from(self.get_install_path());
        path.starts_with(&install_root)
    }

    pub fn record_registry_write(&self, key: &str, name: &str) {
        if is_shared_registry_key(key) {
            self.record_registry_value(key.to_string(), name.to_string());
        } else {
            self.record_registry_key(key.to_string());
        }
    }

    pub fn mark_tracked_uninstall_invoked(&self) {
        self.tracked_uninstall_invoked.store(true, Ordering::SeqCst);
    }

    pub fn tracked_uninstall_invoked(&self) -> bool {
        self.tracked_uninstall_invoked.load(Ordering::SeqCst)
    }

    pub fn run_manifest_uninstall(&self, start_pct: f32, end_pct: f32) -> Result<bool> {
        let install_path = self.get_install_path();
        let manifest_path = Path::new(&install_path).join("uninstall.json");
        if !manifest_path.exists() {
            return Ok(false);
        }

        self.mark_tracked_uninstall_invoked();
        let keep_data = self
            .checkbox_values
            .read()
            .get("keep_data")
            .copied()
            .unwrap_or(false);
        let install_dir = Path::new(&install_path);
        let progress_span = (end_pct - start_pct).max(0.0);

        let mut engine =
            crate::uninstaller::UninstallEngine::from_install_path(install_dir, keep_data)?;
        engine.uninstall_with_progress(|pct, _step| {
            let mapped = start_pct + (pct * progress_span);
            self.set_progress(mapped);
        })?;

        Ok(true)
    }
}

fn is_shared_registry_key(key: &str) -> bool {
    let key_upper = key.to_ascii_uppercase();
    key_upper.contains("\\SOFTWARE\\MICROSOFT\\WINDOWS\\CURRENTVERSION\\RUN")
}
