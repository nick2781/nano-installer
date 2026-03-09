// Configurable task pipeline
//
// Reads install_tasks from config and executes them in order.
// Falls back to default pipeline if not configured.

use crate::common::Result;
use crate::config::InstallerConfig;
use crate::installer::state::InstallState;
use crate::installer::tasks::*;
use crate::resources::RuntimeResources;

/// Task definition from config
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskConfig {
    /// Task type: "extract", "copy_uninstaller", "create_shortcuts", "write_registry"
    #[serde(rename = "type")]
    pub task_type: String,
    /// Optional payload path (for extract task)
    #[serde(default)]
    pub payload: Option<String>,
    /// Optional custom parameters
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
}

/// Task runner that executes tasks from config
pub struct TaskRunner {
    tasks: Vec<TaskConfig>,
    completed_tasks: Vec<String>,
}

impl TaskRunner {
    /// Create from config. If install_tasks is empty, use default pipeline.
    pub fn new(config: &InstallerConfig) -> Self {
        let tasks = if let Some(ref task_configs) = config.install_tasks {
            task_configs.clone()
        } else {
            Self::default_pipeline()
        };

        Self {
            tasks,
            completed_tasks: Vec::new(),
        }
    }

    /// Default task pipeline (matches existing behavior)
    fn default_pipeline() -> Vec<TaskConfig> {
        vec![
            TaskConfig {
                task_type: "extract".to_string(),
                payload: None,
                params: std::collections::HashMap::new(),
            },
            TaskConfig {
                task_type: "copy_uninstaller".to_string(),
                payload: None,
                params: std::collections::HashMap::new(),
            },
            TaskConfig {
                task_type: "create_shortcuts".to_string(),
                payload: None,
                params: std::collections::HashMap::new(),
            },
            TaskConfig {
                task_type: "write_registry".to_string(),
                payload: None,
                params: std::collections::HashMap::new(),
            },
        ]
    }

    /// Execute all tasks in sequence
    pub fn execute(
        &mut self,
        state: &InstallState,
        config: &InstallerConfig,
    ) -> Result<()> {
        let total = self.tasks.len() as f32;

        for (i, task_config) in self.tasks.clone().iter().enumerate() {
            if state.is_cancelled() {
                tracing::info!("Installation cancelled, rolling back...");
                self.rollback(state, config)?;
                return Err(crate::common::Error::InstallationFailed(
                    "Installation cancelled by user".to_string(),
                ));
            }

            let progress_base = (i as f32 / total) * 100.0;
            let progress_end = ((i + 1) as f32 / total) * 100.0;

            tracing::info!("Executing task {}/{}: {}", i + 1, self.tasks.len(), task_config.task_type);

            match self.execute_task(task_config, state, config, progress_base, progress_end) {
                Ok(()) => {
                    self.completed_tasks.push(task_config.task_type.clone());
                }
                Err(e) => {
                    tracing::error!("Task {} failed: {}", task_config.task_type, e);
                    // Rollback completed tasks in reverse order
                    self.rollback(state, config)?;
                    return Err(e);
                }
            }
        }

        // Final progress
        state.update_progress(crate::installer::state::InstallProgress {
            current_step: "Installation complete".to_string(),
            percentage: 100.0,
            ..Default::default()
        });

        Ok(())
    }

