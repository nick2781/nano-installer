//! 配置验证
//!
//! 验证配置文件的完整性和正确性

use crate::config::InstallerConfig;
use std::path::{Path, PathBuf};

/// 配置验证器
pub struct ConfigValidator {
    base_path: PathBuf,
}

impl ConfigValidator {
    /// 创建新的验证器
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// 验证配置
    pub fn validate(&self, config: &InstallerConfig) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 验证必需字段
        self.validate_required_fields(config, &mut errors);

        // 验证路径存在性
        self.validate_paths(config, &mut errors);

        // 验证资源完整性
        self.validate_resources(config, &mut errors);

        // 验证配置逻辑
        self.validate_logic(config, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 验证必需字段
    fn validate_required_fields(&self, config: &InstallerConfig, errors: &mut Vec<String>) {
        if config.project.name.is_empty() {
            errors.push("项目名称不能为空".to_string());
        }
        if config.project.version.is_empty() {
            errors.push("项目版本不能为空".to_string());
        }
        if config.install.exe_name.is_empty() {
            errors.push("主程序名不能为空".to_string());
        }
        if config.install.default_path.is_empty() {
            errors.push("默认安装路径不能为空".to_string());
        }
        if config.install.mutex_name.is_empty() {
            errors.push("互斥锁名称不能为空".to_string());
        }
        for target in &config.install.close_targets {
            if target.name.trim().is_empty() {
                errors.push("关闭目标名称不能为空".to_string());
            }
        }
        if config.registry.install_path_key.is_empty() {
            errors.push("安装路径注册表键不能为空".to_string());
        }
        if config.registry.uninstall_key.is_empty() {
            errors.push("卸载信息注册表键不能为空".to_string());
        }
    }

    /// 验证路径存在性
    fn validate_paths(&self, config: &InstallerConfig, errors: &mut Vec<String>) {
        // 验证资源目录
        let layouts_dir = self.base_path.join(&config.resources.layouts_dir);
        if !layouts_dir.exists() {
            errors.push(format!("布局目录不存在: {}", layouts_dir.display()));
        }

        let assets_dir = self.base_path.join(&config.resources.assets_dir);
        if !assets_dir.exists() {
            errors.push(format!("资源目录不存在: {}", assets_dir.display()));
        }

        let locales_dir = self.base_path.join(&config.resources.locales_dir);
        if !locales_dir.exists() {
            errors.push(format!("语言目录不存在: {}", locales_dir.display()));
        }

        // 验证payload文件
        let payload_file = self.base_path.join(&config.resources.payload_file);
        if !payload_file.exists() {
            errors.push(format!("Payload文件不存在: {}", payload_file.display()));
        }

        // 验证图标文件
        let installer_icon = self.base_path.join(&config.resources.installer_icon);
        if !installer_icon.exists() {
            errors.push(format!("安装器图标不存在: {}", installer_icon.display()));
        }

        let uninstaller_icon = self.base_path.join(&config.resources.uninstaller_icon);
        if !uninstaller_icon.exists() {
            errors.push(format!("卸载器图标不存在: {}", uninstaller_icon.display()));
        }
    }

    /// 验证资源完整性
    fn validate_resources(&self, config: &InstallerConfig, errors: &mut Vec<String>) {
        // 验证布局文件
        for page in &config.wizard.pages {
            let layout_file = self
                .base_path
                .join(&config.resources.layouts_dir)
                .join(&page.layout);
            if !layout_file.exists() {
                errors.push(format!("布局文件不存在: {}", layout_file.display()));
            }
        }

        for page in &config.wizard.uninstall_pages {
            let layout_file = self
                .base_path
                .join(&config.resources.layouts_dir)
                .join(&page.layout);
            if !layout_file.exists() {
                errors.push(format!("卸载布局文件不存在: {}", layout_file.display()));
            }
        }

        // 验证语言文件
        for locale in &config.localization.supported_locales {
            let locale_file = self
                .base_path
                .join(&config.resources.locales_dir)
                .join(format!("{}.json", locale));
            if !locale_file.exists() {
                errors.push(format!("语言文件不存在: {}", locale_file.display()));
            }
        }

        // 验证默认语言文件
        let default_locale_file = self
            .base_path
            .join(&config.resources.locales_dir)
            .join(format!("{}.json", config.localization.default_locale));
        if !default_locale_file.exists() {
            errors.push(format!(
                "默认语言文件不存在: {}",
                default_locale_file.display()
            ));
        }
    }

    /// 验证配置逻辑
    fn validate_logic(&self, config: &InstallerConfig, errors: &mut Vec<String>) {
        // 验证版本号格式
        if !self.is_valid_version(&config.project.version) {
            errors.push("版本号格式无效，应为 x.y.z 格式".to_string());
        }

        // 验证互斥锁名称格式
        if !self.is_valid_mutex_name(&config.install.mutex_name) {
            errors.push("互斥锁名称格式无效，应为有效的Windows对象名称".to_string());
        }

        // 验证注册表键格式
        if !self.is_valid_registry_key(&config.registry.install_path_key) {
            errors.push("安装路径注册表键格式无效".to_string());
        }

        if !self.is_valid_registry_key(&config.registry.uninstall_key) {
            errors.push("卸载信息注册表键格式无效".to_string());
        }

        // 验证磁盘空间
        if config.install.required_space_mb == 0 {
            errors.push("需要的磁盘空间不能为0".to_string());
        }

        // 验证窗口尺寸
        if config.ui.window_width == 0 || config.ui.window_height == 0 {
            errors.push("窗口尺寸不能为0".to_string());
        }

        if config.ui.expanded_height < config.ui.window_height {
            errors.push("展开高度不能小于窗口高度".to_string());
        }

        // 验证DPI阈值
        if config.ui.dpi_threshold == 0 {
            errors.push("DPI阈值不能为0".to_string());
        }
    }

    /// 验证版本号格式
    fn is_valid_version(&self, version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }

        for part in parts {
            if part.parse::<u32>().is_err() {
                return false;
            }
        }

        true
    }

