// Path validation for install directory

use anyhow::Result;
use std::path::Path;

#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW};

/// Path validation result
pub struct PathValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub disk_type: DiskType,
    pub free_space_bytes: u64,
    pub total_space_bytes: u64,
    pub required_space_bytes: u64,
}

/// Disk type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiskType {
    Fixed,
    Removable,
    Unknown,
    Network,
    CdRom,
    RamDisk,
}

impl Default for DiskType {
    fn default() -> Self {
        DiskType::Unknown
    }
}

/// Path validator
pub struct PathValidator {
    required_space_bytes: u64,
    allow_removable_drives: bool,
    allow_network_drives: bool,
}

impl PathValidator {
    /// Create a new path validator
    pub fn new(required_space_bytes: u64) -> Self {
        Self {
            required_space_bytes,
            allow_removable_drives: true,
            allow_network_drives: false,
        }
    }

    /// Set whether to allow removable drives
    pub fn set_allow_removable_drives(&mut self, allow: bool) {
        self.allow_removable_drives = allow;
    }

    /// Set whether to allow network drives
    pub fn set_allow_network_drives(&mut self, allow: bool) {
        self.allow_network_drives = allow;
    }

    /// Validate install path
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

        // 1. Check path format
        self.validate_path_format(path, &mut result)?;

        // 2. Check disk type
        self.validate_disk_type(path, &mut result)?;

        // 3. Check disk space
        self.validate_disk_space(path, &mut result)?;

        // 4. Check path permissions
        self.validate_path_permissions(path, &mut result)?;

        // 5. Check path length
        self.validate_path_length(path, &mut result)?;

        // 6. Check special characters
        self.validate_special_characters(path, &mut result)?;

        result.is_valid = result.errors.is_empty();

