// 集成测试

#[cfg(test)]
mod tests {
    use nano_installer::i18n::LanguagePack;
    use nano_installer::common::config::InstallerConfig;
    
    #[test]
    fn test_config_default() {
        let config = InstallerConfig::default();
        assert_eq!(config.default_locale, "en-US");
        assert_eq!(config.supported_locales.len(), 5);
    }
    
    #[test]
    fn test_language_pack_serialization() {
        let mut pack = LanguagePack::new("test".to_string());
        pack.translations.insert("key1".to_string(), "value1".to_string());
        pack.translations.insert("key2".to_string(), "value2".to_string());
        
        // 序列化
        let bytes = pack.to_bytes().expect("Failed to serialize");
        
        // 反序列化
        let loaded = LanguagePack::from_bytes(&bytes).expect("Failed to deserialize");
        
        assert_eq!(loaded.locale, "test");
        assert_eq!(loaded.translations.get("key1"), Some(&"value1".to_string()));
        assert_eq!(loaded.translations.get("key2"), Some(&"value2".to_string()));
    }
    
    #[test]
    fn test_language_pack_from_json() {
        let json = r#"{"hello": "Hello", "world": "World"}"#;
        let pack = LanguagePack::from_json("en-US".to_string(), json)
            .expect("Failed to create from JSON");
        
        assert_eq!(pack.locale, "en-US");
        assert_eq!(pack.translations.len(), 2);
        assert_eq!(pack.get("hello"), Some("Hello"));
        assert_eq!(pack.get("world"), Some("World"));
    }
    
    #[test]
    fn test_supported_locales() {
        use nano_installer::i18n;
        
        assert!(i18n::is_locale_supported("en-US"));
        assert!(i18n::is_locale_supported("zh-CN"));
        assert!(i18n::is_locale_supported("zh-TW"));
        assert!(i18n::is_locale_supported("ja"));
        assert!(i18n::is_locale_supported("vi"));
        assert!(!i18n::is_locale_supported("fr"));
    }
    
    #[test]
    fn test_locale_native_names() {
        use nano_installer::i18n;
        
        assert_eq!(i18n::get_locale_native_name("en-US"), "English");
        assert_eq!(i18n::get_locale_native_name("zh-CN"), "简体中文");
        assert_eq!(i18n::get_locale_native_name("zh-TW"), "繁體中文");
        assert_eq!(i18n::get_locale_native_name("ja"), "日本語");
        assert_eq!(i18n::get_locale_native_name("vi"), "Tiếng Việt");
    }
}

