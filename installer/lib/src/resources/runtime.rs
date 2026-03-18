/// 运行时资源管理
///
/// 从 exe 中提取并缓存嵌入的资源（分段格式）
use super::bundle::{extract_bundle_from_exe, ResourceBundle, SegmentType};
use crate::config::InstallerConfig;
use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;

static RUNTIME_RESOURCES: OnceCell<RwLock<RuntimeResources>> = OnceCell::new();

/// 运行时资源容器
pub struct RuntimeResources {
    bundle: ResourceBundle,

    // 解压后的分段缓存
    ui_files: OnceCell<HashMap<String, Vec<u8>>>,
    locale_files: OnceCell<HashMap<String, Vec<u8>>>,
    script_files: OnceCell<HashMap<String, Vec<u8>>>,
}

impl RuntimeResources {
    /// 初始化运行时资源
    pub fn init(exe_path: Option<PathBuf>) -> Result<()> {
        let exe_path = exe_path
            .unwrap_or_else(|| std::env::current_exe().expect("Failed to get current exe path"));

        let bundle = extract_bundle_from_exe(&exe_path)
            .context("Failed to extract resource bundle from exe")?;

        let resources = RuntimeResources {
            bundle,
            ui_files: OnceCell::new(),
            locale_files: OnceCell::new(),
            script_files: OnceCell::new(),
        };

        RUNTIME_RESOURCES
            .set(RwLock::new(resources))
            .map_err(|_| anyhow::anyhow!("Runtime resources already initialized"))?;

        Ok(())
    }

    /// 获取配置
    pub fn get_config() -> Result<InstallerConfig> {
        let resources = RUNTIME_RESOURCES
            .get()
            .context("Runtime resources not initialized")?;
        let resources = resources.read();

        // Config 段是未压缩的，直接获取
        let config_data = resources
            .bundle
            .get("config")
            .or_else(|| resources.bundle.get("installer_config.json"))
            .context("Config not found in embedded resources")?;

        let config: InstallerConfig =
            serde_json::from_slice(&config_data).context("Failed to parse config")?;

        Ok(config)
    }

    /// 获取布局文件
    pub fn get_layout(name: &str) -> Result<String> {
        let resources = RUNTIME_RESOURCES
            .get()
            .context("Runtime resources not initialized")?;
        let resources = resources.write();

        // 确保 UI 资源已解压
        let ui_files = resources.ui_files.get_or_try_init(|| {
            tracing::debug!("Initializing UI files from bundle...");
            resources
                .bundle
                .decompress_segment(SegmentType::UIResources)
        })?;

        tracing::debug!("UI files cache has {} entries", ui_files.len());
        if ui_files.len() < 10 {
            for key in ui_files.keys() {
                tracing::debug!("  - {}", key);
            }
        }

        // 文件名已经包含完整路径（layouts/xxx.xml），直接使用
        let data = ui_files
            .get(name)
            .with_context(|| format!("Layout not found: {}", name))?;

        tracing::debug!("Layout {} loaded, size: {} bytes", name, data.len());

        String::from_utf8(data.clone()).context("Layout file is not valid UTF-8")
    }

    /// 获取资源文件（图片等）
    pub fn get_asset(name: &str) -> Result<Vec<u8>> {
        let resources = RUNTIME_RESOURCES
            .get()
            .context("Runtime resources not initialized")?;
        let resources = resources.write();

        // 确保 UI 资源已解压
        let ui_files = resources.ui_files.get_or_try_init(|| {
            resources
                .bundle
                .decompress_segment(SegmentType::UIResources)
        })?;

        // 文件名已经包含完整路径（assets/xxx.png），直接使用
        let data = ui_files
            .get(name)
            .with_context(|| format!("Asset not found: {}", name))?;

        tracing::debug!("Asset {} loaded, size: {} bytes", name, data.len());

        Ok(data.clone())
    }

