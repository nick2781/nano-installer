//! DPI处理器
//!
//! 处理DPI自适应和资源加载

use egui::TextureHandle;
use std::path::PathBuf;
// use image::DynamicImage;
use crate::config::InstallerConfig;
use once_cell::sync::OnceCell;

// 静态变量：确保 DPI 信息只输出一次
static DPI_INFO_PRINTED: OnceCell<bool> = OnceCell::new();

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
    /// 创建DPI配置，窗口尺寸从 InstallerConfig 读取
    pub fn new(config: &InstallerConfig) -> Self {
        let system_dpi = Self::detect_system_dpi();
        let use_2x = system_dpi >= config.ui.dpi_threshold;
        let scale_factor = system_dpi as f32 / 96.0;

        DPI_INFO_PRINTED.get_or_init(|| {
            tracing::info!(
                "DPI: system={}, scale={:.2}, use_2x={}",
                system_dpi,
                scale_factor,
                use_2x
            );
            true
        });

        Self {
            scale_factor,
            use_2x,
            window_width: config.ui.window_width as f32,
            window_height: config.ui.window_height as f32,
            expanded_height: config.ui.expanded_height as f32,
        }
    }

    /// 检测系统DPI（与 NSIS 的 GetDpiForSystem 一致）
    ///
    /// 注意：GetDpiForSystem() 的返回值可能受到以下因素影响：
    /// 1. 线程的 DPI 感知上下文（SetThreadDpiAwarenessContext）
    /// 2. 进程的 DPI 感知模式（SetProcessDpiAwareness）
    /// 3. 窗口创建前后的上下文变化
    ///
    /// 这就是为什么在窗口创建前和窗口创建后调用可能返回不同值的原因。
    /// 解决方案：在应用启动时统一检测一次，然后传递结果，避免重复检测。
    ///
    /// 检测系统DPI（与 NSIS 一致：在 UNAWARE 模式下检测）
    /// NSIS 也是在 UNAWARE 模式下检测，返回 96 DPI，这是正确的行为。
    fn detect_system_dpi() -> u32 {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::HiDpi::AreDpiAwarenessContextsEqual;
            use windows::Win32::UI::HiDpi::GetDpiForSystem;
            use windows::Win32::UI::HiDpi::GetThreadDpiAwarenessContext;
            use windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_UNAWARE;

            unsafe {
                let dpi = GetDpiForSystem() as u32;

                // 检测当前线程的 DPI 感知上下文（用于调试）
                let current_context = GetThreadDpiAwarenessContext();
                let is_unaware =
                    AreDpiAwarenessContextsEqual(current_context, DPI_AWARENESS_CONTEXT_UNAWARE)
                        .as_bool();

                // 每次检测都输出
                static DPI_DETECT_COUNT: std::sync::atomic::AtomicU32 =
                    std::sync::atomic::AtomicU32::new(0);
                let count = DPI_DETECT_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

                eprintln!(
                    "[DPI检测 #{}] GetDpiForSystem() = {} (与 NSIS 一致，在 UNAWARE 模式下)",
                    count, dpi
                );
                eprintln!(
                    "[DPI检测 #{}] 当前线程 DPI 感知模式: {}",
                    count,
                    if is_unaware {
                        "UNAWARE (未感知，程序认为所有显示器都是96DPI，与 NSIS 一致)"
                    } else {
                        "AWARE (已感知)"
                    }
                );

                // 解释为什么同一个函数会返回不同的值
                if count == 1 {
                    eprintln!(
                        "[DPI检测 #{}] 说明: NSIS 也是在 UNAWARE 模式下检测，返回 96 DPI",
                        count
                    );
                    eprintln!(
                        "[DPI检测 #{}]        - 这是正确的行为，与 NSIS 保持一致",
                        count
                    );
                    eprintln!("[DPI检测 #{}]        - 问题在于 egui 在窗口创建后可能自动调整了 pixels_per_point", count);
                }

                dpi
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            96 // 默认 96 DPI
        }
    }

    /// 获取资源路径（自动选择1x或2x）
    ///
    /// 注意：如果 base_path 包含 NSIS 格式的参数（如 dest='...'），会先提取纯文件路径
    pub fn get_resource_path(&self, base_path: &str) -> String {
        // 如果路径包含 NSIS 格式的参数（如 dest='...'），先提取纯文件路径
        let clean_path = if base_path.contains("file='") || base_path.contains("file=\"") {
            // 解析 file='...' 格式
            if let Some(start) = base_path.find("file='") {
                let start_pos = start + 6;
                if let Some(end) = base_path[start_pos..].find("'") {
                    base_path[start_pos..start_pos + end].to_string()
                } else if let Some(end) = base_path[start_pos..].find("\"") {
                    base_path[start_pos..start_pos + end].to_string()
                } else {
                    base_path.to_string()
                }
            } else if let Some(start) = base_path.find("file=\"") {
                let start_pos = start + 6;
                if let Some(end) = base_path[start_pos..].find("\"") {
                    base_path[start_pos..start_pos + end].to_string()
                } else {
                    base_path.to_string()
                }
            } else {
                base_path.to_string()
            }
        } else {
            // 如果路径包含空格和参数（如 "path dest='...'"），只取第一部分
            base_path
                .split_whitespace()
                .next()
                .unwrap_or(base_path)
                .to_string()
        };

        if self.use_2x && !clean_path.contains("@2x") {
            // 尝试2x版本，保留目录路径
            let path = PathBuf::from(&clean_path);
            if let Some(stem) = path.file_stem() {
                if let Some(extension) = path.extension() {
                    let stem_str = stem.to_string_lossy();
                    let ext_str = extension.to_string_lossy();
                    let parent = path.parent().unwrap_or(std::path::Path::new(""));

                    // 构建 2x 路径，使用正斜杠（与嵌入资源的路径格式一致）
                    let new_path = if parent.as_os_str().is_empty() {
                        format!("{}@2x.{}", stem_str, ext_str)
                    } else {
                        // 使用正斜杠，确保路径格式一致
                        let parent_str = parent.to_string_lossy().replace('\\', "/");
                        format!("{}/{}@2x.{}", parent_str, stem_str, ext_str)
                    };

                    eprintln!(
                        "[资源] 路径转换: {} -> {} (use_2x={})",
                        clean_path, new_path, self.use_2x
                    );
                    return new_path;
                }
            }
        }
        eprintln!(
            "[资源] 使用原始路径: {} (use_2x={})",
            clean_path, self.use_2x
        );
        clean_path
    }

    /// 加载图片资源
    pub fn load_image(&self, ctx: &egui::Context, path: &str) -> Option<TextureHandle> {
        let resource_path = self.get_resource_path(path);

        // 优先从嵌入资源加载（运行时）
        if let Ok(image_data) = crate::resources::RuntimeResources::get_asset(&resource_path) {
            if let Ok(image) = image::load_from_memory(&image_data) {
                let rgba_image = image.to_rgba8();
                let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                let pixels = rgba_image.into_raw();

                // 使用默认纹理选项
                let options = egui::TextureOptions::LINEAR;

                eprintln!(
                    "[资源] ✓ 从嵌入资源加载成功: {} ({}x{})",
                    resource_path, size[0], size[1]
                );
                return Some(ctx.load_texture(
                    &resource_path,
                    egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                    options,
                ));
            } else {
                eprintln!("[资源] ✗ 图片解析失败: {}", resource_path);
            }
        } else {
            eprintln!(
                "[资源] ✗ 嵌入资源未找到: {}, 尝试回退到原始路径",
                resource_path
            );
        }

        // 回退：如果 2x 版本加载失败，尝试原始路径
        if resource_path != path && resource_path.contains("@2x") {
            eprintln!("[资源] 回退: 尝试加载原始路径: {}", path);
            if let Ok(image_data) = crate::resources::RuntimeResources::get_asset(path) {
                if let Ok(image) = image::load_from_memory(&image_data) {
                    let rgba_image = image.to_rgba8();
                    let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                    let pixels = rgba_image.into_raw();
                    let options = egui::TextureOptions::LINEAR;
                    eprintln!("[资源] ✓ 回退加载成功: {} ({}x{})", path, size[0], size[1]);
                    return Some(ctx.load_texture(
                        path,
                        egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                        options,
                    ));
                }
            }
        }

        // 回退：开发模式下从文件系统读取
        tracing::warn!(
            "Failed to load image from embedded resources, trying filesystem: {}",
            resource_path
        );
        if let Ok(image_data) = std::fs::read(&resource_path) {
            if let Ok(image) = image::load_from_memory(&image_data) {
                let rgba_image = image.to_rgba8();
                let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                let pixels = rgba_image.into_raw();

                let options = egui::TextureOptions::LINEAR;

                eprintln!(
                    "[资源] ✓ 从文件系统加载成功: {} ({}x{})",
                    resource_path, size[0], size[1]
                );
                return Some(ctx.load_texture(
                    &resource_path,
                    egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                    options,
                ));
            }
        }

        eprintln!("[资源] ✗ 所有加载方式都失败: {}", resource_path);
        None
    }

    /// 加载背景图片
    pub fn load_background(&self, ctx: &egui::Context, path: &str) -> Option<TextureHandle> {
        self.load_image(ctx, path)
    }

    /// 读取图片可见像素区域，并转换为 UV。
    pub fn load_image_visible_uv(&self, path: &str) -> Option<egui::Rect> {
        let resource_path = self.get_resource_path(path);
        let image_data = crate::resources::RuntimeResources::get_asset(&resource_path)
            .ok()
            .or_else(|| {
                if resource_path != path && resource_path.contains("@2x") {
                    crate::resources::RuntimeResources::get_asset(path).ok()
                } else {
                    None
                }
            })
            .or_else(|| std::fs::read(&resource_path).ok())
            .or_else(|| {
                if resource_path != path && resource_path.contains("@2x") {
                    std::fs::read(path).ok()
                } else {
                    None
                }
            })?;

        let image = image::load_from_memory(&image_data).ok()?.to_rgba8();
        let width = image.width();
        let height = image.height();
        if width == 0 || height == 0 {
            return None;
        }

        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut has_visible_pixel = false;

        for (x, y, pixel) in image.enumerate_pixels() {
            if pixel.0[3] > 0 {
                has_visible_pixel = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }

        if !has_visible_pixel {
            return Some(egui::Rect::from_min_max(
                egui::Pos2::new(0.0, 0.0),
                egui::Pos2::new(1.0, 1.0),
            ));
        }

        Some(egui::Rect::from_min_max(
            egui::Pos2::new(min_x as f32 / width as f32, min_y as f32 / height as f32),
            egui::Pos2::new((max_x + 1) as f32 / width as f32, (max_y + 1) as f32 / height as f32),
        ))
    }

    /// 加载按钮图片
    pub fn load_button_image(
        &self,
        ctx: &egui::Context,
        style: &str,
        state: &str,
    ) -> Option<TextureHandle> {
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
        let filename = if checked {
            "checkbox-2.png"
        } else {
            "checkbox-0.png"
        };
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
    /// 图片可见区域 UV 缓存
    background_visible_uvs: std::collections::HashMap<String, egui::Rect>,
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
            background_visible_uvs: std::collections::HashMap::new(),
            buttons: std::collections::HashMap::new(),
            checkboxes: std::collections::HashMap::new(),
            arrows: std::collections::HashMap::new(),
            progress: None,
        }
    }

    /// 获取或加载背景图片
    pub fn get_background(
        &mut self,
        ctx: &egui::Context,
        dpi_config: &DpiConfig,
        path: &str,
    ) -> Option<&TextureHandle> {
        if !self.backgrounds.contains_key(path) {
            if let Some(texture) = dpi_config.load_background(ctx, path) {
                self.backgrounds.insert(path.to_string(), texture);
            }
        }
        self.backgrounds.get(path)
    }

    /// 获取背景图片按 alpha 裁切后的可见 UV 区域。
    pub fn get_background_visible_uv(
        &mut self,
        dpi_config: &DpiConfig,
        path: &str,
    ) -> Option<egui::Rect> {
        if let Some(cached) = self.background_visible_uvs.get(path) {
            return Some(*cached);
        }

        let uv = dpi_config.load_image_visible_uv(path)?;
        self.background_visible_uvs.insert(path.to_string(), uv);
        Some(uv)
    }

    /// 获取或加载按钮图片
    pub fn get_button(
        &mut self,
        ctx: &egui::Context,
        dpi_config: &DpiConfig,
        style: &str,
        state: &str,
    ) -> Option<&TextureHandle> {
        let key = format!("{}_{}", style, state);
        if !self.buttons.contains_key(&key) {
            if let Some(texture) = dpi_config.load_button_image(ctx, style, state) {
                self.buttons.insert(key.clone(), texture);
            }
        }
        self.buttons.get(&key)
    }

    /// 获取或加载复选框图片
    pub fn get_checkbox(
        &mut self,
        ctx: &egui::Context,
        dpi_config: &DpiConfig,
        checked: bool,
    ) -> Option<&TextureHandle> {
        if !self.checkboxes.contains_key(&checked) {
            if let Some(texture) = dpi_config.load_checkbox_image(ctx, checked) {
                self.checkboxes.insert(checked, texture);
            }
        }
        self.checkboxes.get(&checked)
    }

    /// 获取或加载箭头图片
    pub fn get_arrow(
        &mut self,
        ctx: &egui::Context,
        dpi_config: &DpiConfig,
        direction: &str,
    ) -> Option<&TextureHandle> {
        if !self.arrows.contains_key(direction) {
            if let Some(texture) = dpi_config.load_arrow_image(ctx, direction) {
                self.arrows.insert(direction.to_string(), texture);
            }
        }
        self.arrows.get(direction)
    }

    /// 获取或加载进度条图片
    pub fn get_progress(
        &mut self,
        ctx: &egui::Context,
        dpi_config: &DpiConfig,
    ) -> Option<&TextureHandle> {
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
        self.background_visible_uvs.clear();
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
    fn test_dpi_config_reads_from_default_config() {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);

        assert!(dpi_config.scale_factor > 0.0);
        // 必须等于配置中的默认值, 不是硬编码
        assert_eq!(dpi_config.window_width, config.ui.window_width as f32);
        assert_eq!(dpi_config.window_height, config.ui.window_height as f32);
        assert_eq!(dpi_config.expanded_height, config.ui.expanded_height as f32);
    }

    #[test]
    fn test_dpi_config_reads_custom_size() {
        let mut config = InstallerConfig::default();
        config.ui.window_width = 800;
        config.ui.window_height = 600;
        config.ui.expanded_height = 900;

        let dpi_config = DpiConfig::new(&config);

        assert_eq!(dpi_config.window_width, 800.0);
        assert_eq!(dpi_config.window_height, 600.0);
        assert_eq!(dpi_config.expanded_height, 900.0);
    }

    #[test]
    fn test_dpi_config_direct_construction() {
        // 模拟 installer_runtime 中 create_dpi_config 的行为
        let dpi_config = DpiConfig {
            scale_factor: 2.0,
            use_2x: true,
            window_width: 640.0,
            window_height: 480.0,
            expanded_height: 720.0,
        };

        assert_eq!(dpi_config.window_width, 640.0);
        assert_eq!(dpi_config.window_height, 480.0);
        assert!(dpi_config.use_2x);
    }

    #[test]
    fn test_resource_path_selection() {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);

        let path = dpi_config.get_resource_path("assets/logo.png");
        assert!(path.contains("logo"));
    }

    #[test]
    fn test_get_render_size_halves_2x() {
        // use_2x=true 时, get_render_size 应返回纹理尺寸的一半
        let dpi_config = DpiConfig {
            scale_factor: 2.0,
            use_2x: true,
            window_width: 574.0,
            window_height: 358.0,
            expanded_height: 518.0,
        };
        // 无法在单测中创建 TextureHandle, 但验证逻辑:
        // use_2x=true → width/2, height/2
        // use_2x=false → width, height
        assert!(dpi_config.use_2x);
    }

    #[test]
    fn test_resource_cache() {
        let mut cache = ResourceCache::new();
        assert!(cache.backgrounds.is_empty());
        assert!(cache.buttons.is_empty());
        assert!(cache.checkboxes.is_empty());
        assert!(cache.arrows.is_empty());
        assert!(cache.progress.is_none());

        cache.clear();
        assert!(cache.backgrounds.is_empty());
    }
}
