// 安装路径校验功能

use crate::common::{Error, Result};
use std::path::{Path, PathBuf};

#[cfg(windows)]
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::BOOL,
        Storage::FileSystem::{
            GetDiskFreeSpaceExW, GetDriveTypeW,
        },
        System::SystemInformation::GetSystemDirectoryW,
    },
};

/// 路径校验结果
#[derive(Debug, Clone)]
pub struct PathValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub disk_type: DiskType,
    pub free_space_bytes: u64,
    pub total_space_bytes: u64,
    pub required_space_bytes: u64,
}

/// 磁盘类型
#[derive(Debug, Clone, PartialEq)]
pub enum DiskType {
    Fixed,      // 固定磁盘 (HDD/SSD)
    Removable,  // 可移动磁盘
    Unknown,    // 未知类型
    Network,    // 网络磁盘
    CdRom,      // 光盘
    RamDisk,    // 内存磁盘
}

impl DiskType {
    pub fn is_suitable_for_installation(&self) -> bool {
        matches!(self, DiskType::Fixed)
    }
}

/// 路径校验器
pub struct PathValidator {
    required_space_bytes: u64,
    allow_removable_drives: bool,
    allow_network_drives: bool,
}

impl PathValidator {
    /// 创建新的路径校验器
    pub fn new(required_space_bytes: u64) -> Self {
        Self {
            required_space_bytes,
            allow_removable_drives: false,
            allow_network_drives: false,
        }
    }

    /// 设置是否允许可移动磁盘
    pub fn allow_removable_drives(mut self, allow: bool) -> Self {
        self.allow_removable_drives = allow;
        self
    }

    /// 设置是否允许网络磁盘
    pub fn allow_network_drives(mut self, allow: bool) -> Self {
        self.allow_network_drives = allow;
        self
    }

    /// 校验安装路径
    pub fn validate_path(&self, path: &Path) -> Result<PathValidationResult> {
        let mut result = PathValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            disk_type: DiskType::Unknown,
            free_space_bytes: 0,
            total_space_bytes: 0,
            required_space_bytes: self.required_space_bytes,
        };

        // 1. 检查路径格式
        self.validate_path_format(path, &mut result)?;

        // 2. 检查磁盘类型
        self.validate_disk_type(path, &mut result)?;

        // 3. 检查磁盘空间
        self.validate_disk_space(path, &mut result)?;

        // 4. 检查路径权限
        self.validate_path_permissions(path, &mut result)?;

        // 5. 检查路径长度
        self.validate_path_length(path, &mut result)?;

        // 6. 检查特殊字符
        self.validate_special_characters(path, &mut result)?;

