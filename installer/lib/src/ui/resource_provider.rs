use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// UI 布局和语言资源提供接口。
pub trait UiResourceProvider: Send + Sync {
    /// 读取布局 XML 文本。
    fn load_layout(&self, path: &str) -> Result<String>;
    /// 读取指定语言的键值翻译表。
    fn load_locale_strings(&self, locale: &str) -> Result<HashMap<String, String>>;
}

/// 共享 UI 资源提供器对象。
pub type SharedUiResourceProvider = Arc<dyn UiResourceProvider>;

/// 运行时资源提供器，数据来自嵌入 bundle。
pub struct RuntimeUiResourceProvider;

impl UiResourceProvider for RuntimeUiResourceProvider {
    fn load_layout(&self, path: &str) -> Result<String> {
        crate::resources::RuntimeResources::get_layout(path)
            .with_context(|| format!("failed to load embedded layout: {}", path))
    }

    fn load_locale_strings(&self, locale: &str) -> Result<HashMap<String, String>> {
        let pak_data = crate::resources::RuntimeResources::get_locale(locale)
            .with_context(|| format!("failed to load embedded locale pack: {}", locale))?;
        let pack = crate::i18n::langpack::LanguagePack::from_bytes(&pak_data)
            .map_err(|e| anyhow::anyhow!("failed to parse locale pack {}: {}", locale, e))?;
        Ok(pack.translations)
    }
}

/// 文件系统资源提供器，供 fixture / harness / 本地工程调试使用。
pub struct FilesystemUiResourceProvider {
    root: PathBuf,
}

impl FilesystemUiResourceProvider {
    /// 基于项目根目录创建 provider。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn resolve(&self, path: &str) -> PathBuf {
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.root.join(candidate)
        }
    }
}

impl UiResourceProvider for FilesystemUiResourceProvider {
    fn load_layout(&self, path: &str) -> Result<String> {
        let resolved = self.resolve(path);
        std::fs::read_to_string(&resolved).with_context(|| {
            format!(
                "failed to read layout from filesystem: {}",
                resolved.display()
            )
        })
    }

    fn load_locale_strings(&self, locale: &str) -> Result<HashMap<String, String>> {
        let resolved = self.root.join("locales").join(format!("{}.json", locale));
        let content = std::fs::read_to_string(&resolved).with_context(|| {
            format!(
                "failed to read locale json from filesystem: {}",
                resolved.display()
            )
        })?;
        let json_value: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse locale json: {}", resolved.display()))?;

        let strings = json_value
            .get("strings")
            .and_then(|v| v.as_object())
            .or_else(|| json_value.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|text| (key.clone(), text.to_string()))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();

        Ok(strings)
    }
}
