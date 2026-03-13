use crate::common::close_targets::CloseTarget;
use crate::common::{Error, Result};
use crate::config::InstallerConfig;
use crate::installer::state::InstallState;
use crate::resources::manifest::{RegistryValueRemoval, UninstallManifest};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone)]
pub(crate) struct InstallArtifacts {
    files: BTreeSet<PathBuf>,
    directories: BTreeSet<PathBuf>,
    shortcuts: BTreeSet<PathBuf>,
    registry_keys: BTreeSet<String>,
    registry_values: BTreeSet<(String, String)>,
    rollback_backup_root: Option<PathBuf>,
    preserved_files: BTreeSet<PathBuf>,
    preserved_directories: BTreeSet<PathBuf>,
}

impl InstallArtifacts {
    pub(crate) fn record_file(&mut self, path: PathBuf) {
        self.files.insert(path);
    }

    pub(crate) fn record_directory(&mut self, path: PathBuf) {
        self.directories.insert(path);
    }

    pub(crate) fn record_shortcut(&mut self, path: PathBuf) {
        self.shortcuts.insert(path);
    }

    pub(crate) fn record_registry_key(&mut self, path: impl Into<String>) {
        self.registry_keys.insert(path.into());
    }

    pub(crate) fn record_registry_value(
        &mut self,
        key_path: impl Into<String>,
        value_name: impl Into<String>,
    ) {
        self.registry_values
            .insert((key_path.into(), value_name.into()));
    }

    pub(crate) fn capture_existing_install(&mut self, install_root: &Path) -> Result<()> {
        if self.rollback_backup_root.is_some() || !install_root.exists() {
            return Ok(());
        }

        let (existing_files, existing_directories) = snapshot_install_tree(install_root)?;
        if existing_files.is_empty() && existing_directories.is_empty() {
            return Ok(());
        }

        let backup_root = std::env::temp_dir().join(format!(
            "nano_installer_update_backup_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&backup_root)?;

        for dir in &existing_directories {
            let relative = dir.strip_prefix(install_root).map_err(|e| {
                Error::InstallationFailed(format!(
                    "Failed to compute backup directory for {}: {}",
                    dir.display(),
                    e
                ))
            })?;
            std::fs::create_dir_all(backup_root.join(relative))?;
            self.preserved_directories.insert(dir.clone());
        }

        for file in &existing_files {
            let relative = file.strip_prefix(install_root).map_err(|e| {
                Error::InstallationFailed(format!(
                    "Failed to compute backup file for {}: {}",
                    file.display(),
                    e
                ))
            })?;
            let backup_path = backup_root.join(relative);
            if let Some(parent) = backup_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(file, &backup_path).map_err(|e| {
                Error::InstallationFailed(format!(
                    "Failed to back up {} before update: {}",
                    file.display(),
                    e
                ))
            })?;
            self.preserved_files.insert(file.clone());
        }

        self.rollback_backup_root = Some(backup_root);
        Ok(())
    }

    pub(crate) fn write_uninstall_manifest(
        &mut self,
        config: &InstallerConfig,
        state: &InstallState,
    ) -> Result<()> {
        let manifest_path = PathBuf::from(state.install_path()).join("uninstall.json");
        let mut close_targets = config.install.effective_close_targets();
        for target in self.merge_existing_manifest(&manifest_path)? {
            if !close_targets.contains(&target) {
                close_targets.push(target);
            }
        }
        self.record_file(manifest_path.clone());
        self.record_directory(PathBuf::from(state.install_path()));

        let manifest = UninstallManifest {
            version: config.project.version.clone(),
            product_name: config.project.name.clone(),
            publisher: config.project.publisher.clone(),
            install_path: state.install_path(),
            locale: config.localization.default_locale.clone(),
            files_to_remove: self
                .files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            directories_to_remove: self
                .directories
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            registry_keys_to_remove: self.registry_keys.iter().cloned().collect(),
            registry_values_to_remove: self
                .registry_values
                .iter()
                .map(|(key_path, value_name)| RegistryValueRemoval {
                    key_path: key_path.clone(),
                    value_name: value_name.clone(),
                })
                .collect(),
            shortcuts_to_remove: self
                .shortcuts
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            close_targets,
        };

        manifest.save(&manifest_path).map_err(|e| {
            Error::InstallationFailed(format!("Failed to write uninstall manifest: {}", e))
        })?;
        self.cleanup_backup();
        Ok(())
    }

    fn merge_existing_manifest(&mut self, manifest_path: &Path) -> Result<Vec<CloseTarget>> {
        if !manifest_path.exists() {
            return Ok(Vec::new());
        }

        let manifest = UninstallManifest::load(manifest_path).map_err(|e| {
            Error::InstallationFailed(format!("Failed to load existing uninstall manifest: {}", e))
        })?;
        let inherited_close_targets = manifest.close_targets.clone();

        for file in manifest.files_to_remove {
            self.record_file(PathBuf::from(file));
        }
        for dir in manifest.directories_to_remove {
            self.record_directory(PathBuf::from(dir));
        }
        for key in manifest.registry_keys_to_remove {
            self.record_registry_key(key);
        }
        for value in manifest.registry_values_to_remove {
            self.record_registry_value(value.key_path, value.value_name);
        }
        for shortcut in manifest.shortcuts_to_remove {
            self.record_shortcut(PathBuf::from(shortcut));
        }

        Ok(inherited_close_targets)
    }

    pub(crate) fn rollback(&self, install_root: &Path) -> Result<()> {
        #[cfg(windows)]
        {
            use crate::installer::windows::{registry, shortcuts};

            for shortcut in &self.shortcuts {
                if let Err(e) = shortcuts::remove_shortcut(shortcut) {
                    tracing::warn!(
                        "Rollback failed to remove shortcut {}: {}",
                        shortcut.display(),
                        e
                    );
                }
            }

            for (key_path, value_name) in self.registry_values.iter().rev() {
                if let Err(e) = registry::delete_value(key_path, value_name) {
                    tracing::warn!(
                        "Rollback failed to delete registry value {}\\{}: {}",
                        key_path,
                        value_name,
                        e
                    );
                }
            }

            for key_path in self.registry_keys.iter().rev() {
                if let Err(e) = registry::delete_key(key_path) {
                    tracing::warn!("Rollback failed to delete registry key {}: {}", key_path, e);
                }
            }
        }

        for file in self.files.iter().rev() {
            if file.exists() {
                if let Err(e) = std::fs::remove_file(file) {
                    tracing::warn!("Rollback failed to remove file {}: {}", file.display(), e);
                }
            }
        }

        let mut directories: Vec<_> = self.directories.iter().cloned().collect();
        directories.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
        for dir in directories {
            if dir.exists() {
                match std::fs::remove_dir(&dir) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
                    Err(e) => tracing::warn!(
                        "Rollback failed to remove directory {}: {}",
                        dir.display(),
                        e
                    ),
                }
            }
        }

        if install_root.exists() {
            match std::fs::remove_dir(install_root) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
                Err(e) => tracing::warn!(
                    "Rollback failed to remove install root {}: {}",
                    install_root.display(),
                    e
                ),
            }
        }

        self.restore_preserved_install(install_root)?;

        Ok(())
    }

