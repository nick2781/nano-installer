// 命令行参数解析

use crate::common::Result;

/// 安装器命令行参数
#[derive(Debug, Clone)]
pub struct InstallerArgs {
    /// 静默安装
    pub silent: bool,

    /// 指定安装目录
    pub install_dir: Option<String>,

    /// 指定语言
    pub locale: Option<String>,

    /// 显示帮助
    pub help: bool,

    /// 显示版本
    pub version: bool,
}

impl InstallerArgs {
    /// 解析命令行参数
    pub fn parse() -> Result<Self> {
        let args: Vec<String> = std::env::args().collect();

        let mut installer_args = Self {
            silent: false,
            install_dir: None,
            locale: None,
            help: false,
            version: false,
        };

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "/S" | "/s" | "--silent" => {
                    installer_args.silent = true;
                }
                "/D" | "/d" => {
                    if i + 1 < args.len() {
                        installer_args.install_dir = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                s if s.starts_with("/D=") => {
                    installer_args.install_dir = Some(s[3..].to_string());
                }
                "/L" | "/l" | "--locale" => {
                    if i + 1 < args.len() {
                        installer_args.locale = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                s if s.starts_with("/L=") => {
                    installer_args.locale = Some(s[3..].to_string());
                }
                "/?" | "--help" => {
                    installer_args.help = true;
                }
                "--version" => {
                    installer_args.version = true;
                }
                _ => {
                    // 忽略未知参数
                }
            }
            i += 1;
        }

        Ok(installer_args)
    }

    /// 打印帮助信息
    pub fn print_help(app_name: &str, version: &str) {
        println!("{} Installer v{}", app_name, version);
        println!("\nUsage: installer.exe [options]");
        println!("\nOptions:");
        println!("  /S, --silent         Silent installation");
        println!("  /D=<path>            Specify installation directory");
        println!("  /L=<locale>          Specify language (en-US, zh-CN, zh-TW, ja, vi)");
        println!("  /?, --help           Show this help message");
        println!("  --version            Show version information");
        println!("\nExamples:");
        println!("  installer.exe");
        println!("  installer.exe /S /D=C:\\MyApp");
        println!("  installer.exe /L=zh-CN");
    }
}

/// 卸载器命令行参数
#[derive(Debug, Clone)]
pub struct UninstallerArgs {
    /// 静默卸载
    pub silent: bool,

    /// 显示帮助
    pub help: bool,

    /// 保留用户数据
    pub keep_data: bool,
}

impl UninstallerArgs {
    /// 解析命令行参数
    pub fn parse() -> Result<Self> {
        let args: Vec<String> = std::env::args().collect();

        let mut uninstaller_args = Self {
            silent: false,
            help: false,
            keep_data: false,
        };

        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "/S" | "/s" | "--silent" => {
                    uninstaller_args.silent = true;
                }
                "/K" | "/k" | "--keep-data" => {
                    uninstaller_args.keep_data = true;
                }
                "/?" | "--help" => {
                    uninstaller_args.help = true;
                }
                _ => {
                    // 忽略未知参数
                }
            }
        }

        Ok(uninstaller_args)
    }

    /// 打印帮助信息
    pub fn print_help(app_name: &str, version: &str) {
        println!("{} Uninstaller v{}", app_name, version);
        println!("\nUsage: uninstaller.exe [options]");
        println!("\nOptions:");
        println!("  /S, --silent         Silent uninstallation");
        println!("  /K, --keep-data      Keep user data and configuration");
        println!("  /?, --help           Show this help message");
        println!("\nExamples:");
        println!("  uninstaller.exe");
        println!("  uninstaller.exe /S");
        println!("  uninstaller.exe /S /K");
    }
}
