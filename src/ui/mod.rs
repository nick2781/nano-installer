// UI 模块（基于 egui + eframe）
// egui 提供更好的 Win7+ 支持和内建布局功能

pub mod egui_app;
pub mod wizard;

pub use egui_app::InstallerApp;
pub use wizard::{Wizard, WizardPage};

// 注意：image_button 是 egui_app 的私有子模块
