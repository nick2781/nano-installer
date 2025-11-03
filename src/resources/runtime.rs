/// 运行时资源加载器
/// 
/// 从嵌入在 exe 中的资源包加载配置、布局、资源等

use anyhow::{Context, Result};
use std::path::Path;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::collections::HashMap;

use super::bundle::{ResourceBundle, ResourceType, ResourceItem, extract_bundle_from_exe};
use crate::config::InstallerConfig;

/// 全局资源加载器
static RUNTIME_RESOURCES: OnceCell<RwLock<RuntimeResources>> = OnceCell::new();

/// 运行时资源
pub struct RuntimeResources {
    bundle: ResourceBundle,
    cache: HashMap<String, Vec<u8>>,
}

impl RuntimeResources {
    /// 初始化运行时资源（从当前 exe 加载）
    pub fn init() -> Result<()> {
        let exe_path = std::env::current_exe()
            .context("Failed to get current exe path")?;
        
        let bundle = extract_bundle_from_exe(&exe_path)
            .context("Failed to extract embedded resources")?;
        
        RUNTIME_RESOURCES.set(RwLock::new(RuntimeResources {
            bundle,
            cache: HashMap::new(),
        })).map_err(|_| anyhow::anyhow!("Runtime resources already initialized"))?;
        
        Ok(())
    }
    
    /// 获取配置
    pub fn get_config() -> Result<InstallerConfig> {
        let resources = RUNTIME_RESOURCES.get()
            .context("Runtime resources not initialized")?;
        let resources = resources.read();
        
        let config_item = resources.bundle.get("installer_config.json")
            .context("Config not found in embedded resources")?;
        
        let config: InstallerConfig = serde_json::from_slice(&config_item.data)
            .context("Failed to parse config")?;
        
        Ok(config)
    }
    
    /// 获取布局文件
    pub fn get_layout(name: &str) -> Result<String> {
        let resources = RUNTIME_RESOURCES.get()
            .context("Runtime resources not initialized")?;
        let resources = resources.read();
        
        let path = format!("layouts/{}", name);
        let item = resources.bundle.get(&path)
            .with_context(|| format!("Layout not found: {}", name))?;
        
        String::from_utf8(item.data.clone())
            .context("Layout file is not valid UTF-8")
    }
    
    /// 获取资源文件（图片等）
    pub fn get_asset(name: &str) -> Result<Vec<u8>> {
        let resources = RUNTIME_RESOURCES.get()
            .context("Runtime resources not initialized")?;
        let mut resources = resources.write();
        
        let path = format!("assets/{}", name);
        
        // 检查缓存
        if let Some(data) = resources.cache.get(&path) {
            return Ok(data.clone());
        }
        
        // 从 bundle 加载
        let item = resources.bundle.get(&path)
            .with_context(|| format!("Asset not found: {}", name))?;
        
        let data = item.data.clone();
        resources.cache.insert(path, data.clone());
        
        Ok(data)
    }
    
    /// 获取语言文件
    pub fn get_locale(locale: &str) -> Result<Vec<u8>> {
        let resources = RUNTIME_RESOURCES.get()
            .context("Runtime resources not initialized")?;
        let resources = resources.read();
        
        let path = format!("locales/{}.json", locale);
        let item = resources.bundle.get(&path)
            .with_context(|| format!("Locale not found: {}", locale))?;
        
        Ok(item.data.clone())
    }
    
    /// 获取 payload
    pub fn get_payload() -> Result<Vec<u8>> {
        let resources = RUNTIME_RESOURCES.get()
            .context("Runtime resources not initialized")?;
        let resources = resources.read();
        
        let item = resources.bundle.get("payload.7z")
            .context("Payload not found in embedded resources")?;
        
        Ok(item.data.clone())
    }
    
    /// 列出所有布局文件
    pub fn list_layouts() -> Result<Vec<String>> {
        let resources = RUNTIME_RESOURCES.get()
            .context("Runtime resources not initialized")?;
        let resources = resources.read();
        
        let layouts = resources.bundle.get_by_type(ResourceType::Layout)
            .iter()
            .map(|item| item.name.strip_prefix("layouts/").unwrap_or(&item.name).to_string())
            .collect();
        
        Ok(layouts)
    }
    
    /// 列出所有资源文件
    pub fn list_assets() -> Result<Vec<String>> {
        let resources = RUNTIME_RESOURCES.get()
            .context("Runtime resources not initialized")?;
        let resources = resources.read();
        
        let assets = resources.bundle.get_by_type(ResourceType::Asset)
            .iter()
            .map(|item| item.name.strip_prefix("assets/").unwrap_or(&item.name).to_string())
            .collect();
        
        Ok(assets)
    }
    
    /// 列出所有语言
    pub fn list_locales() -> Result<Vec<String>> {
        let resources = RUNTIME_RESOURCES.get()
            .context("Runtime resources not initialized")?;
        let resources = resources.read();
        
        let locales = resources.bundle.get_by_type(ResourceType::Locale)
            .iter()
            .filter_map(|item| {
                let name = item.name.strip_prefix("locales/")?;
                let name = name.strip_suffix(".json")?;
                Some(name.to_string())
            })
            .collect();
        
        Ok(locales)
    }
    
    /// 提取 payload 到临时文件
    pub fn extract_payload_to_temp() -> Result<std::path::PathBuf> {
        let payload_data = Self::get_payload()?;
        
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("nano_installer_payload_{}.7z", uuid::Uuid::new_v4()));
        
        std::fs::write(&temp_file, payload_data)
            .context("Failed to write payload to temp file")?;
        
        Ok(temp_file)
    }
}

/// 检查是否有嵌入资源
pub fn has_embedded_resources() -> bool {
    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    
    extract_bundle_from_exe(&exe_path).is_ok()
}

/// 是否在开发模式（没有嵌入资源，从文件系统加载）
pub fn is_dev_mode() -> bool {
    !has_embedded_resources()
}

