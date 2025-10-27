// Nano Installer Library
// 提供安装和卸载功能的核心库

pub mod common;
pub mod i18n;
pub mod installer;
pub mod logger;
pub mod resources;
pub mod ui;
pub mod uninstaller;

// Re-exports
pub use common::{error::Error, result::Result};
