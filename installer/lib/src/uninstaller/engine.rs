// 卸载引擎

use crate::common::{Error, Result};
use crate::resources::manifest::UninstallManifest;
use std::path::Path;

/// 卸载引擎
pub struct UninstallEngine {
    manifest: UninstallManifest,
    keep_user_data: bool,
}

impl UninstallEngine {
    pub fn new(manifest: UninstallManifest) -> Self {
        Self {
            manifest,
            keep_user_data: false,
        }
    }

    /// 从安装路径创建卸载引擎
    pub fn from_install_path(install_path: &Path, keep_data: bool) -> Result<Self> {
        // 尝试从安装路径读取卸载清单
        let manifest_path = install_path.join("uninstall.json");
        if !manifest_path.exists() {
            return Err(Error::UninstallationFailed(
                "Uninstall manifest not found".to_string()
            ));
        }

        let manifest_content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| Error::UninstallationFailed(format!(
                "Failed to read uninstall manifest: {}", e
            )))?;

        let manifest: UninstallManifest = serde_json::from_str(&manifest_content)
            .map_err(|e| Error::UninstallationFailed(format!(
                "Failed to parse uninstall manifest: {}", e
            )))?;

        let mut engine = Self::new(manifest);
        engine.set_keep_user_data(keep_data);
        Ok(engine)
    }

    /// 设置是否保留用户数据
    pub fn set_keep_user_data(&mut self, keep: bool) {
        self.keep_user_data = keep;
    }

    /// 执行卸载
    pub fn uninstall(&mut self) -> Result<()> {
        tracing::info!("Starting uninstallation process");

        // 1. 删除快捷方式
        self.remove_shortcuts()?;

        // 2. 清理注册表
        self.clean_registry()?;

        // 3. 删除文件
        self.remove_files()?;

        // 4. 删除用户数据（如果用户选择不保留）
        if !self.keep_user_data {
            self.remove_user_data()?;
        }

        tracing::info!("Uninstallation completed successfully");
        Ok(())
    }

    /// 删除快捷方式
    fn remove_shortcuts(&self) -> Result<()> {
        #[cfg(windows)]
        {
            use crate::installer::windows::shortcuts;
            
            for shortcut_path in &self.manifest.shortcuts_to_remove {
                let path = std::path::Path::new(shortcut_path);
                if let Err(e) = shortcuts::remove_shortcut(path) {
                    tracing::warn!("Failed to remove shortcut {:?}: {}", shortcut_path, e);
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
        
        if let Err(e) = registry::remove_uninstall_entry(&self.manifest.product_name) {
            tracing::warn!("Failed to remove uninstall entry: {}", e);
        }
        
        Ok(())
    }

    #[cfg(not(windows))]
    fn clean_registry(&self) -> Result<()> {
        // 非Windows平台不需要清理注册表
        Ok(())
    }

    /// 删除文件
    fn remove_files(&self) -> Result<()> {
        tracing::info!("Removing installation files from: {:?}", self.manifest.install_path);
        
        if std::path::Path::new(&self.manifest.install_path).exists() {
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
        let user_data_dir = crate::common::platform::get_user_data_dir(&self.manifest.product_name)?;
        let user_data_path = Path::new(&user_data_dir);
        
        if user_data_path.exists() {
            tracing::info!("Removing user data from: {:?}", user_data_path);
            std::fs::remove_dir_all(user_data_path)
                .map_err(|e| Error::UninstallationFailed(format!(
                    "Failed to remove user data directory: {}",
                    e
                )))?;
        }
        
        Ok(())
    }

    /// 获取卸载进度（0.0 - 1.0）
    pub fn get_progress(&self) -> f32 {
        // 这里可以根据实际删除的文件数量来计算进度
        // 目前返回一个固定值
        0.5
    }

    /// 获取当前卸载状态
    pub fn get_status(&self) -> String {
        "正在卸载...".to_string()
    }
}

/// 卸载任务
pub struct UninstallTask {
    engine: UninstallEngine,
    progress: f32,
    status: String,
    is_completed: bool,
    error: Option<Error>,
}

impl UninstallTask {
    pub fn new(manifest: UninstallManifest) -> Self {
        Self {
            engine: UninstallEngine::new(manifest),
            progress: 0.0,
            status: "准备卸载...".to_string(),
            is_completed: false,
            error: None,
        }
    }

    pub fn set_keep_user_data(&mut self, keep: bool) {
        self.engine.set_keep_user_data(keep);
    }

    pub fn start(&mut self) {
        // 在后台线程中执行卸载
        let mut engine = std::mem::replace(&mut self.engine, UninstallEngine::new(
            crate::resources::manifest::UninstallManifest {
                version: "".to_string(),
                product_name: "".to_string(),
                install_path: "".to_string(),
                locale: "en-US".to_string(),
                files_to_remove: vec![],
                directories_to_remove: vec![],
                registry_keys_to_remove: vec![],
                shortcuts_to_remove: vec![],
            }
        ));

        // 这里应该使用异步任务或线程池
        // 为了简化，我们直接同步执行
        match engine.uninstall() {
            Ok(_) => {
                self.progress = 1.0;
                self.status = "卸载完成".to_string();
                self.is_completed = true;
            }
            Err(e) => {
                self.error = Some(e);
                self.status = "卸载失败".to_string();
            }
        }

        self.engine = engine;
    }

    pub fn get_progress(&self) -> f32 {
        self.progress
    }

    pub fn get_status(&self) -> &str {
        &self.status
    }

    pub fn is_completed(&self) -> bool {
        self.is_completed
    }

    pub fn get_error(&self) -> Option<&Error> {
        self.error.as_ref()
    }
}