        Ok(result)
    }

    /// Validate path format
    fn validate_path_format(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        if path.to_string_lossy().is_empty() {
            result
                .errors
                .push("Install path cannot be empty".to_string());
            return Ok(());
        }

        // Check for invalid characters (skip Windows drive letter ':')
        let path_str = path.to_string_lossy();
        // Windows: "C:\foo" -> check "\foo" portion, skip drive "C:"
        let check_str = if cfg!(windows) && path_str.len() >= 2 && path_str.as_bytes()[1] == b':' {
            &path_str[2..]
        } else {
            &path_str
        };
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];

        for ch in invalid_chars {
            if check_str.contains(ch) {
                result
                    .errors
                    .push(format!("Path contains illegal character: '{}'", ch));
            }
        }

        // Check trailing backslash (Windows)
        #[cfg(windows)]
        {
            if path_str.ends_with('\\') && path_str.len() > 3 {
                result
                    .warnings
                    .push("Trailing backslash will be removed automatically".to_string());
            }
        }

        Ok(())
    }

    /// Validate disk type
    fn validate_disk_type(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        let drive = self.get_drive_from_path(path);
        result.disk_type = self.get_disk_type(&drive)?;

        match result.disk_type {
            DiskType::Fixed => {
                // Fixed disk, suitable for installation
            }
            DiskType::Removable => {
                if !self.allow_removable_drives {
                    result
                        .errors
                        .push("Installation on removable disk is not allowed".to_string());
                } else {
                    result
                        .warnings
                        .push("Installation on removable disk may cause issues".to_string());
                }
            }
            DiskType::Network => {
                if !self.allow_network_drives {
                    result
                        .errors
                        .push("Installation on network disk is not allowed".to_string());
                } else {
                    result.warnings.push(
                        "Installation on network disk may cause performance issues".to_string(),
                    );
                }
            }
            DiskType::CdRom => {
                result
                    .errors
                    .push("Cannot install on CD/DVD drive".to_string());
            }
            DiskType::RamDisk => {
                result
                    .warnings
                    .push("Installation on RAM disk will be lost after reboot".to_string());
            }
            DiskType::Unknown => {
                result
                    .warnings
                    .push("Unknown disk type, installation may be unstable".to_string());
            }
        }

        Ok(())
    }

    /// Validate disk space
    fn validate_disk_space(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        let drive = self.get_drive_from_path(path);
        let (free_space, total_space) = self.get_disk_space(&drive)?;

        result.free_space_bytes = free_space;
        result.total_space_bytes = total_space;

        if free_space < self.required_space_bytes {
            result.errors.push(format!(
                "Insufficient disk space. Required: {} MB, Available: {} MB",
                self.required_space_bytes / (1024 * 1024),
                free_space / (1024 * 1024)
            ));
        } else if free_space < self.required_space_bytes * 2 {
            result.warnings.push(format!(
                "Low disk space. Recommended: at least {} MB free",
                self.required_space_bytes * 2 / (1024 * 1024)
            ));
        }

        Ok(())
    }

    /// Validate path permissions
    fn validate_path_permissions(
        &self,
        path: &Path,
        result: &mut PathValidationResult,
    ) -> Result<()> {
        // Check if parent directory exists and is writable
        // Write permission is checked as a warning, not a blocking error.
        // The actual installation (create_dir_all, file extraction) will produce
        // a real error if permission is truly insufficient.
        if path.exists() {
            let test_file = path.join(".installer_test_write");
            if std::fs::write(&test_file, b"test").is_err() {
                result
                    .warnings
                    .push("May require elevated permissions".to_string());
            } else {
                let _ = std::fs::remove_file(&test_file);
            }
        } else if let Some(parent) = path.parent() {
            if parent.exists() {
                let test_file = parent.join(".installer_test_write");
                if std::fs::write(&test_file, b"test").is_err() {
                    result
                        .warnings
                        .push("May require elevated permissions".to_string());
                } else {
                    let _ = std::fs::remove_file(&test_file);
                }
            }
        }

        // Check if in system directory
        #[cfg(windows)]
        {
            if let Ok(system_dir) = self.get_system_directory() {
                if path.starts_with(&system_dir) {
                    result
                        .warnings
                        .push("Installing in system directory is not recommended".to_string());
                }
            }
        }

        Ok(())
    }

    /// Validate path length
    fn validate_path_length(&self, path: &Path, result: &mut PathValidationResult) -> Result<()> {
        let path_str = path.to_string_lossy();

        // Windows path length limit
        #[cfg(windows)]
        {
            if path_str.len() > 260 {
                result
                    .errors
                    .push("Path exceeds 260 character limit".to_string());
            }
        }

        // Check if too short
        if path_str.len() < 3 {
            result.errors.push("Path is too short".to_string());
        }

        Ok(())
    }

    /// Validate special characters
    fn validate_special_characters(
        &self,
        path: &Path,
        result: &mut PathValidationResult,
    ) -> Result<()> {
        let path_str = path.to_string_lossy();

        // Check for relative path symbols
        if path_str.contains("..") {
            result
                .warnings
                .push("Path contains relative path symbols, may cause security issues".to_string());
        }

        // Check for spaces
        if path_str.contains(' ') {
            result
                .warnings
                .push("Path contains spaces, some programs may not work properly".to_string());
        }

        // Check for non-ASCII characters
        if path_str.chars().any(|c| c as u32 > 127) {
            result
                .warnings
                .push("Path contains non-ASCII characters, English path recommended".to_string());
        }

        Ok(())
    }

    /// Get drive letter from path
    fn get_drive_from_path(&self, path: &Path) -> String {
        let path_str = path.to_string_lossy();
        if path_str.len() >= 2 && path_str.as_bytes()[1] == b':' {
            format!("{}:\\", &path_str[0..1])
        } else {
            "C:\\".to_string()
        }
    }

    /// Get disk type
    #[cfg(windows)]
    fn get_disk_type(&self, drive: &str) -> Result<DiskType> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let wide: Vec<u16> = OsStr::new(drive)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(wide.as_ptr())) };

        Ok(match drive_type {
            2 => DiskType::Removable,
            3 => DiskType::Fixed,
            4 => DiskType::Network,
            5 => DiskType::CdRom,
            6 => DiskType::RamDisk,
            _ => DiskType::Unknown,
        })
    }

    #[cfg(not(windows))]
    fn get_disk_type(&self, _drive: &str) -> Result<DiskType> {
        Ok(DiskType::Fixed)
    }

    /// Get disk space
    #[cfg(windows)]
    fn get_disk_space(&self, drive: &str) -> Result<(u64, u64)> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let wide: Vec<u16> = OsStr::new(drive)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut free_bytes_available: u64 = 0;
        let mut total_number_of_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;

        let result = unsafe {
            GetDiskFreeSpaceExW(
                windows::core::PCWSTR(wide.as_ptr()),
                Some(&mut free_bytes_available as *mut u64),
                Some(&mut total_number_of_bytes as *mut u64),
                Some(&mut total_free_bytes as *mut u64),
            )
        };

        match result {
            Ok(()) => Ok((free_bytes_available, total_number_of_bytes)),
            Err(e) => {
                tracing::warn!("Failed to get disk space for {}: {}", drive, e);
                Err(anyhow::anyhow!("Failed to get disk space info: {}", e))
            }
        }
    }

    #[cfg(windows)]
    fn get_system_directory(&self) -> Result<std::path::PathBuf> {
        std::env::var("SystemRoot")
            .map(std::path::PathBuf::from)
            .map_err(|_| anyhow::anyhow!("Cannot get system directory"))
    }

    #[cfg(not(windows))]
    fn get_disk_space(&self, _drive: &str) -> Result<(u64, u64)> {
        // Unix simplified implementation
        let output = std::process::Command::new("df")
            .arg("-B1")
            .arg("/")
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute df: {}", e))?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("df command failed"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        if lines.len() < 2 {
            return Err(anyhow::anyhow!("df output format error"));
        }

        let fields: Vec<&str> = lines[1].split_whitespace().collect();
        if fields.len() < 4 {
            return Err(anyhow::anyhow!("df output fields insufficient"));
        }

        let total: u64 = fields[1]
            .parse()
            .map_err(|_| anyhow::anyhow!("Cannot parse total space"))?;
        let available: u64 = fields[3]
            .parse()
            .map_err(|_| anyhow::anyhow!("Cannot parse available space"))?;

        Ok((available, total))
    }
}

/// Convenience function: validate install path
pub fn validate_install_path(path: &Path, required_space_mb: u64) -> Result<PathValidationResult> {
    let validator = PathValidator::new(required_space_mb * 1024 * 1024);
    validator.validate_path(path)
}

/// Convenience function: check if path is suitable for installation
pub fn is_valid_install_path(path: &Path, required_space_mb: u64) -> bool {
    validate_install_path(path, required_space_mb)
        .map(|r| r.is_valid)
        .unwrap_or(false)
}
