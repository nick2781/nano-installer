// 语言加载器

use super::{LanguageBundle, LanguagePack};
use crate::common::{Error, Result};
use std::borrow::Cow;

/// 语言加载器
pub struct LanguageLoader {
    bundle: LanguageBundle,
    default_locale: String,
}

impl LanguageLoader {
    /// 创建新的语言加载器
    pub fn new() -> Self {
        Self {
            bundle: LanguageBundle::new(),
            default_locale: "en-US".to_string(),
        }
    }

    /// 从嵌入的资源加载语言包
    pub fn load_locale(&mut self, locale: &str) -> Result<()> {
        if self.bundle.has_locale(locale) {
            return Ok(());
        }

        // 尝试从嵌入的资源加载
        let pack_data = self.load_embedded_pack(locale)?;
        let pack = LanguagePack::from_bytes(&pack_data)?;

        self.bundle.add_pack(pack);
        Ok(())
    }

    /// 加载嵌入的语言包数据
    fn load_embedded_pack(&self, locale: &str) -> Result<Vec<u8>> {
        // 这里会在编译时嵌入语言包
        // 暂时返回错误，实际实现会使用 include_bytes! 宏
        match locale {
            "en-US" => {
                #[cfg(feature = "embedded-locales")]
                {
                    Ok(include_bytes!(concat!(env!("OUT_DIR"), "/locales/en-US.pak")).to_vec())
                }
                #[cfg(not(feature = "embedded-locales"))]
                {
                    Err(Error::LanguagePackNotFound(locale.to_string()))
                }
            }
            "zh-CN" => {
                #[cfg(feature = "embedded-locales")]
                {
                    Ok(include_bytes!(concat!(env!("OUT_DIR"), "/locales/zh-CN.pak")).to_vec())
                }
                #[cfg(not(feature = "embedded-locales"))]
                {
                    Err(Error::LanguagePackNotFound(locale.to_string()))
                }
            }
            "zh-TW" => {
                #[cfg(feature = "embedded-locales")]
                {
                    Ok(include_bytes!(concat!(env!("OUT_DIR"), "/locales/zh-TW.pak")).to_vec())
                }
                #[cfg(not(feature = "embedded-locales"))]
                {
                    Err(Error::LanguagePackNotFound(locale.to_string()))
                }
            }
            "ja" => {
                #[cfg(feature = "embedded-locales")]
                {
                    Ok(include_bytes!(concat!(env!("OUT_DIR"), "/locales/ja.pak")).to_vec())
                }
                #[cfg(not(feature = "embedded-locales"))]
                {
                    Err(Error::LanguagePackNotFound(locale.to_string()))
                }
            }
            "vi" => {
                #[cfg(feature = "embedded-locales")]
                {
                    Ok(include_bytes!(concat!(env!("OUT_DIR"), "/locales/vi.pak")).to_vec())
                }
                #[cfg(not(feature = "embedded-locales"))]
                {
                    Err(Error::LanguagePackNotFound(locale.to_string()))
                }
            }
            _ => Err(Error::LanguagePackNotFound(locale.to_string())),
        }
    }

    /// 设置当前语言
    pub fn set_current_locale(&mut self, locale: &str) {
        self.bundle.set_current_locale(locale);
    }

    /// 获取当前语言
    pub fn current_locale(&self) -> &str {
        self.bundle.current_locale().unwrap_or(&self.default_locale)
    }

    /// 翻译文本
    pub fn translate(&self, key: &str) -> Cow<'static, str> {
        self.bundle
            .get(key)
            .map(|s| Cow::Owned(s.to_string()))
            .unwrap_or_else(|| {
                tracing::error!("Missing translation key: {}", key);
                Cow::Owned(format!("[{}]", key))
            })
    }

    /// 带参数的翻译
    pub fn translate_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let template = self.translate(key);
        let mut result = template.into_owned();

        for (placeholder, value) in args {
            let placeholder_str = format!("{{{}}}", placeholder);
            result = result.replace(&placeholder_str, value);
        }

        result
    }
}

impl Default for LanguageLoader {
    fn default() -> Self {
        Self::new()
    }
}
