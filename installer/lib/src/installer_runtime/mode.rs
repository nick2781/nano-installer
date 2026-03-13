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
    /// 静默卸载模式 (无 GUI)
    SilentUninstall,
}

impl InstallerMode {
    /// 从命令行参数或 exe 名称检测模式
    pub fn detect() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let exe_name = std::env::current_exe().ok().and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        });

        Self::detect_from_parts(&args, exe_name.as_deref())
    }

    fn detect_from_parts(args: &[String], exe_name: Option<&str>) -> Self {
        let mut explicit_mode = None;
        let mut silent_requested = false;

        // 1. 检查命令行参数
        for (i, arg) in args.iter().enumerate() {
            let lower = arg.to_lowercase();
            match lower.as_str() {
                "--uninstall" | "/uninstall" | "-u" => {
                    explicit_mode = Some(InstallerMode::Uninstall)
                }
                "--update" | "/update" => explicit_mode = Some(InstallerMode::Update),
                "--silent" | "/s" | "-s" => silent_requested = true,
                "--mode" | "-m" => {
                    if let Some(mode_str) = args.get(i + 1) {
                        match mode_str.to_lowercase().as_str() {
                            "update" => explicit_mode = Some(InstallerMode::Update),
                            "silent" => explicit_mode = Some(InstallerMode::Silent),
                            "uninstall" => explicit_mode = Some(InstallerMode::Uninstall),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        // 2. 检查 exe 名称
        let exe_is_uninstaller = exe_name
            .map(|name| {
                let lower = name.to_lowercase();
                lower.contains("uninst") || lower.contains("uninstall")
            })
            .unwrap_or(false);

        match explicit_mode {
            Some(InstallerMode::Uninstall) => {
                if silent_requested {
                    return InstallerMode::SilentUninstall;
                }
                return InstallerMode::Uninstall;
            }
            Some(InstallerMode::Update) => return InstallerMode::Update,
            Some(InstallerMode::Silent) => {
                if exe_is_uninstaller {
                    return InstallerMode::SilentUninstall;
                }
                return InstallerMode::Silent;
            }
            Some(InstallerMode::Install) | Some(InstallerMode::SilentUninstall) | None => {}
        }

        if exe_is_uninstaller {
            if silent_requested {
                return InstallerMode::SilentUninstall;
            }
            return InstallerMode::Uninstall;
        }

        if silent_requested {
            return InstallerMode::Silent;
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
        let path = config.registry.uninstall_key.replace('/', "\\");
        let Some((hive, subkey)) = path.split_once('\\') else {
            return false;
        };
        use winreg::enums::*;
        use winreg::RegKey;
        let root = match hive.to_uppercase().as_str() {
            "HKLM" | "HKEY_LOCAL_MACHINE" => RegKey::predef(HKEY_LOCAL_MACHINE),
            "HKCU" | "HKEY_CURRENT_USER" => RegKey::predef(HKEY_CURRENT_USER),
            _ => return false,
        };
        root.open_subkey(subkey).is_ok()
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

#[cfg(test)]
mod tests {
    use super::InstallerMode;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn detects_silent_uninstall_from_uninstall_flag() {
        let mode = InstallerMode::detect_from_parts(
            &args(&["setup.exe", "--mode", "uninstall", "--silent"]),
            Some("setup.exe"),
        );
        assert_eq!(mode, InstallerMode::SilentUninstall);
    }

    #[test]
    fn detects_silent_uninstall_from_uninstaller_name() {
        let mode = InstallerMode::detect_from_parts(
            &args(&["uninst.exe", "--silent"]),
            Some("uninst.exe"),
        );
        assert_eq!(mode, InstallerMode::SilentUninstall);
    }

    #[test]
    fn keeps_update_mode_when_silent_is_also_present() {
        let mode = InstallerMode::detect_from_parts(
            &args(&["setup.exe", "--update", "--silent"]),
            Some("setup.exe"),
        );
        assert_eq!(mode, InstallerMode::Update);
    }
}
