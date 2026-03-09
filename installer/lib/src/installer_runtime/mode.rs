/// 安装器运行模式

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerMode {
    /// 安装模式
    Install,
    /// 更新模式 (跳过配置页)
    Update,
    /// 静默安装模式 (无 GUI)
    Silent,
    /// 卸载模式
    Uninstall,
}

impl InstallerMode {
    /// 从命令行参数或 exe 名称检测模式
    pub fn detect() -> Self {
        let args: Vec<String> = std::env::args().collect();

        // 1. 检查命令行参数
        for (i, arg) in args.iter().enumerate() {
            let lower = arg.to_lowercase();
            match lower.as_str() {
                "--uninstall" | "/uninstall" | "-u" => return InstallerMode::Uninstall,
                "--update" | "/update" => return InstallerMode::Update,
                "--silent" | "/s" | "-s" => return InstallerMode::Silent,
                "--mode" | "-m" => {
                    if let Some(mode_str) = args.get(i + 1) {
                        match mode_str.to_lowercase().as_str() {
                            "update" => return InstallerMode::Update,
                            "silent" => return InstallerMode::Silent,
                            "uninstall" => return InstallerMode::Uninstall,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
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

        // 3. 自动检测更新模式：如果注册表中已有安装路径，切换到更新模式
        #[cfg(windows)]
        {
            // 尝试读取注册表检测已安装
            // 此检测依赖 config，在 run_installer 中做更合理
        }

        // 默认为安装模式
        InstallerMode::Install
    }

    /// 检测更新模式（需要 config 信息）
    #[cfg(windows)]
    pub fn detect_update(config: &crate::config::InstallerConfig) -> bool {
        if !config.advanced.update_mode_support {
            return false;
        }
        // 检查注册表中是否已有安装路径
        use winreg::enums::*;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let uninstall_key = format!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}",
            config.project.name
        );
        hklm.open_subkey(&uninstall_key).is_ok()
    }

    #[cfg(not(windows))]
    pub fn detect_update(_config: &crate::config::InstallerConfig) -> bool {
        false
    }

    /// 获取 CLI 参数中的安装路径
    pub fn get_cli_install_path() -> Option<String> {
        let args: Vec<String> = std::env::args().collect();
        for (i, arg) in args.iter().enumerate() {
            let lower = arg.to_lowercase();
            if (lower == "--path" || lower == "-p") && i + 1 < args.len() {
                return Some(args[i + 1].clone());
            }
        }
        None
    }
}
