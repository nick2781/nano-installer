use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::Result;

/// 品牌配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandConfig {
    pub name: String,
    pub display_name: String,
    pub publisher: String,
    pub version: String,
    pub output_name: String,
    pub exe_name: String,
    pub default_path: String,
    pub mutex_name: String,
    pub help_link: Option<String>,
    pub terms_link: Option<String>,
    pub privacy_link: Option<String>,
    pub supported_locales: Vec<String>,
    pub default_locale: String,
    pub assets_prefix: String,
    pub icon_name: String,
}

impl BrandConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: name.to_string(),
            publisher: "Your Company".to_string(),
            version: "1.0.0".to_string(),
            output_name: format!("{}_Setup", name),
            exe_name: name.to_lowercase(),
            default_path: format!("C:\\Program Files\\{}", name),
            mutex_name: format!("{}_Installer", name),
            help_link: None,
            terms_link: None,
            privacy_link: None,
            supported_locales: vec!["en-US".to_string()],
            default_locale: "en-US".to_string(),
            assets_prefix: name.to_lowercase(),
            icon_name: "icon".to_string(),
        }
    }
}

/// 品牌管理器
pub struct BrandManager {
    current_brand: String,
    brands: HashMap<String, BrandConfig>,
}

impl BrandManager {
    pub fn new() -> Self {
        Self {
            current_brand: "default".to_string(),
            brands: HashMap::new(),
        }
    }

    /// 加载品牌配置
    pub fn load_brands(&mut self, config_dir: &Path) -> Result<()> {
        let brands_dir = config_dir.join("brands");
        if !brands_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&brands_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let content = std::fs::read_to_string(&path)?;
                let brand: BrandConfig = toml::from_str(&content)?;
                
                let brand_name = path.file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                
                self.brands.insert(brand_name, brand);
            }
        }

        Ok(())
    }

    /// 设置当前品牌
    pub fn set_brand(&mut self, brand_name: &str) -> Result<()> {
        if !self.brands.contains_key(brand_name) {
            anyhow::bail!("Brand '{}' not found", brand_name);
        }
        
        self.current_brand = brand_name.to_string();
        Ok(())
    }

    /// 获取当前品牌配置
    pub fn get_current_brand(&self) -> Option<&BrandConfig> {
        self.brands.get(&self.current_brand)
    }

    /// 获取品牌配置
    pub fn get_brand(&self, name: &str) -> Option<&BrandConfig> {
        self.brands.get(name)
    }

    /// 列出所有品牌
    pub fn list_brands(&self) -> Vec<&String> {
        self.brands.keys().collect()
    }

    /// 获取资源路径
    pub fn get_resource_path(&self, resource_name: &str) -> String {
        if let Some(brand) = self.get_current_brand() {
            format!("{}/{}", brand.assets_prefix, resource_name)
        } else {
            resource_name.to_string()
        }
    }

    /// 获取图标路径
    pub fn get_icon_path(&self) -> String {
        if let Some(brand) = self.get_current_brand() {
            format!("{}/{}.ico", brand.assets_prefix, brand.icon_name)
        } else {
            "icon.ico".to_string()
        }
    }
}

impl Default for BrandManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 预定义品牌配置
pub mod presets {
    use super::BrandConfig;

    pub fn taptap_cn() -> BrandConfig {
        BrandConfig {
            name: "TapTap".to_string(),
            display_name: "TapTap".to_string(),
            publisher: "TapTap".to_string(),
            version: "1.0.0".to_string(),
            output_name: "TapTap_Setup".to_string(),
            exe_name: "taptap".to_string(),
            default_path: "C:\\Program Files\\TapTap".to_string(),
            mutex_name: "TapTap_Installer".to_string(),
            help_link: Some("https://www.taptap.com/help".to_string()),
            terms_link: Some("https://www.taptap.com/terms".to_string()),
            privacy_link: Some("https://www.taptap.com/privacy".to_string()),
            supported_locales: vec!["zh-CN".to_string()],
            default_locale: "zh-CN".to_string(),
            assets_prefix: "taptap".to_string(),
            icon_name: "taptap".to_string(),
        }
    }

    pub fn taptap_global() -> BrandConfig {
        BrandConfig {
            name: "TapTap".to_string(),
            display_name: "TapTap".to_string(),
            publisher: "TapTap".to_string(),
            version: "1.0.0".to_string(),
            output_name: "TapTap_Setup".to_string(),
            exe_name: "taptap".to_string(),
            default_path: "C:\\Program Files\\TapTap".to_string(),
            mutex_name: "TapTap_Installer".to_string(),
            help_link: Some("https://www.taptap.com/help".to_string()),
            terms_link: Some("https://www.taptap.com/terms".to_string()),
            privacy_link: Some("https://www.taptap.com/privacy".to_string()),
            supported_locales: vec![
                "en-US".to_string(),
                "zh-CN".to_string(),
                "zh-TW".to_string(),
                "ja".to_string(),
                "ko".to_string(),
                "vi".to_string(),
                "th".to_string(),
                "id".to_string(),
                "ms".to_string(),
                "pt".to_string(),
            ],
            default_locale: "en-US".to_string(),
            assets_prefix: "taptap".to_string(),
            icon_name: "taptap".to_string(),
        }
    }

    pub fn minimal() -> BrandConfig {
        BrandConfig::new("MyApp")
    }
}
