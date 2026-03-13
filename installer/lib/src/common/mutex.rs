// 互斥锁机制 - 防止多个安装程序实例同时运行

use crate::common::{Error, Result};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(windows)]
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::WAIT_OBJECT_0,
        Foundation::{CloseHandle, HANDLE, WAIT_TIMEOUT},
        System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject},
    },
};

#[cfg(not(windows))]
use std::fs::File;
#[cfg(not(windows))]
use std::io::Write;
#[cfg(not(windows))]
use std::path::Path;

/// 互斥锁管理器
pub struct MutexManager {
    #[cfg(windows)]
    handle: Option<HANDLE>,
    #[cfg(not(windows))]
    lock_file: Option<std::fs::File>,
    name: String,
}

// 为 MutexManager 实现 Send 和 Sync
unsafe impl Send for MutexManager {}
unsafe impl Sync for MutexManager {}

impl MutexManager {
    /// 创建新的互斥锁管理器
    pub fn new(name: &str) -> Self {
        Self {
            #[cfg(windows)]
            handle: None,
            #[cfg(not(windows))]
            lock_file: None,
            name: name.to_string(),
        }
    }

    /// 尝试获取互斥锁
    pub fn try_lock(&mut self) -> Result<bool> {
        #[cfg(windows)]
        {
            self.try_lock_windows()
        }
        #[cfg(not(windows))]
        {
            self.try_lock_unix()
        }
    }

    /// 等待获取互斥锁（带超时）
    pub fn wait_for_lock(&mut self, timeout: Duration) -> Result<bool> {
        #[cfg(windows)]
        {
            self.wait_for_lock_windows(timeout)
        }
        #[cfg(not(windows))]
        {
            self.wait_for_lock_unix(timeout)
        }
    }

    /// 释放互斥锁
    pub fn unlock(&mut self) -> Result<()> {
        #[cfg(windows)]
        {
            self.unlock_windows()
        }
        #[cfg(not(windows))]
        {
            self.unlock_unix()
        }
    }

    /// 检查互斥锁是否已被其他进程持有
    pub fn is_locked_by_other(&self) -> bool {
        #[cfg(windows)]
        {
            self.is_locked_by_other_windows()
        }
        #[cfg(not(windows))]
        {
            self.is_locked_by_other_unix()
        }
    }

    #[cfg(windows)]
    fn try_lock_windows(&mut self) -> Result<bool> {
        unsafe {
            let name_wide: Vec<u16> = format!("Global\\{}", self.name)
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let handle = CreateMutexW(
                None,
                true, // 初始拥有者
                PWSTR(name_wide.as_ptr() as *mut u16),
            );

            match handle {
                Ok(h) => {
                    // 检查是否已经存在
                    let wait_result = WaitForSingleObject(h, 0);
                    match wait_result {
                        WAIT_OBJECT_0 => {
                            // 成功获取锁
                            self.handle = Some(h);
                            Ok(true)
                        }
                        WAIT_TIMEOUT => {
                            // 锁已被其他进程持有
                            let _ = CloseHandle(h);
                            Ok(false)
                        }
                        _ => {
                            // 错误
                            let _ = CloseHandle(h);
                            Err(Error::MutexError(format!(
                                "Failed to create or wait for mutex: {}",
                                self.name
                            )))
                        }
                    }
                }
                Err(e) => Err(Error::MutexError(format!(
                    "Failed to create mutex '{}': {}",
                    self.name, e
                ))),
            }
        }
    }

    #[cfg(windows)]
    fn wait_for_lock_windows(&mut self, timeout: Duration) -> Result<bool> {
        unsafe {
            let name_wide: Vec<u16> = format!("Global\\{}", self.name)
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let handle = CreateMutexW(
                None,
                true, // 初始拥有者
                PWSTR(name_wide.as_ptr() as *mut u16),
            );

            match handle {
                Ok(h) => {
                    let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;
                    let wait_result = WaitForSingleObject(h, timeout_ms);
                    match wait_result {
                        WAIT_OBJECT_0 => {
                            // 成功获取锁
                            self.handle = Some(h);
                            Ok(true)
                        }
                        WAIT_TIMEOUT => {
                            // 超时
                            let _ = CloseHandle(h);
                            Ok(false)
                        }
                        _ => {
                            // 错误
                            let _ = CloseHandle(h);
                            Err(Error::MutexError(format!(
                                "Failed to wait for mutex: {}",
                                self.name
                            )))
                        }
                    }
                }
                Err(e) => Err(Error::MutexError(format!(
                    "Failed to create mutex '{}': {}",
                    self.name, e
                ))),
            }
        }
    }