    /// Execute a single task
    fn execute_task(
        &self,
        task_config: &TaskConfig,
        state: &InstallState,
        config: &InstallerConfig,
        progress_start: f32,
        progress_end: f32,
    ) -> Result<()> {
        match task_config.task_type.as_str() {
            "extract" => {
                state.update_progress(crate::installer::state::InstallProgress {
                    current_step: "Extracting files...".to_string(),
                    percentage: progress_start,
                    ..Default::default()
                });

                let payload = RuntimeResources::get_payload()
                    .ok_or_else(|| crate::common::Error::InstallationFailed(
                        "Payload not found".to_string()
                    ))?;

                let task = ExtractFilesTask { payload_data: payload };
                task.execute(state)?;

                state.update_progress(crate::installer::state::InstallProgress {
                    current_step: "Files extracted".to_string(),
                    percentage: progress_end,
                    ..Default::default()
                });
            }

            "copy_uninstaller" => {
                if let Some(uninst_data) = RuntimeResources::get_uninstaller() {
                    state.update_progress(crate::installer::state::InstallProgress {
                        current_step: "Installing uninstaller...".to_string(),
                        percentage: progress_start,
                        ..Default::default()
                    });

                    let task = CopyUninstallerTask {
                        uninstaller_data: uninst_data,
                        uninstaller_name: config.output.uninstaller_name.clone(),
                    };
                    task.execute(state)?;
                }
            }

            "create_shortcuts" => {
                state.update_progress(crate::installer::state::InstallProgress {
                    current_step: "Creating shortcuts...".to_string(),
                    percentage: progress_start,
                    ..Default::default()
                });

                let exe_full_path = format!(
                    "{}\\{}",
                    state.install_path(),
                    config.install.exe_name
                );
                let task = CreateShortcutsTask {
                    app_name: config.project.name.clone(),
                    exe_path: exe_full_path,
                };
                task.execute(state)?;
            }

            "write_registry" => {
                state.update_progress(crate::installer::state::InstallProgress {
                    current_step: "Writing registry...".to_string(),
                    percentage: progress_start,
                    ..Default::default()
                });

                let uninst_full_path = format!(
                    "{}\\{}",
                    state.install_path(),
                    config.output.uninstaller_name
                );
                let task = WriteRegistryTask {
                    app_name: config.project.name.clone(),
                    app_version: config.project.version.clone(),
                    install_path: state.install_path().to_string(),
                    publisher: config.project.publisher.clone(),
                    uninstaller_path: uninst_full_path,
                };
                task.execute(state)?;
            }

            "write_uninstall_config" => {
                // Write installer_config.json to install dir for uninst.exe to read
                state.update_progress(crate::installer::state::InstallProgress {
                    current_step: "Writing config...".to_string(),
                    percentage: progress_start,
                    ..Default::default()
                });

                let config_json = serde_json::to_string_pretty(config)
                    .map_err(|e| crate::common::Error::InstallationFailed(
                        format!("Failed to serialize config: {}", e)
                    ))?;
                let install_dir = state.install_path();
                let config_path = std::path::Path::new(&install_dir).join("installer_config.json");
                std::fs::write(&config_path, config_json)
                    .map_err(|e| crate::common::Error::InstallationFailed(
                        format!("Failed to write config: {}", e)
                    ))?;
                tracing::info!("Wrote installer_config.json to {}", config_path.display());
            }

            other => {
                tracing::warn!("Unknown task type: {}, skipping", other);
            }
        }

        Ok(())
    }

    /// Rollback completed tasks in reverse order
    fn rollback(&self, state: &InstallState, _config: &InstallerConfig) -> Result<()> {
        tracing::info!("Rolling back {} completed tasks", self.completed_tasks.len());

        for task_type in self.completed_tasks.iter().rev() {
            match task_type.as_str() {
                "extract" => {
                    // Remove extracted files (best effort)
                    let install_path = state.install_path();
                    tracing::info!("Rollback: cleaning extracted files from {}", install_path);
                    // Don't remove the directory itself, just log the intent
                    // A full rollback would track extracted files
                }
                "create_shortcuts" => {
                    tracing::info!("Rollback: removing shortcuts (not implemented)");
                }
                "write_registry" => {
                    tracing::info!("Rollback: removing registry entries (not implemented)");
                }
                "copy_uninstaller" => {
                    tracing::info!("Rollback: removing uninstaller (not implemented)");
                }
                _ => {}
            }
        }

        Ok(())
    }
}
