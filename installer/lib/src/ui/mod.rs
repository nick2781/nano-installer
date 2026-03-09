// UI 模块（基于 egui + eframe）

pub mod dpi_handler;
// pub mod egui_app;  // 旧的硬编码实现 (已废弃)
pub mod egui_app_xml;  // 基于 XML 布局的新实现
pub mod layout_renderer;
pub mod message_box;
pub mod style_engine;
pub mod wizard;

// 使用基于 XML 的新实现
pub use egui_app_xml::InstallerApp;
// pub use egui_app::InstallerApp;  // 旧的硬编码实现
pub use wizard::{Wizard, WizardMode};
pub use dpi_handler::{DpiConfig, ResourceCache};
pub use layout_renderer::{LayoutRenderer, RenderResult, InteractionState};
pub use message_box::{MessageBoxManager, MessageBoxConfig, MessageBoxType, MessageBoxButton, MessageBoxResult};
pub use style_engine::{StyleEngine, StyleConfig, StyleType};

// 注意：image_button 是 egui_app 的私有子模块
