// Windows 权限提升

use crate::common::Result;

/// 检查是否需要提升权限
pub fn needs_elevation(install_path: &str) -> bool {
    // 检查安装路径是否需要管理员权限
    // Program Files 目录通常需要管理员权限
    let program_files = std::env::var("ProgramFiles").unwrap_or_default();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();

    install_path.starts_with(&program_files)
        || install_path.starts_with(&program_files_x86)
        || install_path.starts_with(r"C:\Program Files")
}

/// 请求提升权限并重启
pub fn request_elevation_and_restart() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    crate::common::platform::request_elevation(&args)?;
    Ok(())
}
