// UI 模块（基于 egui + eframe）

pub mod dpi_handler;
// pub mod egui_app;  // 旧的硬编码实现 (已废弃)
pub mod egui_app_xml; // 基于 XML 布局的新实现
pub mod folder_dialog;
pub mod layout_renderer;
pub mod message_box;
pub mod resource_provider;
pub mod style_engine;
pub mod test_harness;
pub mod wizard;

// 使用基于 XML 的新实现
pub use egui_app_xml::InstallerApp;
// pub use egui_app::InstallerApp;  // 旧的硬编码实现
pub use dpi_handler::{DpiConfig, ResourceCache};
pub use layout_renderer::{InteractionState, LayoutRenderer, RenderResult};
pub use message_box::{
    MessageBoxButton, MessageBoxConfig, MessageBoxManager, MessageBoxResult, MessageBoxType,
};
pub use resource_provider::{
    FilesystemUiResourceProvider, RuntimeUiResourceProvider, SharedUiResourceProvider,
    UiResourceProvider,
};
pub use style_engine::{StyleConfig, StyleEngine, StyleType};
pub use test_harness::{LayoutSnapshot, UiHarness};
pub use wizard::{Wizard, WizardMode};

// 注意：image_button 是 egui_app 的私有子模块
