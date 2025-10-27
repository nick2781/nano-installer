// 错误类型定义

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Language pack error: {0}")]
    LanguagePack(String),

    #[error("Language pack not found: {0}")]
    LanguagePackNotFound(String),

    #[error("Invalid language pack format: {0}")]
    InvalidLanguagePack(String),

    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    #[error("Uninstallation failed: {0}")]
    UninstallationFailed(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Path error: {0}")]
    Path(String),

    #[error("Archive error: {0}")]
    Archive(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Checksum mismatch")]
    ChecksumMismatch,

    #[error("User cancelled")]
    UserCancelled,

    #[error("Process error: {0}")]
    Process(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Windows specific errors
#[cfg(windows)]
impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Error::Unknown(format!("Windows API error: {}", err))
    }
}
