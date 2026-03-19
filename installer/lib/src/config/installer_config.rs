//! 主配置文件解析
//!
//! 解析 installer_config.json 并映射到 InstallerConfig 结构体

use crate::common::close_targets::{effective_close_targets, CloseTarget};
use crate::config::validation::ConfigValidator;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 主安装器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerConfig {
    /// 项目信息
    pub project: ProjectConfig,
    /// 输出配置
    pub output: OutputConfig,
    /// 安装配置
    pub install: InstallConfig,
    /// 注册表配置
    pub registry: RegistryConfig,
    /// 快捷方式配置
    pub shortcuts: ShortcutsConfig,
    /// 开机自启配置
    pub autostart: AutostartConfig,
    /// 多语言配置
    pub localization: LocalizationConfig,
    /// 外部链接
    pub links: LinksConfig,
    /// 资源路径
    pub resources: ResourcesConfig,
    /// UI配置
    pub ui: UiConfig,
    /// 向导流程配置
    pub wizard: WizardConfig,
    /// 卸载配置
    pub uninstall: UninstallConfig,
    /// 路径校验配置
    pub validation: ValidationConfig,
    /// 高级选项
    pub advanced: AdvancedConfig,
    /// 可配置安装任务流水线 (可选, 不配置则用默认流水线)
    #[serde(default)]
    pub install_tasks: Option<Vec<crate::installer::task_runner::TaskConfig>>,
}

/// 项目信息配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// 项目名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 发布者
    pub publisher: String,
    /// 版权信息
    pub copyright: String,
    /// 输出文件名（不含版本和渠道）
    pub output_name: String,
}

/// 输出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// 安装器文件名
    pub installer_name: String,
    /// 安装器图标
    pub installer_icon: String,
    /// 卸载器文件名
    pub uninstaller_name: String,
    /// 卸载器图标
    pub uninstaller_icon: String,
}

/// 安装配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallConfig {
    /// 安装后主程序名
    pub exe_name: String,
    /// 默认安装路径
    pub default_path: String,
    /// 用户选择目录后追加的子目录
    pub append_to_path: String,
    /// 需要的磁盘空间（MB）
    pub required_space_mb: u32,
    /// 是否需要管理员权限
    pub require_admin: bool,
    /// 单实例互斥锁名称
    pub mutex_name: String,
    /// 是否检测正在运行的进程
    pub detect_running_process: bool,
    /// 安装时强制结束进程
    pub kill_process_on_install: bool,
    /// 卸载时强制结束进程
    pub kill_process_on_uninstall: bool,
    /// 需要在安装/卸载时处理的进程或服务
    #[serde(default)]
    pub close_targets: Vec<CloseTarget>,
}

impl InstallConfig {
    pub fn effective_close_targets(&self) -> Vec<CloseTarget> {
        effective_close_targets(
            &self.close_targets,
            &self.exe_name,
            self.detect_running_process,
            self.kill_process_on_install,
            self.kill_process_on_uninstall,
        )
    }
}

/// 注册表配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// 安装路径注册表键
    pub install_path_key: String,
    /// 卸载信息注册表键
    pub uninstall_key: String,
    /// 帮助链接
    pub help_link: String,
}

/// 快捷方式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    /// 是否显示桌面快捷方式选项
    pub desktop_shortcut: bool,
    /// 桌面快捷方式默认勾选
    pub desktop_default: bool,
    /// 是否创建开始菜单快捷方式
    pub start_menu: bool,
    /// 开始菜单文件夹名
    pub start_menu_folder: String,
}

/// 开机自启配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutostartConfig {
    /// 是否显示开机自启选项
    pub enabled: bool,
    /// 开机自启默认勾选
    pub default: bool,
    /// 注册表键
    pub registry_key: String,
    /// 注册表值名称
    pub registry_value_name: String,
}

/// 多语言配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationConfig {
    /// 默认语言
    pub default_locale: String,
    /// 支持的语言列表
    pub supported_locales: Vec<String>,
    /// 是否在界面显示语言选择器
    pub show_language_selector: bool,
}

/// 外部链接配置 (通用 key→URL 映射)
pub type LinksConfig = std::collections::HashMap<String, String>;

