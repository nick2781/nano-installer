// UI 样式定义
// 基于 TapTap NSIS 设计

/// 颜色定义（从 NSIS 设计提取）
#[derive(Debug, Clone)]
pub struct Colors {
    /// 主背景色 #181B22
    pub background: (u8, u8, u8),

    /// 主色调（青色） #00C4B2
    pub primary: (u8, u8, u8),

    /// 悬停色（浅青） #00FFE8
    pub primary_hover: (u8, u8, u8),

    /// 按下色 #00D9C5
    pub primary_pressed: (u8, u8, u8),

    /// 主文字色（白色） #FFFFFF
    pub text: (u8, u8, u8),

    /// 次要文字色（浅灰） #E4E8EC
    pub text_secondary: (u8, u8, u8),

    /// 禁用文字色（灰色） #96A1A9
    pub text_disabled: (u8, u8, u8),

    /// 分割线颜色 #00C4B2 (33% opacity)
    pub divider: (u8, u8, u8, u8),
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            background: (24, 27, 34),        // #181B22
            primary: (0, 196, 178),          // #00C4B2
            primary_hover: (0, 255, 232),    // #00FFE8
            primary_pressed: (0, 217, 197),  // #00D9C5
            text: (255, 255, 255),           // #FFFFFF
            text_secondary: (228, 232, 236), // #E4E8EC
            text_disabled: (150, 161, 169),  // #96A1A9
            divider: (0, 196, 178, 51),      // #00C4B2 at 20% opacity
        }
    }
}

/// 字体大小
pub struct FontSizes {
    /// 小字体 12px
    pub small: f32,
    /// 标准字体 14px
    pub normal: f32,
    /// 大字体 24px
    pub large: f32,
    /// 超大字体 28px
    pub xlarge: f32,
}

impl Default for FontSizes {
    fn default() -> Self {
        Self {
            small: 12.0,
            normal: 14.0,
            large: 24.0,
            xlarge: 28.0,
        }
    }
}

/// 窗口尺寸（从 NSIS 设计）
pub const WINDOW_WIDTH: u32 = 574;
pub const WINDOW_HEIGHT: u32 = 358;

/// 展开后的高度
pub const WINDOW_HEIGHT_EXPANDED: u32 = 518;

/// 圆角半径
pub const CORNER_RADIUS: f32 = 16.0;

/// 按钮尺寸
pub const BUTTON_WIDTH: u32 = 240;
pub const BUTTON_HEIGHT: u32 = 40;

/// Logo 尺寸
pub const LOGO_WIDTH: u32 = 148;
pub const LOGO_HEIGHT: u32 = 80;

/// 资源路径（基础路径，DPI aware）
///
/// 所有资源都有两个版本：
/// - 1x: logo.png (标准 DPI，96 DPI)
/// - 2x: logo@2x.png (高 DPI，192+ DPI)
///
/// 使用 AssetLoader 自动根据系统 DPI 选择合适的版本：
/// ```
/// use crate::ui::AssetLoader;
///
/// let loader = AssetLoader::from_system_dpi();
/// let logo_path = loader.get_asset_path(assets::LOGO);
/// // 在高 DPI 屏幕上自动返回 "assets/logo@2x.png"
/// // 在标准 DPI 屏幕上返回 "assets/logo.png"
/// ```
pub mod assets {
    pub const LOGO: &str = "assets/logo.png";
    pub const BG_MAIN: &str = "assets/bg_main.png";
    pub const BG_INSTALLING: &str = "assets/bg_installing.png";
    pub const BG_COLOR: &str = "assets/bg_color.png";

    pub const BTN_PRIMARY: &str = "assets/btn_primary.png";
    pub const BTN_HOVER: &str = "assets/btn_hover.png";
    pub const BTN_DISABLED: &str = "assets/btn_disabled.png";
    pub const BTN_CLOSE: &str = "assets/btn_close.png";
    pub const BTN_DIALOG: &str = "assets/btn_dialog.png";
    pub const BTN_DIALOG_PRIMARY: &str = "assets/btn_dialog_primary.png";

    pub const CHECKBOX_UNCHECKED: &str = "assets/checkbox-0.png";
    pub const CHECKBOX_CHECKED: &str = "assets/checkbox-2.png";

    pub const ARROW_DOWN: &str = "assets/arrow-down.png";
    pub const ARROW_UP: &str = "assets/arrow-up.png";

    pub const PROGRESS_BAR: &str = "assets/bar_installing.png";
}
