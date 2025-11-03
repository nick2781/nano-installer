//! DPI处理器
//! 
//! 处理DPI自适应和资源加载

use std::path::PathBuf;
use egui::TextureHandle;
// use image::DynamicImage;
use crate::config::InstallerConfig;

/// DPI配置
#[derive(Debug, Clone)]
pub struct DpiConfig {
    /// 系统DPI缩放因子
    pub scale_factor: f32,
    /// 是否使用2x资源
    pub use_2x: bool,
    /// 窗口宽度（逻辑像素）
    pub window_width: f32,
    /// 窗口高度（逻辑像素）
    pub window_height: f32,
    /// 窗口展开高度（逻辑像素）
    pub expanded_height: f32,
}

impl DpiConfig {
    /// 创建DPI配置
    pub fn new(config: &InstallerConfig) -> Self {
        let scale_factor = Self::detect_dpi_scale();
        let use_2x = scale_factor >= config.ui.dpi_threshold as f32 / 96.0;
        
        Self {
            scale_factor,
            use_2x,
            window_width: config.ui.window_width as f32,
            window_height: config.ui.window_height as f32,
            expanded_height: config.ui.expanded_height as f32,
        }
    }

    /// 检测系统DPI缩放因子
    fn detect_dpi_scale() -> f32 {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::HiDpi::GetDpiForSystem;
            
            unsafe {
                let dpi = GetDpiForSystem();
                dpi as f32 / 96.0
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            1.0
        }
    }

    /// 获取资源路径（自动选择1x或2x）
    pub fn get_resource_path(&self, base_path: &str) -> String {
        if self.use_2x && !base_path.contains("@2x") {
            // 尝试2x版本
            let path = PathBuf::from(base_path);
            if let Some(stem) = path.file_stem() {
                if let Some(extension) = path.extension() {
                    let stem_str = stem.to_string_lossy();
                    let ext_str = extension.to_string_lossy();
                    let parent = path.parent().unwrap_or(std::path::Path::new(""));
                    let new_path = parent.join(format!("{}@2x.{}", stem_str, ext_str));
                    if new_path.exists() {
                        return new_path.to_string_lossy().to_string();
                    }
                }
            }
        }
        base_path.to_string()
    }

    /// 加载图片资源
    pub fn load_image(&self, ctx: &egui::Context, path: &str) -> Option<TextureHandle> {
        let resource_path = self.get_resource_path(path);
        
        // 尝试加载图片
        if let Ok(image_data) = std::fs::read(&resource_path) {
            if let Ok(image) = image::load_from_memory(&image_data) {
                let rgba_image = image.to_rgba8();
                let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                let pixels = rgba_image.into_raw();
                
                // 使用默认纹理选项
                let options = egui::TextureOptions::LINEAR;
                
                return Some(ctx.load_texture(
                    &resource_path,
                    egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                    options
                ));
            }
        }
        
        None
    }

    /// 加载背景图片
    pub fn load_background(&self, ctx: &egui::Context, path: &str) -> Option<TextureHandle> {
        self.load_image(ctx, path)
    }

    /// 加载按钮图片
    pub fn load_button_image(&self, ctx: &egui::Context, style: &str, state: &str) -> Option<TextureHandle> {
        let filename = match (style, state) {
            ("primary", "normal") => "btn_primary.png",
            ("primary", "hover") => "btn_hover.png", 
            ("primary", "disabled") => "btn_disabled.png",
            ("link", "normal") => "btn_dialog.png",
            ("link", "hover") => "btn_dialog_primary.png",
            ("text", _) => return None,
            _ => return None,
        };
        
        self.load_image(ctx, &format!("assets/{}", filename))
    }

    /// 加载复选框图片
    pub fn load_checkbox_image(&self, ctx: &egui::Context, checked: bool) -> Option<TextureHandle> {
        let filename = if checked { "checkbox-2.png" } else { "checkbox-0.png" };
        self.load_image(ctx, &format!("assets/{}", filename))
    }

    /// 加载箭头图片
    pub fn load_arrow_image(&self, ctx: &egui::Context, direction: &str) -> Option<TextureHandle> {
        let filename = match direction {
            "up" => "arrow-up.png",
            "down" => "arrow-down.png",
            _ => return None,
        };
        
        self.load_image(ctx, &format!("assets/{}", filename))
    }

    /// 加载进度条图片
    pub fn load_progress_image(&self, ctx: &egui::Context) -> Option<TextureHandle> {
        self.load_image(ctx, "assets/bar_installing.png")
    }

    /// 获取缩放后的尺寸
    pub fn scale_size(&self, width: f32, height: f32) -> (f32, f32) {
        (width * self.scale_factor, height * self.scale_factor)
    }

    /// 获取缩放后的位置
    pub fn scale_position(&self, x: f32, y: f32) -> (f32, f32) {
        (x * self.scale_factor, y * self.scale_factor)
    }

