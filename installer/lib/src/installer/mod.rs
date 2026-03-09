// 安装逻辑模块

pub mod engine;
pub mod state;
pub mod tasks;
pub mod task_runner;
pub mod registry;
pub mod extractor;
pub mod shortcuts;
pub mod channel;
pub mod launcher;

#[cfg(windows)]
pub mod windows;

pub use engine::InstallEngine;
pub use state::{InstallProgress, InstallState};
pub use tasks::InstallTask;
pub use registry::RegistryOps;
pub use extractor::{SevenZipExtractor, ExtractionResult, ExtractionStatus};
pub use shortcuts::ShortcutCreator;
pub use channel::ChannelManager;
pub use launcher::AppLauncher;
pub use task_runner::{TaskRunner, TaskConfig};
