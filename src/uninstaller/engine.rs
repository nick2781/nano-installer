// 卸载引擎

use crate::common::{Error, Result};
use crate::resources::UninstallManifest;
use std::path::Path;

/// 卸载引擎
pub struct UninstallEngine {
    manifest: UninstallManifest,
    keep_data: bool,
}

impl UninstallEngine {
    /// 从安装路径加载卸载器
    pub fn from_install_path(install_path: &Path, keep_data: bool) -> Result<Self> {
        let manifest_path = install_path.join("install_manifest.json");
        
        if !manifest_path.exists() {
            return Err(Error::Config(
                "Install manifest not found. Cannot uninstall.".to_string()
            ));
        }
        
        let manifest = UninstallManifest::load(&manifest_path)?;
        
        Ok(Self {
            manifest,
            keep_data,
        })
    }
    
    /// 执行卸载
    pub async fn uninstall(&self) -> Result<()> {
        use crate::logger::{log_step, StepStatus};
        
        tracing::info!("Starting uninstallation from: {:?}", self.manifest.install_path);
        
        // 1. 关闭正在运行的进程
        log_step("Close running processes", StepStatus::Started);
        self.close_running_processes()?;
        log_step("Close running processes", StepStatus::Completed);
        
        // 2. 删除快捷方式
        log_step("Remove shortcuts", StepStatus::Started);
        self.remove_shortcuts()?;
        log_step("Remove shortcuts", StepStatus::Completed);
        
        // 3. 清理注册表
        #[cfg(windows)]
        {
            log_step("Clean registry", StepStatus::Started);
            self.clean_registry()?;
            log_step("Clean registry", StepStatus::Completed);
        }
        
        // 4. 删除文件
        log_step("Remove files", StepStatus::Started);
        self.remove_files()?;
        log_step("Remove files", StepStatus::Completed);
        
        // 5. 删除用户数据（如果不保留）
        if !self.keep_data {
            log_step("Remove user data", StepStatus::Started);
            self.remove_user_data()?;
            log_step("Remove user data", StepStatus::Completed);
        } else {
            log_step("Remove user data", StepStatus::Skipped);
        }
        
        tracing::info!("Uninstallation completed successfully");
        
        Ok(())
    }
    
    /// 关闭正在运行的进程
    fn close_running_processes(&self) -> Result<()> {
        // TODO: 实现进程关闭逻辑
        tracing::info!("Closing running processes...");
        Ok(())
    }
    
    /// 删除快捷方式
    fn remove_shortcuts(&self) -> Result<()> {
        #[cfg(windows)]
        {
            use crate::installer::windows::shortcuts;
            
            for shortcut in &self.manifest.shortcuts {
                if let Err(e) = shortcuts::remove_shortcut(&shortcut.path) {
                    tracing::warn!("Failed to remove shortcut {:?}: {}", shortcut.path, e);
                    // 继续删除其他快捷方式
                }
            }
        }
        
        Ok(())
    }
    
    /// 清理注册表
    #[cfg(windows)]
    fn clean_registry(&self) -> Result<()> {
        use crate::installer::windows::registry;
        
        if let Err(e) = registry::remove_uninstall_entry(&self.manifest.app_name) {
            tracing::warn!("Failed to remove uninstall entry: {}", e);
        }
        
        Ok(())
    }
    
    /// 删除文件
    fn remove_files(&self) -> Result<()> {
        tracing::info!("Removing installation files from: {:?}", self.manifest.install_path);
        
        if self.manifest.install_path.exists() {
            std::fs::remove_dir_all(&self.manifest.install_path)
                .map_err(|e| Error::UninstallationFailed(format!(
                    "Failed to remove installation directory: {}",
                    e
                )))?;
        }
        
        Ok(())
    }
    
    /// 删除用户数据
    fn remove_user_data(&self) -> Result<()> {
        let user_data_dir = crate::common::platform::get_user_data_dir(&self.manifest.app_name)?;
        let user_data_path = Path::new(&user_data_dir);
        
        if user_data_path.exists() {
            tracing::info!("Removing user data from: {:?}", user_data_path);
            std::fs::remove_dir_all(user_data_path)
                .map_err(|e| Error::UninstallationFailed(format!(
                    "Failed to remove user data: {}",
                    e
                )))?;
        }
        
        Ok(())
    }
}