    #[cfg(windows)]
    fn unlock_windows(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            unsafe {
                ReleaseMutex(handle)?;
                CloseHandle(handle)?;
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    fn is_locked_by_other_windows(&self) -> bool {
        unsafe {
            let name_wide: Vec<u16> = format!("Global\\{}", self.name)
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            match CreateMutexW(None, false, PWSTR(name_wide.as_ptr() as *mut u16)) {
                Ok(handle) => {
                    let wait_result = WaitForSingleObject(handle, 0);
                    let _ = CloseHandle(handle);
                    wait_result != WAIT_OBJECT_0
                }
                Err(_) => true, // 如果创建失败，假设被锁定
            }
        }
    }

    #[cfg(not(windows))]
    fn try_lock_unix(&mut self) -> Result<bool> {
        let lock_path = self.get_lock_file_path();

        // 尝试创建锁文件
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(file) => {
                // 写入进程ID
                let pid = std::process::id();
                writeln!(file, "{}", pid)?;
                self.lock_file = Some(file);
                Ok(true)
            }
            Err(_) => {
                // 文件已存在，检查是否有效
                if self.is_lock_file_valid(&lock_path) {
                    Ok(false) // 被其他进程锁定
                } else {
                    // 锁文件无效，删除并重新创建
                    let _ = std::fs::remove_file(&lock_path);
                    self.try_lock_unix()
                }
            }
        }
    }

    #[cfg(not(windows))]
    fn wait_for_lock_unix(&mut self, timeout: Duration) -> Result<bool> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if self.try_lock_unix()? {
                return Ok(true);
            }

            // 等待一小段时间后重试
            std::thread::sleep(Duration::from_millis(100));
        }

        Ok(false) // 超时
    }

    #[cfg(not(windows))]
    fn unlock_unix(&mut self) -> Result<()> {
        if let Some(_) = self.lock_file.take() {
            let lock_path = self.get_lock_file_path();
            let _ = std::fs::remove_file(lock_path);
        }
        Ok(())
    }

    #[cfg(not(windows))]
    fn is_locked_by_other_unix(&self) -> bool {
        let lock_path = self.get_lock_file_path();
        self.is_lock_file_valid(&lock_path)
    }

    #[cfg(not(windows))]
    fn get_lock_file_path(&self) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("{}.lock", self.name));
        path
    }

    #[cfg(not(windows))]
    fn is_lock_file_valid(&self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        // 读取PID并检查进程是否还在运行
        match std::fs::read_to_string(path) {
            Ok(content) => {
                if let Ok(pid) = content.trim().parse::<u32>() {
                    // 检查进程是否存在
                    #[cfg(target_os = "linux")]
                    {
                        Path::new(&format!("/proc/{}", pid)).exists()
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        // 对于其他Unix系统，使用kill -0检查进程
                        std::process::Command::new("kill")
                            .args(&["-0", &pid.to_string()])
                            .output()
                            .map(|output| output.status.success())
                            .unwrap_or(false)
                    }
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }
}

impl Drop for MutexManager {
    fn drop(&mut self) {
        let _ = self.unlock();
    }
}

/// 全局互斥锁实例
static GLOBAL_MUTEX: Mutex<Option<Arc<Mutex<MutexManager>>>> = Mutex::new(None);

/// 初始化全局互斥锁
pub fn init_global_mutex(name: &str) -> Result<()> {
    let mut global_mutex = GLOBAL_MUTEX.lock().unwrap();
    if global_mutex.is_none() {
        let mut manager = MutexManager::new(name);
        if !manager.try_lock()? {
            return Err(Error::MutexError(format!(
                "Another instance of '{}' is already running",
                name
            )));
        }
        *global_mutex = Some(Arc::new(Mutex::new(manager)));
    }
    Ok(())
}

/// 获取全局互斥锁
pub fn get_global_mutex() -> Option<Arc<Mutex<MutexManager>>> {
    GLOBAL_MUTEX.lock().unwrap().clone()
}

/// 检查是否有其他实例在运行
pub fn is_other_instance_running(name: &str) -> bool {
    let manager = MutexManager::new(name);
    manager.is_locked_by_other()
}