    /// 验证互斥锁名称格式
    fn is_valid_mutex_name(&self, name: &str) -> bool {
        // Windows对象名称不能包含某些字符
        !name.contains(['<', '>', ':', '"', '|', '?', '*']) && !name.is_empty()
    }

    /// 验证注册表键格式
    fn is_valid_registry_key(&self, key: &str) -> bool {
        // 简单的注册表键格式验证
        key.starts_with("HKLM\\") || key.starts_with("HKCU\\") || key.starts_with("HKCR\\")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InstallerConfig;

    #[test]
    fn test_validate_required_fields() {
        let config = InstallerConfig::default();
        let validator = ConfigValidator::new(".");
        let mut errors = Vec::new();

        validator.validate_required_fields(&config, &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_version() {
        let validator = ConfigValidator::new(".");

        assert!(validator.is_valid_version("1.0.0"));
        assert!(validator.is_valid_version("1.2.3"));
        assert!(!validator.is_valid_version("1.0"));
        assert!(!validator.is_valid_version("1.0.0.0"));
        assert!(!validator.is_valid_version("1.0.a"));
    }

    #[test]
    fn test_validate_mutex_name() {
        let validator = ConfigValidator::new(".");

        assert!(validator.is_valid_mutex_name("my-app-mutex"));
        assert!(validator.is_valid_mutex_name("MyApp_Installer_Mutex"));
        assert!(!validator.is_valid_mutex_name("my<app>mutex"));
        assert!(!validator.is_valid_mutex_name(""));
    }

    #[test]
    fn test_validate_registry_key() {
        let validator = ConfigValidator::new(".");

        assert!(validator.is_valid_registry_key("HKLM\\Software\\MyApp"));
        assert!(validator.is_valid_registry_key("HKCU\\Software\\MyApp"));
        assert!(!validator.is_valid_registry_key("Software\\MyApp"));
        assert!(!validator.is_valid_registry_key(""));
    }
}
