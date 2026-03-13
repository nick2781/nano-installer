// Configurable task pipeline
//
// Reads install_tasks from config and executes them in order.
// Falls back to default pipeline if not configured.

use crate::common::{Error, Result};
use crate::config::InstallerConfig;
use crate::installer::artifacts::{snapshot_install_tree, InstallArtifacts};
use crate::installer::state::InstallState;
use crate::installer::tasks::*;
use crate::resources::RuntimeResources;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    pub params: HashMap<String, String>,
}

/// Task runner that executes tasks from config
pub struct TaskRunner {
    tasks: Vec<TaskConfig>,
    artifacts: InstallArtifacts,
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
            artifacts: InstallArtifacts::default(),
        }
    }

    /// Default task pipeline (matches existing behavior)
    fn default_pipeline() -> Vec<TaskConfig> {
        vec![
            TaskConfig {
                task_type: "extract".to_string(),
                payload: None,
                params: HashMap::new(),
            },
            TaskConfig {
                task_type: "copy_uninstaller".to_string(),
                payload: None,
                params: HashMap::new(),
            },
            TaskConfig {
                task_type: "create_shortcuts".to_string(),
                payload: None,
                params: HashMap::new(),
            },
            TaskConfig {
                task_type: "write_registry".to_string(),
                payload: None,
                params: HashMap::new(),
            },
        ]
    }

    /// Execute all tasks in sequence
    pub fn execute(&mut self, state: &InstallState, config: &InstallerConfig) -> Result<()> {
        self.artifacts = InstallArtifacts::default();
        self.artifacts
            .capture_existing_install(Path::new(&state.install_path()))?;
        let total = self.tasks.len() as f32;

        for (i, task_config) in self.tasks.clone().iter().enumerate() {
            if state.is_cancelled() {
                tracing::info!("Installation cancelled, rolling back...");
                self.rollback(state)?;
                return Err(Error::InstallationFailed(
                    "Installation cancelled by user".to_string(),
                ));
            }

            let progress_base = (i as f32 / total) * 100.0;
            let progress_end = ((i + 1) as f32 / total) * 100.0;

            tracing::info!(
                "Executing task {}/{}: {}",
                i + 1,
                self.tasks.len(),
                task_config.task_type
            );

            match self.execute_task(task_config, state, config, progress_base, progress_end) {
                Ok(()) => {}
                Err(e) => {
                    tracing::error!("Task {} failed: {}", task_config.task_type, e);
                    self.rollback(state)?;
                    return Err(e);
                }
            }
        }

        self.artifacts.write_uninstall_manifest(config, state)?;

        state.update_progress(crate::installer::state::InstallProgress {
            current_step: "Installation complete".to_string(),
            percentage: 100.0,
            ..Default::default()
        });

        Ok(())
    }

    /// Execute a single task
    fn execute_task(
        &mut self,
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

                let install_root = PathBuf::from(state.install_path());
                let (before_files, before_dirs) = snapshot_install_tree(&install_root)?;

                let payload = RuntimeResources::get_payload()
                    .ok_or_else(|| Error::InstallationFailed("Payload not found".to_string()))?;

                let task = ExtractFilesTask {
                    payload_data: payload,
                };
                task.execute(state)?;

                let (after_files, after_dirs) = snapshot_install_tree(&install_root)?;
                for file in after_files.difference(&before_files) {
                    self.artifacts.record_file(file.clone());
                }
                for dir in after_dirs.difference(&before_dirs) {
                    self.artifacts.record_directory(dir.clone());
                }

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
                    self.artifacts.record_file(
                        PathBuf::from(state.install_path()).join(&config.output.uninstaller_name),
                    );
                }
            }

            "create_shortcuts" => {
                state.update_progress(crate::installer::state::InstallProgress {
                    current_step: "Creating shortcuts...".to_string(),
                    percentage: progress_start,
                    ..Default::default()
                });

                let exe_full_path =
                    format!("{}\\{}", state.install_path(), config.install.exe_name);
                let task = CreateShortcutsTask {
                    app_name: config.project.name.clone(),
                    start_menu_folder: config.shortcuts.start_menu_folder.clone(),
                    exe_path: exe_full_path,
                };
                task.execute(state)?;

                #[cfg(windows)]
                {
                    use crate::installer::windows::shortcuts;
                    if state.create_desktop_shortcut() {
                        self.artifacts
                            .record_shortcut(shortcuts::get_desktop_shortcut_path(
                                &config.project.name,
                            )?);
                    }
                    if state.create_start_menu_shortcut() {
                        self.artifacts
                            .record_directory(shortcuts::get_start_menu_folder_path(
                                &config.shortcuts.start_menu_folder,
                            )?);
                        self.artifacts
                            .record_shortcut(shortcuts::get_start_menu_shortcut_path(
                                &config.project.name,
                                &config.shortcuts.start_menu_folder,
                            )?);
                    }
                }
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
                    install_path_key: config.registry.install_path_key.clone(),
                    uninstall_key: config.registry.uninstall_key.clone(),
                    autostart_key: if config.autostart.enabled && state.autostart_enabled() {
                        Some(config.autostart.registry_key.clone())
                    } else {
                        None
                    },
                    autostart_value_name: if config.autostart.enabled && state.autostart_enabled() {
                        Some(config.autostart.registry_value_name.clone())
                    } else {
                        None
                    },
                    exe_name: config.install.exe_name.clone(),
                };
                task.execute(state)?;

                self.artifacts
                    .record_registry_key(config.registry.install_path_key.clone());
                self.artifacts
                    .record_registry_key(config.registry.uninstall_key.clone());
                if config.autostart.enabled && state.autostart_enabled() {
                    self.artifacts.record_registry_value(
                        config.autostart.registry_key.clone(),
                        config.autostart.registry_value_name.clone(),
                    );
                }
            }

            "write_uninstall_config" => {
                state.update_progress(crate::installer::state::InstallProgress {
                    current_step: "Writing config...".to_string(),
                    percentage: progress_start,
                    ..Default::default()
                });

                let config_json = serde_json::to_string_pretty(config).map_err(|e| {
                    Error::InstallationFailed(format!("Failed to serialize config: {}", e))
                })?;
                let install_dir = state.install_path();
                let config_path = Path::new(&install_dir).join("installer_config.json");
                std::fs::write(&config_path, config_json).map_err(|e| {
                    Error::InstallationFailed(format!("Failed to write config: {}", e))
                })?;
                self.artifacts.record_file(config_path.clone());
                tracing::info!("Wrote installer_config.json to {}", config_path.display());
            }

            other => {
                tracing::warn!("Unknown task type: {}, skipping", other);
            }
        }

        Ok(())
    }

    /// Rollback completed tasks in reverse order
    fn rollback(&mut self, state: &InstallState) -> Result<()> {
        tracing::info!("Rolling back installation artifacts");
        let result = self.artifacts.rollback(Path::new(&state.install_path()));
        self.artifacts.cleanup_backup();
        result
    }
}
