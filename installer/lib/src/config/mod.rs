//! 配置系统模块
//!
//! 负责解析和验证 installer_config.toml 配置文件

pub mod installer_config;
pub mod validation;
pub mod wizard_config;

pub use installer_config::InstallerConfig;
pub use validation::ConfigValidator;
pub use wizard_config::WizardConfig;