    /// 检查是否应该使用2x资源
    pub fn should_use_2x(&self) -> bool {
        self.use_2x
    }

    /// 获取DPI缩放因子
    pub fn get_scale_factor(&self) -> f32 {
        self.scale_factor
    }
    
    /// 获取实际渲染尺寸（如果是2x资源，返回一半的尺寸）
    pub fn get_render_size(&self, texture: &TextureHandle) -> egui::Vec2 {
        let size = texture.size();
        let width = size[0] as f32;
        let height = size[1] as f32;
        
        // 如果使用了 2x 资源，实际渲染时应该是一半的尺寸
        if self.use_2x {
            egui::Vec2::new(width / 2.0, height / 2.0)
        } else {
            egui::Vec2::new(width, height)
        }
    }
}

/// 资源缓存
pub struct ResourceCache {
    /// 背景图片缓存
    backgrounds: std::collections::HashMap<String, TextureHandle>,
    /// 按钮图片缓存
    buttons: std::collections::HashMap<String, TextureHandle>,
    /// 复选框图片缓存
    checkboxes: std::collections::HashMap<bool, TextureHandle>,
    /// 箭头图片缓存
    arrows: std::collections::HashMap<String, TextureHandle>,
    /// 进度条图片缓存
    progress: Option<TextureHandle>,
}

impl ResourceCache {
    /// 创建新的资源缓存
    pub fn new() -> Self {
        Self {
            backgrounds: std::collections::HashMap::new(),
            buttons: std::collections::HashMap::new(),
            checkboxes: std::collections::HashMap::new(),
            arrows: std::collections::HashMap::new(),
            progress: None,
        }
    }

    /// 获取或加载背景图片
    pub fn get_background(&mut self, ctx: &egui::Context, dpi_config: &DpiConfig, path: &str) -> Option<&TextureHandle> {
        if !self.backgrounds.contains_key(path) {
            if let Some(texture) = dpi_config.load_background(ctx, path) {
                self.backgrounds.insert(path.to_string(), texture);
            }
        }
        self.backgrounds.get(path)
    }

    /// 获取或加载按钮图片
    pub fn get_button(&mut self, ctx: &egui::Context, dpi_config: &DpiConfig, style: &str, state: &str) -> Option<&TextureHandle> {
        let key = format!("{}_{}", style, state);
        if !self.buttons.contains_key(&key) {
            if let Some(texture) = dpi_config.load_button_image(ctx, style, state) {
                self.buttons.insert(key.clone(), texture);
            }
        }
        self.buttons.get(&key)
    }

    /// 获取或加载复选框图片
    pub fn get_checkbox(&mut self, ctx: &egui::Context, dpi_config: &DpiConfig, checked: bool) -> Option<&TextureHandle> {
        if !self.checkboxes.contains_key(&checked) {
            if let Some(texture) = dpi_config.load_checkbox_image(ctx, checked) {
                self.checkboxes.insert(checked, texture);
            }
        }
        self.checkboxes.get(&checked)
    }

    /// 获取或加载箭头图片
    pub fn get_arrow(&mut self, ctx: &egui::Context, dpi_config: &DpiConfig, direction: &str) -> Option<&TextureHandle> {
        if !self.arrows.contains_key(direction) {
            if let Some(texture) = dpi_config.load_arrow_image(ctx, direction) {
                self.arrows.insert(direction.to_string(), texture);
            }
        }
        self.arrows.get(direction)
    }

    /// 获取或加载进度条图片
    pub fn get_progress(&mut self, ctx: &egui::Context, dpi_config: &DpiConfig) -> Option<&TextureHandle> {
        if self.progress.is_none() {
            if let Some(texture) = dpi_config.load_progress_image(ctx) {
                self.progress = Some(texture);
            }
        }
        self.progress.as_ref()
    }

    /// 清理缓存
    pub fn clear(&mut self) {
        self.backgrounds.clear();
        self.buttons.clear();
        self.checkboxes.clear();
        self.arrows.clear();
        self.progress = None;
    }
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InstallerConfig;

    #[test]
    fn test_dpi_config_creation() {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);
        
        assert!(dpi_config.scale_factor > 0.0);
        assert_eq!(dpi_config.window_width, 574.0);
        assert_eq!(dpi_config.window_height, 358.0);
    }

    #[test]
    fn test_resource_path_selection() {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);
        
        // 测试路径选择逻辑
        let path = dpi_config.get_resource_path("assets/logo.png");
        assert!(path.contains("logo"));
    }

    #[test]
    fn test_resource_cache() {
        let mut cache = ResourceCache::new();
        assert!(cache.backgrounds.is_empty());
        assert!(cache.buttons.is_empty());
        assert!(cache.checkboxes.is_empty());
        assert!(cache.arrows.is_empty());
        assert!(cache.progress.is_none());
    }
}
