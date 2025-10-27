// 配置管理

use serde::{Deserialize, Serialize};

/// 安装器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerConfig {
    /// 应用名称
    pub app_name: String,

    /// 应用版本
    pub app_version: String,

    /// 发布者
    pub publisher: String,

    /// 默认安装路径
    pub default_install_path: String,

    /// 默认语言
    pub default_locale: String,

    /// 支持的语言列表
    pub supported_locales: Vec<String>,

    /// 是否需要管理员权限
    pub require_admin: bool,

    /// 压缩包文件名
    pub payload_filename: String,

    /// 预期的压缩包 SHA256
    pub payload_sha256: Option<String>,
}

impl Default for InstallerConfig {
    fn default() -> Self {
        Self {
            app_name: "MyApp".to_string(),
            app_version: "1.0.0".to_string(),
            publisher: "My Company".to_string(),
            default_install_path: r"C:\Program Files\MyApp".to_string(),
            default_locale: "en-US".to_string(),
            supported_locales: vec![
                "en-US".to_string(),
                "zh-CN".to_string(),
                "zh-TW".to_string(),
                "ja".to_string(),
                "vi".to_string(),
            ],
            require_admin: false,
            payload_filename: "app.7z".to_string(),
            payload_sha256: None,
        }
    }
}

/// 卸载器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallerConfig {
    /// 安装路径
    pub install_path: String,

    /// 应用名称
    pub app_name: String,

    /// 应用版本
    pub app_version: String,

    /// 安装时的语言
    pub locale: String,
}
