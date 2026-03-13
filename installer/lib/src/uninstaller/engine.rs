// 卸载引擎

use crate::common::close_targets::close_uninstall_targets;
use crate::common::{Error, Result};
use crate::resources::manifest::UninstallManifest;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UninstallStep {
    CloseProcesses,
    RemoveShortcuts,
    CleanRegistry,
    RemoveUserData,
    RemoveFiles,
    Finish,
}

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
        let manifest =
            UninstallManifest::load(&install_path.join("uninstall.json")).map_err(|e| {
                Error::UninstallationFailed(format!("Failed to load uninstall manifest: {}", e))
            })?;

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
        self.uninstall_with_progress(|_, _| {})
    }

    pub fn uninstall_with_progress<F>(&mut self, mut progress: F) -> Result<()>
    where
        F: FnMut(f32, UninstallStep),
    {
        tracing::info!("Starting uninstallation process");

        progress(0.05, UninstallStep::CloseProcesses);
        self.close_processes();

        progress(0.15, UninstallStep::RemoveShortcuts);
        self.remove_shortcuts()?;

        progress(0.30, UninstallStep::CleanRegistry);
        self.clean_registry()?;

        if !self.keep_user_data {
            progress(0.45, UninstallStep::RemoveUserData);
            self.remove_user_data()?;
        }

        progress(0.60, UninstallStep::RemoveFiles);
        self.remove_files()?;

        progress(0.95, UninstallStep::Finish);
        self.remove_directories()?;

        tracing::info!("Uninstallation completed successfully");
        progress(1.0, UninstallStep::Finish);
        Ok(())
    }

    fn close_processes(&self) {
        match close_uninstall_targets(&self.manifest.close_targets) {
            Ok(report) => {
                if !report.closed_targets.is_empty() {
                    tracing::info!(
                        "Closed uninstall targets: {}",
                        report.closed_targets.join(", ")
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to close uninstall targets: {}", e);
            }
        }
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
                }
            }
        }

        Ok(())
    }

    /// 清理注册表
    #[cfg(windows)]
    fn clean_registry(&self) -> Result<()> {
        use crate::installer::windows::registry;

        for value in &self.manifest.registry_values_to_remove {
            if let Err(e) = registry::delete_value(&value.key_path, &value.value_name) {
                tracing::warn!(
                    "Failed to remove registry value {}\\{}: {}",
                    value.key_path,
                    value.value_name,
                    e
                );
            }
        }

        for key_path in &self.manifest.registry_keys_to_remove {
            if let Err(e) = registry::delete_key(key_path) {
                tracing::warn!("Failed to remove registry key {}: {}", key_path, e);
            }
        }

        Ok(())
    }

    #[cfg(not(windows))]
    fn clean_registry(&self) -> Result<()> {
        Ok(())
    }

    /// 删除文件
    fn remove_files(&self) -> Result<()> {
        let current_exe = std::env::current_exe().ok();
        let install_root = PathBuf::from(&self.manifest.install_path);
        let manifest_path = PathBuf::from(&self.manifest.install_path).join("uninstall.json");

        for file_path in &self.manifest.files_to_remove {
            let path = Path::new(file_path);
            if !path.exists() {
                continue;
            }
            if current_exe.as_ref().is_some_and(|exe| exe == path) {
                continue;
            }

            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(e)
                    if should_defer_file_cleanup(
                        path,
                        &install_root,
                        current_exe.as_deref(),
                        &manifest_path,
                    ) =>
                {
                    tracing::warn!(
                        "Deferring removal of {} until uninstall process exits: {}",
                        path.display(),
                        e
                    );
                }
                Err(e) => {
                    return Err(Error::UninstallationFailed(format!(
                        "Failed to remove file {}: {}",
                        path.display(),
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    fn remove_directories(&self) -> Result<()> {
        let mut dirs: Vec<_> = self
            .manifest
            .directories_to_remove
            .iter()
            .map(PathBuf::from)
            .collect();
        dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));

        for dir in dirs {
            if !dir.exists() {
                continue;
            }
            match std::fs::remove_dir(&dir) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
                Err(e) => tracing::warn!("Failed to remove directory {}: {}", dir.display(), e),
            }
        }

        Ok(())
    }

    /// 删除用户数据
    fn remove_user_data(&self) -> Result<()> {
        let user_data_dir =
            crate::common::platform::get_user_data_dir(&self.manifest.product_name)?;
        let user_data_path = Path::new(&user_data_dir);

        if user_data_path.exists() {
            tracing::info!("Removing user data from: {:?}", user_data_path);
            std::fs::remove_dir_all(user_data_path).map_err(|e| {
                Error::UninstallationFailed(format!("Failed to remove user data directory: {}", e))
            })?;
        }

        Ok(())
    }

    /// 获取卸载进度（0.0 - 1.0）
    pub fn get_progress(&self) -> f32 {
        0.5
    }

    /// 获取当前卸载状态
    pub fn get_status(&self) -> String {
        "Uninstalling...".to_string()
    }
}

fn should_defer_file_cleanup(
    path: &Path,
    install_root: &Path,
    current_exe: Option<&Path>,
    manifest_path: &Path,
) -> bool {
    path == manifest_path
        || current_exe.is_some_and(|exe| exe == path)
        || path.starts_with(install_root)
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
            status: "Preparing...".to_string(),
            is_completed: false,
            error: None,
        }
    }

    pub fn set_keep_user_data(&mut self, keep: bool) {
        self.engine.set_keep_user_data(keep);
    }

    pub fn start(&mut self) {
        let empty_manifest = crate::resources::manifest::UninstallManifest {
            version: String::new(),
            product_name: String::new(),
            publisher: String::new(),
            install_path: String::new(),
            locale: "en-US".to_string(),
            files_to_remove: vec![],
            directories_to_remove: vec![],
            registry_keys_to_remove: vec![],
            registry_values_to_remove: vec![],
            shortcuts_to_remove: vec![],
            close_targets: vec![],
        };

        let mut engine = std::mem::replace(&mut self.engine, UninstallEngine::new(empty_manifest));

        match engine.uninstall_with_progress(|pct, step| {
            self.progress = pct;
            self.status = format!("{:?}", step);
        }) {
            Ok(_) => {
                self.progress = 1.0;
                self.status = "Uninstall complete".to_string();
                self.is_completed = true;
            }
            Err(e) => {
                self.error = Some(e);
                self.status = "Uninstall failed".to_string();
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn uninstall_manifest_roundtrip_is_supported() {
        let temp = tempdir().unwrap();
        let manifest = UninstallManifest {
            version: "1.0.0".to_string(),
            product_name: "Demo".to_string(),
            publisher: "Publisher".to_string(),
            install_path: temp.path().to_string_lossy().to_string(),
            locale: "en-US".to_string(),
            files_to_remove: vec![],
            directories_to_remove: vec![],
            registry_keys_to_remove: vec![],
            registry_values_to_remove: vec![],
            shortcuts_to_remove: vec![],
            close_targets: vec![],
        };
        manifest.save(&temp.path().join("uninstall.json")).unwrap();
        let loaded = UninstallManifest::load(&temp.path().join("uninstall.json")).unwrap();
        assert_eq!(loaded.product_name, "Demo");
        assert_eq!(loaded.publisher, "Publisher");
    }

    #[test]
    fn install_root_files_are_deferred_to_post_exit_cleanup() {
        let temp = tempdir().unwrap();
        let install_root = temp.path().join("app");
        let manifest_path = install_root.join("uninstall.json");
        let data_file = install_root.join("vcruntime140.dll");

        assert!(should_defer_file_cleanup(
            &manifest_path,
            &install_root,
            None,
            &manifest_path
        ));
        assert!(should_defer_file_cleanup(
            &data_file,
            &install_root,
            None,
            &manifest_path
        ));
    }

    #[test]
    fn install_root_deferral_does_not_swallow_external_failures() {
        let temp = tempdir().unwrap();
        let install_root = temp.path().join("app");
        let manifest_path = install_root.join("uninstall.json");
        let external_file = temp.path().join("outside.txt");

        assert!(!should_defer_file_cleanup(
            &external_file,
            &install_root,
            None,
            &manifest_path
        ));
    }
}
