// 多语言支持模块

pub mod bundle;
pub mod langpack;
pub mod loader;

pub use bundle::LanguageBundle;
pub use langpack::LanguagePack;
pub use loader::LanguageLoader;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::borrow::Cow;

/// 全局语言加载器
static GLOBAL_LOADER: Lazy<RwLock<LanguageLoader>> =
    Lazy::new(|| RwLock::new(LanguageLoader::new()));

/// 初始化语言系统
pub fn init(locale: &str) -> crate::common::Result<()> {
    let mut loader = GLOBAL_LOADER.write();
    loader.load_locale(locale)?;
    loader.set_current_locale(locale);
    Ok(())
}

/// 翻译函数（简写）
pub fn tr(key: &str) -> Cow<'static, str> {
    let loader = GLOBAL_LOADER.read();
    loader.translate(key)
}

/// 带参数的翻译函数
pub fn tr_with_args(key: &str, args: &[(&str, &str)]) -> String {
    let loader = GLOBAL_LOADER.read();
    loader.translate_with_args(key, args)
}

/// 获取当前语言
pub fn current_locale() -> String {
    let loader = GLOBAL_LOADER.read();
    loader.current_locale().to_string()
}

/// 切换语言
pub fn switch_locale(locale: &str) -> crate::common::Result<()> {
    let mut loader = GLOBAL_LOADER.write();
    loader.load_locale(locale)?;
    loader.set_current_locale(locale);
    Ok(())
}

/// 支持的语言列表 (11 种语言，覆盖全球主要市场)
pub const SUPPORTED_LOCALES: &[&str] = &[
    "en-US", // English
    "zh-CN", // 简体中文
    "zh-TW", // 繁体中文
    "ja",    // 日本語
    "ko",    // 한국어
    "ru",    // Русский
    "es",    // Español
    "pt",    // Português
    "vi",    // Tiếng Việt
    "th",    // ไทย
    "id",    // Bahasa Indonesia
];

/// 检查语言是否支持
pub fn is_locale_supported(locale: &str) -> bool {
    SUPPORTED_LOCALES.contains(&locale)
}

/// 获取语言的本地化名称
pub fn get_locale_native_name(locale: &str) -> &'static str {
    match locale {
        "en-US" => "English",
        "zh-CN" => "简体中文",
        "zh-TW" => "繁體中文",
        "ja" => "日本語",
        "ko" => "한국어",
        "ru" => "Русский",
        "es" => "Español",
        "pt" => "Português",
        "vi" => "Tiếng Việt",
        "th" => "ไทย",
        "id" => "Bahasa Indonesia",
        _ => "Unknown",
    }
}
