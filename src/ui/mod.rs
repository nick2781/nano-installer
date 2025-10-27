// UI 模块（基于 GPUI + gpui-component）

pub mod app;
pub mod assets;
pub mod components;
pub mod pages;
pub mod styles;
pub mod wizard;

pub use app::InstallerApp;
pub use assets::AssetLoader;
pub use wizard::{Wizard, WizardPage};

// GPUI 实现说明：
// 1. 实际渲染逻辑需要根据 gpui-component 的最新 API 实现
// 2. 参考文档：docs/GPUI_COMPONENTS_GUIDE.md
// 3. 设计规格：docs/UI_DESIGN.md
// 4. 样式定义：styles/mod.rs

// TODO: 完成实际的 GPUI Render 实现
// 当前提供了完整的结构和逻辑框架
