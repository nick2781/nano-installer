// 语言选择页面
// 注意：当前设计基于 NSIS，没有独立的语言选择页
// 语言选择可以集成到配置页中，或者在启动时自动检测

use gpui::*;

pub struct LanguagePage {
    selected_locale: String,
}

impl LanguagePage {
    pub fn new(default_locale: String) -> Self {
        Self {
            selected_locale: default_locale,
        }
    }

    pub fn selected_locale(&self) -> &str {
        &self.selected_locale
    }
}

// GPUI 渲染实现将在实际集成时完成
// 当前先预留结构
