// 安装任务

use crate::common::Result;
use crate::installer::state::InstallState;

/// 安装任务特征
pub trait InstallTask: Send + Sync {
    /// 任务名称
    fn name(&self) -> &str;

    /// 执行任务
    fn execute(&self, state: &InstallState) -> Result<()>;

    /// 回滚任务（如果安装失败）
    fn rollback(&self, _state: &InstallState) -> Result<()> {
        // 默认不执行回滚
        Ok(())
    }
}

/// 提取文件任务
pub struct ExtractFilesTask {
    pub payload_data: Vec<u8>,
}

impl InstallTask for ExtractFilesTask {
    fn name(&self) -> &str {
        "Extract Files"
    }

    fn execute(&self, state: &InstallState) -> Result<()> {
        use crate::resources::PayloadExt;
        use std::path::Path;

        let install_path = state.install_path();
        tracing::info!("Extracting files to: {}", install_path);

        // 更新进度
        state.update_progress(crate::installer::state::InstallProgress {
            current_step: "Extracting files...".to_string(),
            percentage: 10.0,
            ..Default::default()
        });

        // 解压文件
        PayloadExt::extract_7z_to_dir(&self.payload_data, Path::new(&install_path))?;

        // 更新进度
        state.update_progress(crate::installer::state::InstallProgress {
            current_step: "Files extracted".to_string(),
            percentage: 50.0,
            ..Default::default()
        });

        Ok(())
    }
}

/// 创建快捷方式任务
pub struct CreateShortcutsTask {
    pub app_name: String,
    pub exe_path: String,
}

impl InstallTask for CreateShortcutsTask {
    fn name(&self) -> &str {
        "Create Shortcuts"
    }

    fn execute(&self, state: &InstallState) -> Result<()> {
        #[cfg(windows)]
        {
            use crate::installer::windows::shortcuts;

            tracing::info!("Creating shortcuts");

            state.update_progress(crate::installer::state::InstallProgress {
                current_step: "Creating shortcuts...".to_string(),
                percentage: 60.0,
                ..Default::default()
            });

            if state.create_desktop_shortcut() {
                shortcuts::create_desktop_shortcut(&self.app_name, &self.exe_path)?;
            }

            if state.create_start_menu_shortcut() {
                shortcuts::create_start_menu_shortcut(&self.app_name, &self.exe_path)?;
            }

            state.update_progress(crate::installer::state::InstallProgress {
                current_step: "Shortcuts created".to_string(),
                percentage: 70.0,
                ..Default::default()
            });
        }

        Ok(())
    }
}

/// 写入注册表任务
pub struct WriteRegistryTask {
    pub app_name: String,
    pub app_version: String,
    pub install_path: String,
    pub publisher: String,
    pub uninstaller_path: String,
}

impl InstallTask for WriteRegistryTask {
    fn name(&self) -> &str {
        "Write Registry"
    }

    fn execute(&self, state: &InstallState) -> Result<()> {
        #[cfg(windows)]
        {
            use crate::installer::windows::registry;

            tracing::info!("Writing registry entries");

            state.update_progress(crate::installer::state::InstallProgress {
                current_step: "Writing registry...".to_string(),
                percentage: 80.0,
                ..Default::default()
            });

            registry::write_uninstall_entry(
                &self.app_name,
                &self.app_version,
                &self.install_path,
                &self.publisher,
                &self.uninstaller_path,
            )?;

            state.update_progress(crate::installer::state::InstallProgress {
                current_step: "Registry updated".to_string(),
                percentage: 90.0,
                ..Default::default()
            });
        }

        Ok(())
    }
}

/// 复制卸载器到安装目录的任务
pub struct CopyUninstallerTask {
    pub uninstaller_data: Vec<u8>,
    pub uninstaller_name: String,
}

impl InstallTask for CopyUninstallerTask {
    fn name(&self) -> &str {
        "Copy Uninstaller"
    }

    fn execute(&self, state: &InstallState) -> Result<()> {
        let install_path = state.install_path();
        let uninst_path = std::path::Path::new(&install_path).join(&self.uninstaller_name);
        
        tracing::info!("Copying uninstaller to: {}", uninst_path.display());
        
        state.update_progress(crate::installer::state::InstallProgress {
            current_step: "Installing uninstaller...".to_string(),
            percentage: 75.0,
            ..Default::default()
        });
        
        std::fs::write(&uninst_path, &self.uninstaller_data)
            .map_err(|e| crate::common::Error::InstallationFailed(
                format!("Failed to write uninstaller: {}", e)
            ))?;
        
        tracing::info!("Uninstaller copied successfully");
        
        Ok(())
    }
}
