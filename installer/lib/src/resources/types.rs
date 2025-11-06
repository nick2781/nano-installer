use std::collections::HashMap;
use egui::TextureHandle;

/// 资源类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ResourceType {
    /// 图片资源
    Image,
    /// 文本资源
    Text,
    /// 压缩包资源
    Archive,
    /// 字体资源
    Font,
    /// 二进制资源
    Binary,
}

/// 资源信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceInfo {
    pub name: String,
    pub resource_type: ResourceType,
    pub data: Vec<u8>,
    pub mime_type: Option<String>,
    pub size: usize,
}

/// 资源缓存
pub struct ResourceCache {
    /// 图片资源缓存
    images: HashMap<String, TextureHandle>,
    /// 文本资源缓存
    texts: HashMap<String, String>,
    /// 二进制资源缓存
    binaries: HashMap<String, Vec<u8>>,
}

impl ResourceCache {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
            texts: HashMap::new(),
            binaries: HashMap::new(),
        }
    }

    /// 获取图片资源
    pub fn get_image(&self, name: &str) -> Option<&TextureHandle> {
        self.images.get(name)
    }

    /// 缓存图片资源
    pub fn cache_image(&mut self, name: String, texture: TextureHandle) {
        self.images.insert(name, texture);
    }

    /// 获取文本资源
    pub fn get_text(&self, name: &str) -> Option<&String> {
        self.texts.get(name)
    }

    /// 缓存文本资源
    pub fn cache_text(&mut self, name: String, text: String) {
        self.texts.insert(name, text);
    }

    /// 获取二进制资源
    pub fn get_binary(&self, name: &str) -> Option<&Vec<u8>> {
        self.binaries.get(name)
    }

    /// 缓存二进制资源
    pub fn cache_binary(&mut self, name: String, data: Vec<u8>) {
        self.binaries.insert(name, data);
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.images.clear();
        self.texts.clear();
        self.binaries.clear();
    }
}
