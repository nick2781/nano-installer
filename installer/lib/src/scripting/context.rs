//! Script execution context — shared state between Rhai functions and the UI thread

use crate::config::InstallerConfig;
use crate::installer::state::{InstallState, InstallProgress};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use parking_lot::RwLock;

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
}

impl ScriptContext {
    /// Create context for install mode
    pub fn for_install(state: InstallState, config: InstallerConfig) -> Self {
        let install_path = state.install_path().to_string();
        let config_json = serde_json::to_value(&config).unwrap_or_default();
        Self {
            install_state: Some(state),
            config,
            install_path: Arc::new(RwLock::new(install_path)),
            config_json: Arc::new(config_json),
            uninstall_progress: Arc::new(RwLock::new(0.0)),
            uninstall_status: Arc::new(RwLock::new(String::new())),
            uninstall_finished: Arc::new(AtomicBool::new(false)),
            checkbox_values: Arc::new(RwLock::new(std::collections::HashMap::new())),
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
        self.install_state.as_ref().map_or(false, |s| s.is_cancelled())
    }

    /// Get install path
    pub fn get_install_path(&self) -> String {
        self.install_path.read().clone()
    }
}
