// DPI-aware 资源加载
// 根据系统 DPI 自动选择 1x 或 2x 图片

use std::path::PathBuf;

/// 资源加载器
pub struct AssetLoader {
    scale_factor: f32,
}

impl AssetLoader {
    /// 创建新的资源加载器
    /// scale_factor: DPI 缩放因子（1.0 = 96 DPI, 2.0 = 192 DPI）
    pub fn new(scale_factor: f32) -> Self {
        Self { scale_factor }
    }

    /// 从系统 DPI 创建加载器
    #[cfg(windows)]
    pub fn from_system_dpi() -> Self {
        let scale_factor = Self::get_system_scale_factor();
        Self::new(scale_factor)
    }

    #[cfg(not(windows))]
    pub fn from_system_dpi() -> Self {
        // 非 Windows 平台默认使用 1x
        Self::new(1.0)
    }

    /// 获取系统 DPI 缩放因子
    fn get_system_scale_factor() -> f32 {
        crate::common::platform::system_dpi() as f32 / 96.0
    }

    /// 加载图片，自动根据 DPI 选择 1x 或 2x
    ///
    /// # 逻辑
    /// - scale_factor < 1.5: 使用 1x 图片
    /// - scale_factor >= 1.5: 优先使用 2x 图片，如果不存在则回退到 1x
    ///
    /// # 示例
    /// ```
    /// let loader = AssetLoader::new(2.0);
    /// // "assets/logo.png" -> "assets/logo@2x.png" (如果存在)
    /// let path = loader.get_asset_path("assets/logo.png");
    /// ```
    pub fn get_asset_path(&self, base_path: &str) -> PathBuf {
        // 如果 scale_factor >= 1.5，尝试加载 2x 图片
        if self.scale_factor >= 1.5 {
            let path_2x = self.get_2x_path(base_path);
            if path_2x.exists() {
                return path_2x;
            }
        }

        // 否则使用 1x 图片（默认）
        PathBuf::from(base_path)
    }

    /// 获取 2x 图片路径
    /// "assets/logo.png" -> "assets/logo@2x.png"
    fn get_2x_path(&self, base_path: &str) -> PathBuf {
        let path = PathBuf::from(base_path);

        if let Some(stem) = path.file_stem() {
            if let Some(parent) = path.parent() {
                if let Some(ext) = path.extension() {
                    let stem_str = stem.to_string_lossy();
                    let ext_str = ext.to_string_lossy();
                    return parent.join(format!("{}@2x.{}", stem_str, ext_str));
                }
            }
        }

        // 如果解析失败，返回原路径
        path
    }

    /// 获取当前缩放因子
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// 检查是否应该使用高清图片
    pub fn should_use_hidpi(&self) -> bool {
        self.scale_factor >= 1.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_2x_path() {
        let loader = AssetLoader::new(2.0);

        let path = loader.get_2x_path("assets/logo.png");
        assert_eq!(path, PathBuf::from("assets/logo@2x.png"));

        let path = loader.get_2x_path("assets/icons/btn_primary.png");
        assert_eq!(path, PathBuf::from("assets/icons/btn_primary@2x.png"));
    }

    #[test]
    fn test_should_use_hidpi() {
        assert!(!AssetLoader::new(1.0).should_use_hidpi());
        assert!(!AssetLoader::new(1.25).should_use_hidpi());
        assert!(AssetLoader::new(1.5).should_use_hidpi());
        assert!(AssetLoader::new(2.0).should_use_hidpi());
        assert!(AssetLoader::new(3.0).should_use_hidpi());
    }
}
