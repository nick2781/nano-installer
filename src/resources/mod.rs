// 资源管理模块

pub mod manifest;
pub mod payload;

pub use manifest::{InstallManifest, UninstallManifest};
pub use payload::PayloadExtractor;
