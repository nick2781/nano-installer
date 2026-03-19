// 安装逻辑模块

pub(crate) mod artifacts;
pub mod engine;
pub mod extractor;
pub mod launcher;
pub mod registry;
pub mod shortcuts;
pub mod state;
pub mod task_runner;
pub mod tasks;

#[cfg(windows)]
pub mod windows;

pub use engine::InstallEngine;
pub use extractor::{ExtractionResult, ExtractionStatus, SevenZipExtractor};
pub use launcher::AppLauncher;
pub use registry::RegistryOps;
pub use shortcuts::ShortcutCreator;
pub use state::{InstallProgress, InstallState};
pub use task_runner::{TaskConfig, TaskRunner};
pub use tasks::InstallTask;
