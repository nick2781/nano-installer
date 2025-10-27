// 安装清单

use crate::common::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 安装清单（记录安装了什么，用于卸载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallManifest {
    /// 应用名称
    pub app_name: String,

    /// 应用版本
    pub app_version: String,

    /// 安装路径
    pub install_path: PathBuf,

    /// 安装时间
    pub install_time: String,

    /// 已安装的文件列表
    pub installed_files: Vec<PathBuf>,

    /// 创建的快捷方式
    pub shortcuts: Vec<Shortcut>,

    /// 写入的注册表项
    pub registry_keys: Vec<RegistryKey>,

    /// 语言
    pub locale: String,
}

impl InstallManifest {
    /// 创建新的安装清单
    pub fn new(
        app_name: String,
        app_version: String,
        install_path: PathBuf,
        locale: String,
    ) -> Self {
        Self {
            app_name,
            app_version,
            install_path,
            install_time: chrono::Local::now().to_rfc3339(),
            installed_files: Vec::new(),
            shortcuts: Vec::new(),
            registry_keys: Vec::new(),
            locale,
        }
    }

    /// 保存清单到文件
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// 从文件加载清单
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let manifest = serde_json::from_str(&json)?;
        Ok(manifest)
    }
}

/// 快捷方式信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    /// 快捷方式路径
    pub path: PathBuf,

    /// 快捷方式类型
    pub shortcut_type: ShortcutType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShortcutType {
    Desktop,
    StartMenu,
    QuickLaunch,
}

/// 注册表项信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryKey {
    /// 注册表根
    pub root: String,

    /// 注册表路径
    pub path: String,

    /// 键名（如果为 None，表示创建了整个路径）
    pub name: Option<String>,
}

/// 卸载清单（用于卸载器）
pub type UninstallManifest = InstallManifest;
