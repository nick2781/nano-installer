// 通用工具和类型

pub mod cli;
pub mod close_targets;
pub mod config;
pub mod error;
pub mod mutex;
pub mod path_validation;
pub mod platform;
pub mod process;
pub mod result;

pub use error::Error;
pub use result::Result;
