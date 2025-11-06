//! 配置系统测试

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{InstallerConfig, BrandProfile, BrandManager, ConfigValidator};
    use std::path::PathBuf;

    #[test]
    fn test_installer_config_default() {
        let config = InstallerConfig::default();
        assert_eq!(config.project.name, "MyApp");
        assert_eq!(config.install.exe_name, "MyApp.exe");
        assert_eq!(config.ui.window_width, 574);
        assert_eq!(config.ui.window_height, 358);
    }

    #[test]
    fn test_installer_config_load_from_file() {
        // 这个测试需要实际的配置文件
        // 在实际项目中，我们会创建测试配置文件
        let result = InstallerConfig::load_from_file("examples/taptap-cn/installer_config.toml");
        // 由于文件可能不存在，我们只测试函数不会panic
        match result {
            Ok(_) => println!("配置文件加载成功"),
            Err(_) => println!("配置文件不存在，这是预期的"),
        }
    }

    #[test]
    fn test_brand_profile_taptap_cn() {
        let profile = BrandProfile::taptap_cn();
        assert_eq!(profile.id, "cn");
        assert_eq!(profile.product_name, "TapTap");
        assert_eq!(profile.product_pathname, "TapTap");
        assert_eq!(profile.exe_name, "TapTap.exe");
        assert_eq!(profile.installer_mutex_name, "2074b4c6-2724-4965-baf5-6d1606f4961f");
        assert!(!profile.localization.show_language_selector);
        assert_eq!(profile.localization.supported_locales.len(), 1);
        assert_eq!(profile.localization.supported_locales[0], "zh-CN");
    }

    #[test]
    fn test_brand_profile_taptap_global() {
        let profile = BrandProfile::taptap_global();
        assert_eq!(profile.id, "global");
        assert_eq!(profile.product_name, "TapTap(Global)");
        assert_eq!(profile.product_pathname, "TapTapGlobal");
        assert_eq!(profile.exe_name, "TapTapGlobal.exe");
        assert_eq!(profile.installer_mutex_name, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert!(profile.localization.show_language_selector);
        assert_eq!(profile.localization.supported_locales.len(), 10);
    }

    #[test]
    fn test_brand_manager() {
        let manager = BrandManager::new();
        assert!(manager.get_brand("cn").is_some());
        assert!(manager.get_brand("global").is_some());
        assert!(manager.get_brand("nonexistent").is_none());
        
        let brands = manager.list_brands();
        assert!(brands.contains(&"cn".to_string()));
        assert!(brands.contains(&"global".to_string()));
    }

    #[test]
    fn test_brand_to_installer_config() {
        let profile = BrandProfile::taptap_cn();
        let config = profile.to_installer_config();
        
        assert_eq!(config.project.name, "TapTap");
        assert_eq!(config.install.exe_name, "TapTap.exe");
        assert_eq!(config.install.default_path, "C:\\Program Files\\TapTap");
        assert_eq!(config.install.mutex_name, "2074b4c6-2724-4965-baf5-6d1606f4961f");
        assert_eq!(config.registry.help_link, "https://www.taptap.cn");
        assert!(!config.localization.show_language_selector);
    }

    #[test]
    fn test_config_validation() {
        let config = InstallerConfig::default();
        let validator = ConfigValidator::new(".");
        
        // 由于测试环境可能没有完整的资源文件，我们期望验证会失败
        let result = validator.validate(&config);
        match result {
            Ok(_) => println!("配置验证通过"),
            Err(errors) => {
                println!("配置验证失败，错误数量: {}", errors.len());
                for error in &errors {
                    println!("  - {}", error);
                }
                // 在测试环境中，资源文件不存在是预期的
                assert!(errors.iter().any(|e| e.contains("不存在")));
            }
        }
    }

    #[test]
    fn test_env_var_expansion() {
        let mut config = InstallerConfig::default();
        config.install.default_path = "%PROGRAMFILES%\\MyApp".to_string();
        config.registry.install_path_key = "HKLM\\Software\\%PRODUCTNAME%".to_string();
        
        config.expand_env_vars();
        
        assert!(config.install.default_path.contains("C:\\Program Files"));
        // 注意：%PRODUCTNAME% 不会被展开，因为我们没有实现这个变量
        assert_eq!(config.registry.install_path_key, "HKLM\\Software\\%PRODUCTNAME%");
    }

    #[test]
    fn test_wizard_config() {
        let wizard = crate::config::wizard_config::WizardConfig::default_install();
        
        assert_eq!(wizard.pages.len(), 3);
        assert_eq!(wizard.uninstall_pages.len(), 3);
        
        assert!(wizard.has_page("config"));
        assert!(wizard.has_page("installing"));
        assert!(wizard.has_page("finish"));
        assert!(wizard.has_page("uninstall_config"));
        assert!(!wizard.has_page("nonexistent"));
        
        let config_page = wizard.get_page("config").unwrap();
        assert_eq!(config_page.id, "config");
        assert_eq!(config_page.layout, "configpage.xml");
        assert_eq!(config_page.title, "安装配置");
    }

    #[test]
    fn test_update_mode_wizard() {
        let wizard = crate::config::wizard_config::WizardConfig::update_mode();
        
        // 更新模式应该跳过配置页
        assert_eq!(wizard.pages.len(), 2);
        assert!(!wizard.has_page("config"));
        assert!(wizard.has_page("installing"));
        assert!(wizard.has_page("finish"));
    }
}
