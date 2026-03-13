use crate::common::error::Error;
use std::collections::HashMap;
use tracing::info;

/// 运行模式
#[derive(Debug, Clone, PartialEq)]
pub enum RunMode {
    Install,
    Update,
    Uninstall,
    Silent,
}

impl std::str::FromStr for RunMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "install" => Ok(RunMode::Install),
            "update" => Ok(RunMode::Update),
            "uninstall" => Ok(RunMode::Uninstall),
            "silent" => Ok(RunMode::Silent),
            _ => Err(Error::InstallationFailed(format!(
                "Invalid run mode: {}",
                s
            ))),
        }
    }
}

/// 命令行参数
#[derive(Debug, Clone)]
pub struct CliArgs {
    pub mode: RunMode,
    pub silent: bool,
    pub channel: Option<String>,
    pub install_path: Option<String>,
    pub config_path: Option<String>,
    pub log_level: Option<String>,
    pub help: bool,
    pub version: bool,
}

impl CliArgs {
    pub fn new() -> Self {
        Self {
            mode: RunMode::Install,
            silent: false,
            channel: None,
            install_path: None,
            config_path: None,
            log_level: None,
            help: false,
            version: false,
        }
    }

    /// 解析命令行参数
    pub fn parse(args: Vec<String>) -> Result<Self, Error> {
        let mut cli_args = Self::new();
        let mut i = 0;

        while i < args.len() {
            let arg = &args[i];

            match arg.as_str() {
                "--mode" | "-m" => {
                    if i + 1 < args.len() {
                        cli_args.mode = args[i + 1].parse()?;
                        i += 2;
                    } else {
                        return Err(Error::InstallationFailed(
                            "--mode requires a value".to_string(),
                        ));
                    }
                }
                "--silent" | "-s" => {
                    cli_args.silent = true;
                    i += 1;
                }
                "--channel" | "-c" => {
                    if i + 1 < args.len() {
                        cli_args.channel = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(Error::InstallationFailed(
                            "--channel requires a value".to_string(),
                        ));
                    }
                }
                "--path" | "-p" => {
                    if i + 1 < args.len() {
                        cli_args.install_path = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(Error::InstallationFailed(
                            "--path requires a value".to_string(),
                        ));
                    }
                }
                "--config" => {
                    if i + 1 < args.len() {
                        cli_args.config_path = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(Error::InstallationFailed(
                            "--config requires a value".to_string(),
                        ));
                    }
                }
                "--log-level" => {
                    if i + 1 < args.len() {
                        cli_args.log_level = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(Error::InstallationFailed(
                            "--log-level requires a value".to_string(),
                        ));
                    }
                }
                "--help" | "-h" => {
                    cli_args.help = true;
                    i += 1;
                }
                "--version" | "-v" => {
                    cli_args.version = true;
                    i += 1;
                }
                _ => {
                    // 尝试从文件名提取渠道信息
                    if cli_args.channel.is_none() {
                        cli_args.channel = Self::extract_channel_from_filename(arg);
                    }
                    i += 1;
                }
            }
        }

        // 如果指定了silent模式，自动设置为Silent运行模式
        if cli_args.silent {
            cli_args.mode = RunMode::Silent;
        }

        info!("Parsed CLI args: {:?}", cli_args);
        Ok(cli_args)
    }

    /// 从文件名提取渠道信息
    fn extract_channel_from_filename(filename: &str) -> Option<String> {
        // 简单的渠道提取逻辑
        if let Some(underscore_pos) = filename.rfind('_') {
            if let Some(dot_pos) = filename.rfind('.') {
                if underscore_pos < dot_pos {
                    let potential_channel = &filename[underscore_pos + 1..dot_pos];
                    if !potential_channel.is_empty()
                        && potential_channel.chars().all(|c| c.is_alphanumeric())
                    {
                        return Some(potential_channel.to_string());
                    }
                }
            }
        }
        None
    }

    /// 显示帮助信息
    pub fn show_help() {
        println!("nano-installer - Universal installer generator");
        println!();
        println!("Usage: nano-installer [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -m, --mode <MODE>        Run mode: install, update, uninstall, silent");
        println!("  -s, --silent             Silent mode (no UI)");
        println!("  -c, --channel <CHANNEL> Channel identifier");
        println!("  -p, --path <PATH>        Installation path");
        println!("      --config <PATH>      Configuration file path");
        println!("      --log-level <LEVEL>  Log level: trace, debug, info, warn, error");
        println!("  -h, --help               Show this help message");
        println!("  -v, --version            Show version information");
        println!();
        println!("Examples:");
        println!("  nano-installer --mode install --channel CN");
        println!("  nano-installer --silent --path \"C:\\Program Files\\MyApp\"");
        println!("  nano-installer --mode uninstall");
    }

    /// 显示版本信息
    pub fn show_version() {
        println!("nano-installer version 1.0.0");
        println!("Universal installer generator for Windows");
    }

    /// 验证参数
    pub fn validate(&self) -> Result<(), Error> {
        // 验证运行模式
        match self.mode {
            RunMode::Silent => {
                if self.install_path.is_none() {
                    return Err(Error::InstallationFailed(
                        "Silent mode requires --path parameter".to_string(),
                    ));
                }
            }
            _ => {}
        }

        // 验证渠道名称
        if let Some(ref channel) = self.channel {
            if !channel
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return Err(Error::InstallationFailed(
                    "Invalid channel name. Only alphanumeric characters, underscores and hyphens are allowed".to_string()
                ));
            }
        }

        // 验证安装路径
        if let Some(ref path) = self.install_path {
            if !std::path::Path::new(path).is_absolute() {
                return Err(Error::InstallationFailed(
                    "Installation path must be absolute".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// 获取环境变量
    pub fn get_env_vars(&self) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();

        // 添加一些常用的环境变量
        if let Ok(program_files) = std::env::var("PROGRAMFILES") {
            env_vars.insert("PROGRAMFILES".to_string(), program_files);
        }
        if let Ok(program_files_x86) = std::env::var("PROGRAMFILES(X86)") {
            env_vars.insert("PROGRAMFILES(X86)".to_string(), program_files_x86);
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            env_vars.insert("APPDATA".to_string(), appdata);
        }
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            env_vars.insert("LOCALAPPDATA".to_string(), local_appdata);
        }

        env_vars
    }
}

impl Default for CliArgs {
    fn default() -> Self {
        Self::new()
    }
}
