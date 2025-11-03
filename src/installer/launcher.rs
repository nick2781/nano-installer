use crate::common::error::Error;
use std::path::Path;
use std::process::Command;
use tracing::{info, warn, error};

/// 应用启动器
pub struct AppLauncher {
    exe_path: String,
    exe_args: Option<String>,
    working_dir: Option<String>,
}

impl AppLauncher {
    pub fn new(exe_path: String, exe_args: Option<String>, working_dir: Option<String>) -> Self {
        Self {
            exe_path,
            exe_args,
            working_dir,
        }
    }

    /// 启动应用程序
    pub fn launch(&self) -> Result<(), Error> {
        info!("Launching application: {}", self.exe_path);

        // 检查可执行文件是否存在
        if !Path::new(&self.exe_path).exists() {
            return Err(Error::InstallationFailed(format!(
                "Executable not found: {}",
                self.exe_path
            )));
        }

        // 构建命令
        let mut command = Command::new(&self.exe_path);

        // 设置工作目录
        if let Some(working_dir) = &self.working_dir {
            command.current_dir(working_dir);
        } else if let Some(parent) = Path::new(&self.exe_path).parent() {
            command.current_dir(parent);
        }

        // 设置参数
        if let Some(args) = &self.exe_args {
            // 简单分割参数（实际应用中可能需要更复杂的解析）
            for arg in args.split_whitespace() {
                command.arg(arg);
            }
        }

        // 启动进程
        match command.spawn() {
            Ok(child) => {
                info!("Successfully launched application with PID: {}", child.id());
                
                // 在Windows上，我们可以选择是否等待进程
                #[cfg(windows)]
                {
                    // 不等待进程，让它在后台运行
                    std::mem::forget(child);
                }
                
                #[cfg(not(windows))]
                {
                    // 在非Windows系统上，可以选择等待或分离
                    std::mem::forget(child);
                }
                
                Ok(())
            }
            Err(e) => {
                error!("Failed to launch application: {}", e);
                Err(Error::InstallationFailed(format!(
                    "Failed to launch application: {}",
                    e
                )))
            }
        }
    }

    /// 检查应用程序是否正在运行
    pub fn is_running(&self) -> bool {
        let exe_name = Path::new(&self.exe_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        #[cfg(windows)]
        {
            use crate::common::process::ProcessDetector;
            let detector = ProcessDetector::new(vec![exe_name.to_string()]);
            detector.is_target_running().unwrap_or(false)
        }
        #[cfg(not(windows))]
        {
            // 在非Windows系统上，使用pgrep命令
            match Command::new("pgrep").arg(exe_name).output() {
                Ok(output) => output.status.success(),
                Err(_) => false,
            }
        }
    }

    /// 等待应用程序启动完成
    pub fn wait_for_launch(&self, timeout_seconds: u64) -> Result<bool, Error> {
        use std::time::{Duration, Instant};

        let start_time = Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);

        while start_time.elapsed() < timeout {
            if self.is_running() {
                info!("Application is now running");
                return Ok(true);
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        warn!("Application did not start within {} seconds", timeout_seconds);
        Ok(false)
    }

    /// 启动并等待
    pub fn launch_and_wait(&self, timeout_seconds: u64) -> Result<bool, Error> {
        self.launch()?;
        self.wait_for_launch(timeout_seconds)
    }
}
