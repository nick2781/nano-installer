// 安装逻辑模块

pub mod engine;
pub mod state;
pub mod tasks;

#[cfg(windows)]
pub mod windows;

pub use engine::InstallEngine;
pub use state::{InstallProgress, InstallState};
pub use tasks::InstallTask;