/// 资源路径配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesConfig {
    /// 布局文件目录
    pub layouts_dir: String,
    /// 图片资源目录
    pub assets_dir: String,
    /// 语言文件目录
    pub locales_dir: String,
    /// 要安装的7z文件
    pub payload_file: String,
    /// 安装器图标
    pub installer_icon: String,
    /// 卸载器图标
    pub uninstaller_icon: String,
}

/// UI配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// 窗口宽度
    pub window_width: u32,
    /// 窗口高度
    pub window_height: u32,
    /// 展开自定义选项后的高度
    pub expanded_height: u32,
    /// 主窗口圆角半径
    #[serde(default = "default_window_corner_radius")]
    pub window_corner_radius: u32,
    /// 对话框宽度 (消息框等)
    #[serde(default = "default_dialog_width")]
    pub dialog_width: u32,
    /// 对话框高度
    #[serde(default = "default_dialog_height")]
    pub dialog_height: u32,
    /// DPI自适应
    pub dpi_aware: bool,
    /// DPI阈值（>=此值用2x资源）
    pub dpi_threshold: u32,
}

fn default_window_corner_radius() -> u32 {
    8
}

fn default_dialog_width() -> u32 {
    400
}
fn default_dialog_height() -> u32 {
    230
}

/// 向导流程配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardConfig {
    /// 安装页面列表
    pub pages: Vec<PageConfig>,
    /// 更新模式页面列表 (跳过配置页, 直接 installing -> finish)
    #[serde(default)]
    pub update_pages: Vec<PageConfig>,
    /// 卸载页面列表
    pub uninstall_pages: Vec<PageConfig>,
}

/// 页面配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageConfig {
    /// 页面ID
    pub id: String,
    /// 布局文件
    pub layout: String,
    /// 页面标题
    pub title: String,
}

/// 卸载配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallConfig {
    /// 是否显示"保留数据"选项
    pub show_keep_data_option: bool,
    /// "保留数据"默认勾选
    pub keep_data_default: bool,
}

/// 路径校验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// 检查路径合法性
    pub check_path_legal: bool,
    /// 限制磁盘类型（HDD/SSD/Any）
    pub check_disk_type: String,
    /// 检查磁盘空间
    pub check_disk_space: bool,
}

/// 高级选项配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// 支持静默安装
    pub silent_mode_support: bool,
    /// 支持更新模式
    pub update_mode_support: bool,
    /// 支持卸载模式
    pub uninstall_mode_support: bool,
    /// 安装完成后启动应用
    pub launch_app_after_install: bool,
}

impl InstallerConfig {
    /// 从文件加载配置
    pub fn load_from_file<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: InstallerConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 验证配置
    pub fn validate(&self, base_path: &PathBuf) -> Result<(), Vec<String>> {
        let validator = ConfigValidator::new(base_path);
        validator.validate(self)
    }

    /// 展开环境变量
    pub fn expand_env_vars(&mut self) {
        self.install.default_path = Self::expand_env_vars_in_string(&self.install.default_path);
        self.registry.install_path_key =
            Self::expand_env_vars_in_string(&self.registry.install_path_key);
        self.registry.uninstall_key = Self::expand_env_vars_in_string(&self.registry.uninstall_key);
        self.autostart.registry_key = Self::expand_env_vars_in_string(&self.autostart.registry_key);
    }

    /// 展开字符串中的环境变量
    fn expand_env_vars_in_string(s: &str) -> String {
        s.replace("%PROGRAMFILES%", "C:\\Program Files")
            .replace("%PROGRAMFILES(X86)%", "C:\\Program Files (x86)")
            .replace(
                "%APPDATA%",
                &std::env::var("APPDATA").unwrap_or_else(|_| "%APPDATA%".to_string()),
            )
            .replace(
                "%USERPROFILE%",
                &std::env::var("USERPROFILE").unwrap_or_else(|_| "%USERPROFILE%".to_string()),
            )
    }
}

