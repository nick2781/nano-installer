/// 安装器运行模式

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerMode {
    /// 安装模式
    Install,
    /// 卸载模式
    Uninstall,
}

impl InstallerMode {
    /// 从命令行参数或 exe 名称检测模式
    pub fn detect() -> Self {
        // 1. 检查命令行参数
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|arg| arg == "--uninstall" || arg == "/uninstall") {
            return InstallerMode::Uninstall;
        }
        
        // 2. 检查 exe 名称
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(file_name) = exe_path.file_name() {
                let name = file_name.to_string_lossy().to_lowercase();
                if name.contains("uninst") || name.contains("uninstall") {
                    return InstallerMode::Uninstall;
                }
            }
        }
        
        // 默认为安装模式
        InstallerMode::Install
    }
}

