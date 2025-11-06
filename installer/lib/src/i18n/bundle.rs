// 语言包集合

use super::LanguagePack;
use std::collections::HashMap;

/// 语言包集合
#[derive(Debug, Default)]
pub struct LanguageBundle {
    /// 所有已加载的语言包
    packs: HashMap<String, LanguagePack>,

    /// 当前激活的语言
    current_locale: Option<String>,
}

impl LanguageBundle {
    /// 创建新的语言包集合
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加语言包
    pub fn add_pack(&mut self, pack: LanguagePack) {
        let locale = pack.locale.clone();
        self.packs.insert(locale, pack);
    }

    /// 设置当前语言
    pub fn set_current_locale(&mut self, locale: &str) {
        self.current_locale = Some(locale.to_string());
    }

    /// 获取当前语言
    pub fn current_locale(&self) -> Option<&str> {
        self.current_locale.as_deref()
    }

    /// 获取翻译文本
    pub fn get(&self, key: &str) -> Option<&str> {
        let locale = self.current_locale.as_ref()?;

        // 尝试从当前语言获取
        if let Some(pack) = self.packs.get(locale) {
            if let Some(text) = pack.get(key) {
                return Some(text);
            }
        }

        // 回退到英语
        if locale != "en-US" {
            if let Some(pack) = self.packs.get("en-US") {
                if let Some(text) = pack.get(key) {
                    tracing::warn!(
                        "Translation key '{}' not found in {}, falling back to en-US",
                        key,
                        locale
                    );
                    return Some(text);
                }
            }
        }

        tracing::error!("Translation key '{}' not found", key);
        None
    }

    /// 检查是否有某个语言包
    pub fn has_locale(&self, locale: &str) -> bool {
        self.packs.contains_key(locale)
    }
}
