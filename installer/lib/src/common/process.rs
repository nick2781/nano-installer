// 进程检测功能

use crate::common::{Error, Result};
#[cfg(not(windows))]
use std::process::Command;

#[cfg(windows)]
use windows::Win32::{
    Foundation::CloseHandle,
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE},
    },
};

/// 进程信息
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub executable_path: Option<String>,
}

/// 进程检测器
pub struct ProcessDetector {
    target_processes: Vec<String>,
}

impl ProcessDetector {
    /// 创建新的进程检测器
    pub fn new(target_processes: Vec<String>) -> Self {
        Self { target_processes }
    }

    /// 检测目标进程是否正在运行
    pub fn is_target_running(&self) -> Result<bool> {
        let running_processes = self.get_running_processes()?;

        for target in &self.target_processes {
            if running_processes.iter().any(|proc| {
                proc.name.to_lowercase().contains(&target.to_lowercase())
                    || proc
                        .executable_path
                        .as_ref()
                        .map(|path| path.to_lowercase().contains(&target.to_lowercase()))
                        .unwrap_or(false)
            }) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 获取正在运行的目标进程列表
    pub fn get_running_target_processes(&self) -> Result<Vec<ProcessInfo>> {
        let running_processes = self.get_running_processes()?;

        let mut target_processes = Vec::new();
        for proc in running_processes {
            if self.target_processes.iter().any(|target| {
                proc.name.to_lowercase().contains(&target.to_lowercase())
                    || proc
                        .executable_path
                        .as_ref()
                        .map(|path| path.to_lowercase().contains(&target.to_lowercase()))
                        .unwrap_or(false)
            }) {
                target_processes.push(proc);
            }
        }

        Ok(target_processes)
    }

    /// 强制终止目标进程
    pub fn terminate_target_processes(&self) -> Result<()> {
        let target_processes = self.get_running_target_processes()?;

        for proc in target_processes {
            self.terminate_process(proc.pid)?;
        }

        Ok(())
    }

    /// 请求目标进程优雅退出
    pub fn request_target_processes_exit(&self) -> Result<()> {
        let target_processes = self.get_running_target_processes()?;

        for proc in target_processes {
            self.request_process_exit(proc.pid)?;
        }

        Ok(())
    }

    /// 获取所有正在运行的进程
    fn get_running_processes(&self) -> Result<Vec<ProcessInfo>> {
        #[cfg(windows)]
        {
            self.get_running_processes_windows()
        }
        #[cfg(not(windows))]
        {
            self.get_running_processes_unix()
        }
    }

    /// 终止指定进程
    fn terminate_process(&self, pid: u32) -> Result<()> {
        #[cfg(windows)]
        {
            self.terminate_process_windows(pid)
        }
        #[cfg(not(windows))]
        {
            self.terminate_process_unix(pid)
        }
    }

    /// 请求进程优雅退出
    fn request_process_exit(&self, pid: u32) -> Result<()> {
        #[cfg(windows)]
        {
            self.request_process_exit_windows(pid)
        }
        #[cfg(not(windows))]
        {
            self.request_process_exit_unix(pid)
        }
    }

    #[cfg(windows)]
    fn get_running_processes_windows(&self) -> Result<Vec<ProcessInfo>> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| {
                Error::ProcessDetectionFailed(format!("Failed to create snapshot: {}", e))
            })?;

            let mut processes = Vec::new();
            let mut entry = PROCESSENTRY32 {
                dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
                ..Default::default()
            };

            if Process32First(snapshot, &mut entry).is_ok() {
                loop {
                    // Convert the byte array to a null-terminated string
                    let process_name =
                        if let Some(null_pos) = entry.szExeFile.iter().position(|&b| b == 0) {
                            String::from_utf8_lossy(std::slice::from_raw_parts(
                                entry.szExeFile.as_ptr() as *const u8,
                                null_pos,
                            ))
                            .to_string()
                        } else {
                            String::from_utf8_lossy(std::slice::from_raw_parts(
                                entry.szExeFile.as_ptr() as *const u8,
                                entry.szExeFile.len(),
                            ))
                            .to_string()
                        };

                    // 获取进程可执行文件路径
                    let executable_path = self.get_process_executable_path(entry.th32ProcessID);

                    processes.push(ProcessInfo {
                        pid: entry.th32ProcessID,
                        name: process_name,
                        executable_path,
                    });

                    if Process32Next(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }

            CloseHandle(snapshot)?;
            Ok(processes)
        }
    }

    #[cfg(windows)]
    fn get_process_executable_path(&self, pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid).ok()?;
            // 这里需要更复杂的Windows API调用来获取完整路径
            // 为了简化，我们返回None
            let _ = CloseHandle(handle);
            None
        }
    }

    #[cfg(windows)]
    fn terminate_process_windows(&self, pid: u32) -> Result<()> {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|e| {
                Error::ProcessTerminationFailed(format!("Failed to open process {}: {}", pid, e))
            })?;

            windows::Win32::System::Threading::TerminateProcess(handle, 1).map_err(|e| {
                Error::ProcessTerminationFailed(format!(
                    "Failed to terminate process {}: {}",
                    pid, e
                ))
            })?;

            CloseHandle(handle)?;
            Ok(())
        }
    }

    #[cfg(windows)]
    fn request_process_exit_windows(&self, pid: u32) -> Result<()> {
        // 在Windows上，我们可以尝试发送WM_CLOSE消息
        // 这里简化实现，直接调用terminate
        self.terminate_process_windows(pid)
    }

    #[cfg(not(windows))]
    fn get_running_processes_unix(&self) -> Result<Vec<ProcessInfo>> {
        let output = Command::new("ps")
            .args(&["-eo", "pid,comm,args"])
            .output()
            .map_err(|e| Error::ProcessDetectionFailed(format!("Failed to run ps: {}", e)))?;

        if !output.status.success() {
            return Err(Error::ProcessDetectionFailed(
                "ps command failed".to_string(),
            ));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut processes = Vec::new();

        for line in output_str.lines().skip(1) {
            // Skip header
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(pid) = parts[0].parse::<u32>() {
                    let name = parts[1].to_string();
                    let executable_path = if parts.len() > 2 {
                        Some(parts[2..].join(" "))
                    } else {
                        None
                    };

                    processes.push(ProcessInfo {
                        pid,
                        name,
                        executable_path,
                    });
                }
            }
        }

        Ok(processes)
    }

    #[cfg(not(windows))]
    fn terminate_process_unix(&self, pid: u32) -> Result<()> {
        let status = Command::new("kill")
            .args(&["-9", &pid.to_string()])
            .status()
            .map_err(|e| Error::ProcessTerminationFailed(format!("Failed to run kill: {}", e)))?;

        if !status.success() {
            return Err(Error::ProcessTerminationFailed(format!(
                "kill command failed for PID {}",
                pid
            )));
        }

        Ok(())
    }

    #[cfg(not(windows))]
    fn request_process_exit_unix(&self, pid: u32) -> Result<()> {
        let status = Command::new("kill")
            .args(&["-TERM", &pid.to_string()])
            .status()
            .map_err(|e| Error::ProcessTerminationFailed(format!("Failed to run kill: {}", e)))?;

        if !status.success() {
            return Err(Error::ProcessTerminationFailed(format!(
                "kill -TERM command failed for PID {}",
                pid
            )));
        }

        Ok(())
    }
}

/// 便捷函数：检测特定进程是否运行
pub fn is_process_running(process_name: &str) -> Result<bool> {
    let detector = ProcessDetector::new(vec![process_name.to_string()]);
    detector.is_target_running()
}

/// 便捷函数：终止特定进程
pub fn terminate_process(process_name: &str) -> Result<()> {
    let detector = ProcessDetector::new(vec![process_name.to_string()]);
    detector.terminate_target_processes()
}
