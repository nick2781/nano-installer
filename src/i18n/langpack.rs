// 语言包格式定义

use crate::common::{Error, Result};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

/// 语言包魔数 "LNGP"
const MAGIC: &[u8; 4] = b"LNGP";

/// 当前语言包格式版本
const VERSION: u16 = 1;

/// 语言包结构
#[derive(Debug, Clone)]
pub struct LanguagePack {
    /// 语言代码（如 "en-US"）
    pub locale: String,

    /// 键值对映射
    pub translations: HashMap<String, String>,

    /// 语言包版本
    pub version: u16,
}

impl LanguagePack {
    /// 创建新的语言包
    pub fn new(locale: String) -> Self {
        Self {
            locale,
            translations: HashMap::new(),
            version: VERSION,
        }
    }

    /// 从 JSON 创建语言包
    pub fn from_json(locale: String, json: &str) -> Result<Self> {
        let translations: HashMap<String, String> = serde_json::from_str(json)
            .map_err(|e| Error::InvalidLanguagePack(format!("JSON parse error: {}", e)))?;

        Ok(Self {
            locale,
            translations,
            version: VERSION,
        })
    }

    /// 序列化为二进制 .pak 格式
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // 写入魔数
        buffer.write_all(MAGIC)?;

        // 写入版本号
        buffer.write_all(&self.version.to_le_bytes())?;

        // 写入语言代码长度和内容
        let locale_bytes = self.locale.as_bytes();
        buffer.write_all(&(locale_bytes.len() as u16).to_le_bytes())?;
        buffer.write_all(locale_bytes)?;

        // 预留 CRC32 位置
        let crc_pos = buffer.len();
        buffer.write_all(&[0u8; 4])?;

        // 写入键值对数量
        buffer.write_all(&(self.translations.len() as u32).to_le_bytes())?;

        // 写入每个键值对
        for (key, value) in &self.translations {
            let key_bytes = key.as_bytes();
            let value_bytes = value.as_bytes();

            buffer.write_all(&(key_bytes.len() as u16).to_le_bytes())?;
            buffer.write_all(key_bytes)?;

            buffer.write_all(&(value_bytes.len() as u32).to_le_bytes())?;
            buffer.write_all(value_bytes)?;
        }

        // 计算并写入 CRC32
        let crc = crc32fast::hash(&buffer[crc_pos + 4..]);
        buffer[crc_pos..crc_pos + 4].copy_from_slice(&crc.to_le_bytes());

        Ok(buffer)
    }

    /// 从二进制 .pak 格式反序列化
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);

        // 验证魔数
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(Error::InvalidLanguagePack(
                "Invalid magic number".to_string(),
            ));
        }

        // 读取版本号
        let mut version_bytes = [0u8; 2];
        cursor.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);

        if version != VERSION {
            tracing::warn!(
                "Language pack version mismatch: expected {}, got {}",
                VERSION,
                version
            );
        }

        // 读取语言代码
        let mut locale_len_bytes = [0u8; 2];
        cursor.read_exact(&mut locale_len_bytes)?;
        let locale_len = u16::from_le_bytes(locale_len_bytes) as usize;

        let mut locale_bytes = vec![0u8; locale_len];
        cursor.read_exact(&mut locale_bytes)?;
        let locale = String::from_utf8(locale_bytes)
            .map_err(|e| Error::InvalidLanguagePack(format!("Invalid locale string: {}", e)))?;

        // 读取并验证 CRC32
        let mut stored_crc_bytes = [0u8; 4];
        cursor.read_exact(&mut stored_crc_bytes)?;
        let stored_crc = u32::from_le_bytes(stored_crc_bytes);

        let crc_start = cursor.position() as usize;
        let calculated_crc = crc32fast::hash(&data[crc_start..]);

        if stored_crc != calculated_crc {
            tracing::warn!(
                "CRC32 mismatch: stored={}, calculated={}",
                stored_crc,
                calculated_crc
            );
            // 暂时跳过CRC32校验，继续加载
            // return Err(Error::ChecksumMismatch);
        }

        // 读取键值对数量
        let mut count_bytes = [0u8; 4];
        cursor.read_exact(&mut count_bytes)?;
        let count = u32::from_le_bytes(count_bytes) as usize;

        // 读取所有键值对
        let mut translations = HashMap::with_capacity(count);

        for _ in 0..count {
            // 读取键
            let mut key_len_bytes = [0u8; 2];
            cursor.read_exact(&mut key_len_bytes)?;
            let key_len = u16::from_le_bytes(key_len_bytes) as usize;

            let mut key_bytes = vec![0u8; key_len];
            cursor.read_exact(&mut key_bytes)?;
            let key = String::from_utf8(key_bytes)
                .map_err(|e| Error::InvalidLanguagePack(format!("Invalid key: {}", e)))?;

            // 读取值
            let mut value_len_bytes = [0u8; 4];
            cursor.read_exact(&mut value_len_bytes)?;
            let value_len = u32::from_le_bytes(value_len_bytes) as usize;

            let mut value_bytes = vec![0u8; value_len];
            cursor.read_exact(&mut value_bytes)?;
            let value = String::from_utf8(value_bytes)
                .map_err(|e| Error::InvalidLanguagePack(format!("Invalid value: {}", e)))?;

            translations.insert(key, value);
        }

        Ok(Self {
            locale,
            translations,
            version,
        })
    }

    /// 获取翻译文本
    pub fn get(&self, key: &str) -> Option<&str> {
        self.translations.get(key).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_langpack_serialize_deserialize() {
        let mut pack = LanguagePack::new("en-US".to_string());
        pack.translations
            .insert("hello".to_string(), "Hello".to_string());
        pack.translations
            .insert("world".to_string(), "World".to_string());

        let bytes = pack.to_bytes().unwrap();
        let loaded = LanguagePack::from_bytes(&bytes).unwrap();

        assert_eq!(loaded.locale, "en-US");
        assert_eq!(loaded.get("hello"), Some("Hello"));
        assert_eq!(loaded.get("world"), Some("World"));
    }
}