        result.is_valid = result.errors.is_empty();
        Ok(result)
    }

    /// 校验路径格式
    fn validate_path_format(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        if path.to_string_lossy().is_empty() {
            result.errors.push("安装路径不能为空".to_string());
            return Ok(());
        }

        // 检查路径是否包含非法字符
        let path_str = path.to_string_lossy();
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        
        for ch in invalid_chars {
            if path_str.contains(ch) {
                result.errors.push(format!("路径包含非法字符: '{}'", ch));
            }
        }

        // 检查路径是否以反斜杠结尾（Windows）
        #[cfg(windows)]
        {
            if path_str.ends_with('\\') && path_str.len() > 3 {
                result.warnings.push("路径末尾的反斜杠将被自动移除".to_string());
            }
        }

        Ok(())
    }

    /// 校验磁盘类型
    fn validate_disk_type(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        let drive = self.get_drive_from_path(path);
        result.disk_type = self.get_disk_type(&drive)?;

        match result.disk_type {
            DiskType::Fixed => {
                // 固定磁盘，适合安装
            }
            DiskType::Removable => {
                if !self.allow_removable_drives {
                    result.errors.push("不允许在可移动磁盘上安装".to_string());
                } else {
                    result.warnings.push("在可移动磁盘上安装可能导致程序无法正常使用".to_string());
                }
            }
            DiskType::Network => {
                if !self.allow_network_drives {
                    result.errors.push("不允许在网络磁盘上安装".to_string());
                } else {
                    result.warnings.push("在网络磁盘上安装可能导致性能问题".to_string());
                }
            }
            DiskType::CdRom => {
                result.errors.push("不能在光盘上安装程序".to_string());
            }
            DiskType::RamDisk => {
                result.warnings.push("在内存磁盘上安装，重启后数据将丢失".to_string());
            }
            DiskType::Unknown => {
                result.warnings.push("无法确定磁盘类型，安装可能不稳定".to_string());
            }
        }

        Ok(())
    }

    /// 校验磁盘空间
    fn validate_disk_space(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        let drive = self.get_drive_from_path(path);
        let (free_space, total_space) = self.get_disk_space(&drive)?;
        
        result.free_space_bytes = free_space;
        result.total_space_bytes = total_space;

        if free_space < self.required_space_bytes {
            result.errors.push(format!(
                "磁盘空间不足。需要: {} MB，可用: {} MB",
                self.required_space_bytes / (1024 * 1024),
                free_space / (1024 * 1024)
            ));
        } else if free_space < self.required_space_bytes * 2 {
            result.warnings.push(format!(
                "磁盘空间较少。建议至少保留 {} MB 可用空间",
                self.required_space_bytes * 2 / (1024 * 1024)
            ));
        }

        Ok(())
    }

    /// 校验路径权限
    fn validate_path_permissions(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        // 检查父目录是否存在且可写
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                result.errors.push("父目录不存在".to_string());
                return Ok(());
            }

            // 尝试创建测试文件来检查写权限
            let test_file = parent.join(".installer_test_write");
            if std::fs::write(&test_file, b"test").is_err() {
                result.errors.push("没有写入权限".to_string());
            } else {
                let _ = std::fs::remove_file(&test_file);
            }
        }

        // 检查是否在系统目录
        #[cfg(windows)]
        {
            if let Ok(system_dir) = self.get_system_directory() {
                if path.starts_with(&system_dir) {
                    result.warnings.push("不建议在系统目录中安装程序".to_string());
                }
            }
        }

        Ok(())
    }

    /// 校验路径长度
    fn validate_path_length(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        let path_str = path.to_string_lossy();
        
        // Windows 路径长度限制
        #[cfg(windows)]
        {
            if path_str.len() > 260 {
                result.errors.push("路径长度超过 260 字符限制".to_string());
            }
        }

        // 检查是否过短
        if path_str.len() < 3 {
            result.errors.push("路径过短".to_string());
        }

        Ok(())
    }

    /// 校验特殊字符
    fn validate_special_characters(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        let path_str = path.to_string_lossy();
        
        // 检查是否包含连续的点
        if path_str.contains("..") {
            result.warnings.push("路径包含相对路径符号，可能导致安全问题".to_string());
        }

        // 检查是否包含空格
        if path_str.contains(' ') {
            result.warnings.push("路径包含空格，某些程序可能无法正常工作".to_string());
        }

        // 检查是否包含中文字符
        if path_str.chars().any(|c| c as u32 > 127) {
            result.warnings.push("路径包含非ASCII字符，建议使用英文路径".to_string());
        }

        Ok(())
    }

    /// 从路径获取驱动器
    fn get_drive_from_path(&self, path: &Path) -> String {
        #[cfg(windows)]
        {
            if let Some(prefix) = path.components().next() {
                format!("{}", prefix.as_os_str().to_string_lossy())
            } else {
                "C:\\".to_string()
            }
        }
        #[cfg(not(windows))]
        {
            "/".to_string()
        }
    }

    /// 获取磁盘类型
    fn get_disk_type(&self, drive: &str) -> Result<DiskType> {
        #[cfg(windows)]
        {
            self.get_disk_type_windows(drive)
        }
        #[cfg(not(windows))]
        {
            self.get_disk_type_unix(drive)
        }
    }

    /// 获取磁盘空间
    fn get_disk_space(&self, drive: &str) -> Result<(u64, u64)> {
        #[cfg(windows)]
        {
            self.get_disk_space_windows(drive)
        }
        #[cfg(not(windows))]
        {
            self.get_disk_space_unix(drive)
        }
    }

    #[cfg(windows)]
    fn get_disk_type_windows(&self, drive: &str) -> Result<DiskType> {
        unsafe {
            let drive_wide: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0)).collect();
            let drive_type = GetDriveTypeW(PWSTR(drive_wide.as_ptr() as *mut u16));

            Ok(match drive_type {
                3 => DiskType::Fixed,      // DRIVE_FIXED
                2 => DiskType::Removable,  // DRIVE_REMOVABLE
                1 => DiskType::Unknown,    // DRIVE_UNKNOWN
                4 => DiskType::Network,    // DRIVE_REMOTE
                5 => DiskType::CdRom,      // DRIVE_CDROM
                6 => DiskType::RamDisk,    // DRIVE_RAMDISK
                _ => DiskType::Unknown,
            })
        }
    }

    #[cfg(windows)]
    fn get_disk_space_windows(&self, drive: &str) -> Result<(u64, u64)> {
        unsafe {
            let drive_wide: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0)).collect();
            let mut free_bytes = 0u64;
            let mut total_bytes = 0u64;

            match GetDiskFreeSpaceExW(
                PWSTR(drive_wide.as_ptr() as *mut u16),
                Some(&mut free_bytes),
                Some(&mut total_bytes),
                None,
            ) {
                Ok(_) => Ok((free_bytes, total_bytes)),
                Err(e) => Err(Error::PathValidationFailed(format!(
                    "无法获取磁盘空间信息: {} - {}",
                    drive, e
                ))),
            }
        }
    }

    #[cfg(windows)]
    fn get_system_directory(&self) -> Result<PathBuf> {
        unsafe {
            let mut buffer = [0u16; 260];
            let len = GetSystemDirectoryW(Some(&mut buffer));
            if len > 0 {
                let path = String::from_utf16_lossy(&buffer[..len as usize]);
                Ok(PathBuf::from(path))
            } else {
                Err(Error::PathValidationFailed("无法获取系统目录".to_string()))
            }
        }
    }

    #[cfg(not(windows))]
    fn get_disk_type_unix(&self, _drive: &str) -> Result<DiskType> {
        // Unix 系统简化实现
        Ok(DiskType::Fixed)
    }

    #[cfg(not(windows))]
    fn get_disk_space_unix(&self, path: &str) -> Result<(u64, u64)> {
        use std::process::Command;
        
        let output = Command::new("df")
            .args(&["-B1", path])
            .output()
            .map_err(|e| Error::PathValidationFailed(format!("无法执行 df 命令: {}", e)))?;

        if !output.status.success() {
            return Err(Error::PathValidationFailed("df 命令执行失败".to_string()));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = output_str.lines().collect();
        
        if lines.len() < 2 {
            return Err(Error::PathValidationFailed("df 输出格式错误".to_string()));
        }

        let parts: Vec<&str> = lines[1].split_whitespace().collect();
        if parts.len() < 4 {
            return Err(Error::PathValidationFailed("df 输出字段不足".to_string()));
        }

        let total_bytes = parts[1].parse::<u64>()
            .map_err(|_| Error::PathValidationFailed("无法解析总空间".to_string()))?;
        let free_bytes = parts[3].parse::<u64>()
            .map_err(|_| Error::PathValidationFailed("无法解析可用空间".to_string()))?;

        Ok((free_bytes, total_bytes))
    }
}

/// 便捷函数：校验安装路径
pub fn validate_install_path(path: &Path, required_space_mb: u64) -> Result<PathValidationResult> {
    let validator = PathValidator::new(required_space_mb * 1024 * 1024);
    validator.validate_path(path)
}

/// 便捷函数：检查路径是否适合安装
pub fn is_path_suitable_for_installation(path: &Path, required_space_mb: u64) -> Result<bool> {
    let result = validate_install_path(path, required_space_mb)?;
    Ok(result.is_valid && result.disk_type.is_suitable_for_installation())
}
