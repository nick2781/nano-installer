use crate::config::InstallerConfig;
use crate::resources::types::ResourceType;
use crate::resources::{EmbeddedResources, ResourceCache, ResourceInfo};
use anyhow::Result;
use egui::{Context, TextureHandle};
use std::path::Path;

/// 资源加载器
pub struct ResourceLoader {
    embedded_resources: EmbeddedResources,
    cache: ResourceCache,
}

impl ResourceLoader {
    /// 创建新的资源加载器
    pub fn new(_config: &InstallerConfig) -> Self {
        Self {
            embedded_resources: crate::resources::embedded::generated::get_embedded_resources(),
            cache: ResourceCache::new(),
        }
    }

    /// 加载图片资源
    pub fn load_image(&mut self, ctx: &Context, name: &str) -> Result<Option<TextureHandle>> {
        // 先检查缓存
        if let Some(texture) = self.cache.get_image(name) {
            return Ok(Some(texture.clone()));
        }

        // 从内嵌资源加载
        if let Some(resource) = self.embedded_resources.get_resource(name) {
            if resource.resource_type == ResourceType::Image {
                let image = self.load_image_from_bytes(&resource.data)?;
                let texture = ctx.load_texture(name, image, Default::default());
                self.cache.cache_image(name.to_string(), texture.clone());
                return Ok(Some(texture));
            }
        }

        Ok(None)
    }

    /// 加载文本资源
    pub fn load_text(&mut self, name: &str) -> Result<Option<String>> {
        // 先检查缓存
        if let Some(text) = self.cache.get_text(name) {
            return Ok(Some(text.clone()));
        }

        // 从内嵌资源加载
        if let Some(resource) = self.embedded_resources.get_resource(name) {
            if resource.resource_type == ResourceType::Text {
                let text = String::from_utf8(resource.data.clone())?;
                self.cache.cache_text(name.to_string(), text.clone());
                return Ok(Some(text));
            }
        }

        Ok(None)
    }

    /// 加载二进制资源
    pub fn load_binary(&mut self, name: &str) -> Result<Option<Vec<u8>>> {
        // 先检查缓存
        if let Some(data) = self.cache.get_binary(name) {
            return Ok(Some(data.clone()));
        }

        // 从内嵌资源加载
        if let Some(resource) = self.embedded_resources.get_resource(name) {
            if resource.resource_type == ResourceType::Archive
                || resource.resource_type == ResourceType::Binary
            {
                self.cache
                    .cache_binary(name.to_string(), resource.data.clone());
                return Ok(Some(resource.data.clone()));
            }
        }

        Ok(None)
    }

    /// 从字节数据加载图片
    fn load_image_from_bytes(&self, data: &[u8]) -> Result<egui::ColorImage> {
        let image = image::load_from_memory(data)?;
        let size = [image.width() as usize, image.height() as usize];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.into_raw();
        Ok(egui::ColorImage::from_rgba_unmultiplied(size, &pixels))
    }

    /// 从文件系统加载资源（开发模式）
    pub fn load_from_filesystem(
        &mut self,
        ctx: &Context,
        base_path: &Path,
        name: &str,
    ) -> Result<Option<TextureHandle>> {
        // 先检查缓存
        if let Some(texture) = self.cache.get_image(name) {
            return Ok(Some(texture.clone()));
        }

        // 尝试不同的文件扩展名
        let extensions = ["png", "jpg", "jpeg", "bmp", "gif"];
        for ext in &extensions {
            let file_path = base_path.join(format!("{}.{}", name, ext));
            if file_path.exists() {
                let image = image::open(&file_path)?;
                let size = [image.width() as usize, image.height() as usize];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                let texture = ctx.load_texture(name, color_image, Default::default());
                self.cache.cache_image(name.to_string(), texture.clone());
                return Ok(Some(texture));
            }
        }

        Ok(None)
    }

    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// 获取资源信息
    pub fn get_resource_info(&self, name: &str) -> Option<&ResourceInfo> {
        self.embedded_resources.get_resource(name)
    }

    /// 检查资源是否存在
    pub fn has_resource(&self, name: &str) -> bool {
        self.embedded_resources.has_resource(name)
    }
}