    /// 获取语言包数据
    pub fn get_locale(locale: &str) -> Result<Vec<u8>> {
        let resources = RUNTIME_RESOURCES
            .get()
            .context("Runtime resources not initialized")?;
        let resources = resources.write();

        // 确保 locale 资源已解压
        let locale_files = resources
            .locale_files
            .get_or_try_init(|| resources.bundle.decompress_segment(SegmentType::Locales))?;

        let path = format!("{}.pak", locale);
        let data = locale_files
            .get(&path)
            .with_context(|| format!("Locale not found: {}", locale))?;

        Ok(data.clone())
    }

    /// 获取脚本文件 (e.g., "scripts/install.rhai")
    pub fn get_script(name: &str) -> Result<String> {
        let resources = RUNTIME_RESOURCES
            .get()
            .context("Runtime resources not initialized")?;
        let resources = resources.write();

        let script_files = resources
            .script_files
            .get_or_try_init(|| resources.bundle.decompress_segment(SegmentType::Scripts))?;

        let data = script_files
            .get(name)
            .with_context(|| format!("Script not found: {}", name))?;

        String::from_utf8(data.clone())
            .with_context(|| format!("Script {} is not valid UTF-8", name))
    }

    /// 获取 payload（如果存在）
    pub fn get_payload() -> Option<Vec<u8>> {
        let resources = RUNTIME_RESOURCES.get()?;
        let resources = resources.read();

        // Payload 段本身就是 7z，不需要解压
        resources
            .bundle
            .get_segment(SegmentType::Payload)
            .map(|seg| seg.data.clone())
    }

    /// 获取卸载器（如果存在）
    pub fn get_uninstaller() -> Option<Vec<u8>> {
        let resources = RUNTIME_RESOURCES.get()?;
        let resources = resources.read();

        // Uninstaller 段是完整的 exe，不需要解压
        resources
            .bundle
            .get_segment(SegmentType::Uninstaller)
            .map(|seg| seg.data.clone())
    }

    /// 列出所有布局文件
    pub fn list_layouts() -> Vec<String> {
        let resources = match RUNTIME_RESOURCES.get() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let resources = resources.write();

        // 确保 UI 资源已解压
        let ui_files = match resources.ui_files.get_or_try_init(|| {
            resources
                .bundle
                .decompress_segment(SegmentType::UIResources)
        }) {
            Ok(files) => files,
            Err(_) => return Vec::new(),
        };

        ui_files
            .keys()
            .filter_map(|name| name.strip_prefix("layouts/").map(|s| s.to_string()))
            .collect()
    }

    /// 列出所有资源文件
    pub fn list_assets() -> Vec<String> {
        let resources = match RUNTIME_RESOURCES.get() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let resources = resources.write();

        // 确保 UI 资源已解压
        let ui_files = match resources.ui_files.get_or_try_init(|| {
            resources
                .bundle
                .decompress_segment(SegmentType::UIResources)
        }) {
            Ok(files) => files,
            Err(_) => return Vec::new(),
        };

        ui_files
            .keys()
            .filter_map(|name| name.strip_prefix("assets/").map(|s| s.to_string()))
            .collect()
    }

    /// 列出所有可用的语言
    pub fn list_locales() -> Vec<String> {
        let resources = match RUNTIME_RESOURCES.get() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let resources = resources.write();

        // 确保 locale 资源已解压
        let locale_files = match resources
            .locale_files
            .get_or_try_init(|| resources.bundle.decompress_segment(SegmentType::Locales))
        {
            Ok(files) => files,
            Err(_) => return Vec::new(),
        };

        locale_files
            .keys()
            .filter_map(|name| name.strip_suffix(".pak").map(|s| s.to_string()))
            .collect()
    }

    /// 解压 payload 到临时目录
    pub fn extract_payload_to_temp() -> Result<PathBuf> {
        let payload_data = Self::get_payload().context("Payload not found in resources")?;

        let temp_dir =
            std::env::temp_dir().join(format!("nano_installer_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)?;

        let payload_path = temp_dir.join("payload.7z");
        std::fs::write(&payload_path, payload_data)?;

        // TODO: 使用 7z 解压到 temp_dir
        // 当前先返回 7z 文件路径

        Ok(payload_path)
    }
}
