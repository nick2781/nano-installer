// Windows 特定的安装功能

#[cfg(windows)]
pub mod registry;

#[cfg(windows)]
pub mod shortcuts;

#[cfg(windows)]
pub mod elevation;