    pub(crate) fn cleanup_backup(&mut self) {
        if let Some(backup_root) = self.rollback_backup_root.take() {
            let _ = std::fs::remove_dir_all(&backup_root);
        }
        self.preserved_files.clear();
        self.preserved_directories.clear();
    }

    fn restore_preserved_install(&self, install_root: &Path) -> Result<()> {
        let Some(backup_root) = &self.rollback_backup_root else {
            return Ok(());
        };

        std::fs::create_dir_all(install_root)?;

        for dir in &self.preserved_directories {
            if let Err(e) = std::fs::create_dir_all(dir) {
                tracing::warn!(
                    "Rollback failed to recreate directory {}: {}",
                    dir.display(),
                    e
                );
            }
        }

        for file in &self.preserved_files {
            let relative = file.strip_prefix(install_root).map_err(|e| {
                Error::InstallationFailed(format!(
                    "Failed to restore {} from update backup: {}",
                    file.display(),
                    e
                ))
            })?;
            let backup_file = backup_root.join(relative);
            if !backup_file.exists() {
                continue;
            }
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&backup_file, file).map_err(|e| {
                Error::InstallationFailed(format!(
                    "Failed to restore {} from backup: {}",
                    file.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }
}

pub(crate) fn snapshot_install_tree(root: &Path) -> Result<(BTreeSet<PathBuf>, BTreeSet<PathBuf>)> {
    let mut files = BTreeSet::new();
    let mut directories = BTreeSet::new();

    if !root.exists() {
        return Ok((files, directories));
    }

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path == root {
            continue;
        }
        if entry.file_type().is_file() {
            files.insert(path.to_path_buf());
        } else if entry.file_type().is_dir() {
            directories.insert(path.to_path_buf());
        }
    }

    Ok((files, directories))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn snapshot_tracks_new_files_and_dirs() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let (before_files, before_dirs) = snapshot_install_tree(root).unwrap();
        assert!(before_files.is_empty());
        assert!(before_dirs.is_empty());

        let nested = root.join("bin").join("sub");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("app.exe");
        std::fs::write(&file, b"demo").unwrap();

        let (after_files, after_dirs) = snapshot_install_tree(root).unwrap();
        assert!(after_files.contains(&file));
        assert!(after_dirs.contains(&root.join("bin")));
        assert!(after_dirs.contains(&nested));
    }

    #[test]
    fn manifest_includes_registry_values() {
        let temp = tempdir().unwrap();
        let install_dir = temp.path().join("MyApp");
        std::fs::create_dir_all(&install_dir).unwrap();

        let mut config = InstallerConfig::default();
        config.project.name = "MyApp".to_string();
        config.project.version = "1.2.3".to_string();
        config.project.publisher = "Publisher".to_string();
        config.localization.default_locale = "en-US".to_string();

        let state = InstallState::new(install_dir.to_string_lossy().to_string());
        let mut artifacts = InstallArtifacts::default();
        artifacts.record_file(install_dir.join("uninst.exe"));
        artifacts.record_registry_key("HKLM\\Software\\MyApp");
        artifacts.record_registry_value(
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "MyApp",
        );

        artifacts.write_uninstall_manifest(&config, &state).unwrap();

        let manifest = UninstallManifest::load(&install_dir.join("uninstall.json")).unwrap();
        assert_eq!(manifest.product_name, "MyApp");
        assert_eq!(
            manifest.registry_keys_to_remove,
            vec!["HKLM\\Software\\MyApp".to_string()]
        );
        assert_eq!(manifest.registry_values_to_remove.len(), 1);
        assert_eq!(manifest.registry_values_to_remove[0].value_name, "MyApp");
        assert_eq!(manifest.close_targets.len(), 1);
        assert_eq!(manifest.close_targets[0].name, "MyApp.exe");
    }

    #[test]
    fn manifest_write_merges_existing_entries() {
        let temp = tempdir().unwrap();
        let install_dir = temp.path().join("MyApp");
        std::fs::create_dir_all(&install_dir).unwrap();

        let old_manifest = UninstallManifest {
            version: "0.9.0".to_string(),
            product_name: "MyApp".to_string(),
            publisher: "Publisher".to_string(),
            install_path: install_dir.to_string_lossy().to_string(),
            locale: "en-US".to_string(),
            files_to_remove: vec![install_dir.join("old.dll").to_string_lossy().to_string()],
            directories_to_remove: vec![install_dir.join("legacy").to_string_lossy().to_string()],
            registry_keys_to_remove: vec!["HKLM\\Software\\MyApp\\Legacy".to_string()],
            registry_values_to_remove: vec![],
            shortcuts_to_remove: vec![install_dir.join("old.lnk").to_string_lossy().to_string()],
            close_targets: vec![
                CloseTarget::legacy_process("Legacy.exe", false, false, true).unwrap(),
            ],
        };
        old_manifest
            .save(&install_dir.join("uninstall.json"))
            .unwrap();

        let config = InstallerConfig::default();
        let state = InstallState::new(install_dir.to_string_lossy().to_string());
        let mut artifacts = InstallArtifacts::default();
        artifacts.record_file(install_dir.join("uninst.exe"));

        artifacts.write_uninstall_manifest(&config, &state).unwrap();

        let manifest = UninstallManifest::load(&install_dir.join("uninstall.json")).unwrap();
        assert!(manifest
            .files_to_remove
            .iter()
            .any(|path| path.ends_with("old.dll")));
        assert!(manifest
            .directories_to_remove
            .iter()
            .any(|path| path.ends_with("legacy")));
        assert!(manifest
            .shortcuts_to_remove
            .iter()
            .any(|path| path.ends_with("old.lnk")));
        assert!(manifest
            .registry_keys_to_remove
            .iter()
            .any(|path| path.ends_with("\\Legacy")));
        assert!(manifest
            .close_targets
            .iter()
            .any(|target| target.name == "Legacy.exe"));
    }

    #[test]
    fn rollback_restores_previous_install_files() {
        let temp = tempdir().unwrap();
        let install_dir = temp.path().join("MyApp");
        std::fs::create_dir_all(&install_dir).unwrap();
        let app_file = install_dir.join("app.exe");
        std::fs::write(&app_file, b"old-version").unwrap();

        let mut artifacts = InstallArtifacts::default();
        artifacts.capture_existing_install(&install_dir).unwrap();

        std::fs::write(&app_file, b"new-version").unwrap();
        let new_file = install_dir.join("new.dll");
        std::fs::write(&new_file, b"new").unwrap();
        artifacts.record_file(new_file.clone());

        artifacts.rollback(&install_dir).unwrap();

        assert_eq!(std::fs::read(&app_file).unwrap(), b"old-version");
        assert!(!new_file.exists());
    }
}