impl Default for InstallerConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                name: "MyApp".to_string(),
                version: "1.0.0".to_string(),
                publisher: "My Company".to_string(),
                copyright: "© 2025 My Company".to_string(),
                output_name: "MyApp_Setup".to_string(),
            },
            output: OutputConfig {
                installer_name: "MyApp_Setup.exe".to_string(),
                installer_icon: "assets/logo.ico".to_string(),
                uninstaller_name: "uninst.exe".to_string(),
                uninstaller_icon: "assets/uninst.ico".to_string(),
            },
            install: InstallConfig {
                exe_name: "MyApp.exe".to_string(),
                default_path: "C:\\Program Files\\MyApp".to_string(),
                append_to_path: "MyApp".to_string(),
                required_space_mb: 100,
                require_admin: true,
                mutex_name: "myapp-installer-mutex".to_string(),
                detect_running_process: true,
                kill_process_on_install: true,
                kill_process_on_uninstall: true,
                close_targets: vec![],
            },
            registry: RegistryConfig {
                install_path_key: "HKLM\\Software\\MyApp".to_string(),
                uninstall_key:
                    "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\MyApp"
                        .to_string(),
                help_link: "https://www.myapp.com".to_string(),
            },
            shortcuts: ShortcutsConfig {
                desktop_shortcut: true,
                desktop_default: true,
                start_menu: true,
                start_menu_folder: "MyApp".to_string(),
            },
            autostart: AutostartConfig {
                enabled: true,
                default: false,
                registry_key: "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run".to_string(),
                registry_value_name: "MyApp".to_string(),
            },
            localization: LocalizationConfig {
                default_locale: "en-US".to_string(),
                supported_locales: vec!["en-US".to_string()],
                show_language_selector: false,
            },
            links: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    "terms_of_service".to_string(),
                    "https://www.myapp.com/terms".to_string(),
                );
                m.insert(
                    "privacy_policy".to_string(),
                    "https://www.myapp.com/privacy".to_string(),
                );
                m
            },
            resources: ResourcesConfig {
                layouts_dir: "layouts".to_string(),
                assets_dir: "assets".to_string(),
                locales_dir: "locales".to_string(),
                payload_file: "payload/app.7z".to_string(),
                installer_icon: "assets/logo.ico".to_string(),
                uninstaller_icon: "assets/uninst.ico".to_string(),
            },
            ui: UiConfig {
                window_width: 574,
                window_height: 358,
                expanded_height: 518,
                window_corner_radius: 8,
                dialog_width: 400,
                dialog_height: 230,
                dpi_aware: true,
                dpi_threshold: 144,
            },
            wizard: WizardConfig {
                pages: vec![
                    PageConfig {
                        id: "config".to_string(),
                        layout: "configpage.xml".to_string(),
                        title: "Configuration".to_string(),
                    },
                    PageConfig {
                        id: "installing".to_string(),
                        layout: "installingpage.xml".to_string(),
                        title: "Installing".to_string(),
                    },
                    PageConfig {
                        id: "finish".to_string(),
                        layout: "finishpage.xml".to_string(),
                        title: "Complete".to_string(),
                    },
                ],
                update_pages: vec![
                    PageConfig {
                        id: "installing".to_string(),
                        layout: "installingpage.xml".to_string(),
                        title: "Updating".to_string(),
                    },
                    PageConfig {
                        id: "finish".to_string(),
                        layout: "finishpage.xml".to_string(),
                        title: "Update Complete".to_string(),
                    },
                ],
                uninstall_pages: vec![
                    PageConfig {
                        id: "uninstall_confirm".to_string(),
                        layout: "uninstallpage.xml".to_string(),
                        title: "Confirm Uninstall".to_string(),
                    },
                    PageConfig {
                        id: "uninstall_progress".to_string(),
                        layout: "uninstallingpage.xml".to_string(),
                        title: "Uninstalling".to_string(),
                    },
                    PageConfig {
                        id: "uninstall_finish".to_string(),
                        layout: "uninstallfinishpage.xml".to_string(),
                        title: "Uninstall Complete".to_string(),
                    },
                ],
            },
            uninstall: UninstallConfig {
                show_keep_data_option: true,
                keep_data_default: true,
            },
            validation: ValidationConfig {
                check_path_legal: true,
                check_disk_type: "Any".to_string(),
                check_disk_space: true,
            },
            advanced: AdvancedConfig {
                silent_mode_support: true,
                update_mode_support: true,
                uninstall_mode_support: true,
                launch_app_after_install: true,
            },
            install_tasks: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InstallerConfig;

    #[test]
    fn serialized_uninstall_config_does_not_expose_data_paths() {
        let config = InstallerConfig::default();
        let value = serde_json::to_value(&config).expect("serialize config");
        let uninstall = value
            .get("uninstall")
            .expect("uninstall section should be serialized");

        assert!(
            uninstall.get("data_paths").is_none(),
            "product-specific data cleanup should be expressed in uninstall scripts, not config"
        );
    }
}
