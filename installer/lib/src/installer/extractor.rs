use crate::common::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use tracing::{error, info, warn};

/// 7z解压器
pub struct SevenZipExtractor {
    progress_callback: Option<Box<dyn Fn(f32) + Send + Sync>>,
}

impl SevenZipExtractor {
    pub fn new() -> Self {
        Self {
            progress_callback: None,
        }
    }

    /// 设置进度回调
    pub fn set_progress_callback<F>(&mut self, callback: F)
    where
        F: Fn(f32) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
    }

    /// 解压7z文件到指定目录
    pub fn extract_7z_to_dir(&self, archive_data: &[u8], target_dir: &Path) -> Result<(), Error> {
        info!("Starting 7z extraction to: {:?}", target_dir);

        // 确保目标目录存在
        std::fs::create_dir_all(target_dir).map_err(|e| {
            Error::InstallationFailed(format!("Failed to create target directory: {}", e))
        })?;

        // 创建临时文件 (unique name to avoid conflicts)
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_file = std::env::temp_dir().join(format!("nano_installer_{}.7z", unique_id));
        std::fs::write(&temp_file, archive_data)
            .map_err(|e| Error::InstallationFailed(format!("Failed to write temp file: {}", e)))?;

        // 使用系统7z命令解压
        let result = self.extract_with_system_7z(&temp_file, target_dir);

        // 清理临时文件
        let _ = std::fs::remove_file(&temp_file);

        result
    }

    /// 使用系统7z命令解压
    fn extract_with_system_7z(&self, archive_path: &Path, target_dir: &Path) -> Result<(), Error> {
        // 尝试不同的7z命令
        let commands = ["7z", "7za", "7zr"];

        for cmd in &commands {
            if let Ok(output) = std::process::Command::new(cmd)
                .arg("x")
                .arg(archive_path)
                .arg(format!("-o{}", target_dir.display()))
                .arg("-y") // 自动确认
                .output()
            {
                if output.status.success() {
                    info!("Successfully extracted using {}", cmd);
                    return Ok(());
                } else {
                    warn!(
                        "Failed to extract with {}: {}",
                        cmd,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
        }

        // 如果系统7z不可用，尝试使用Rust库
        self.extract_with_rust_library(archive_path, target_dir)
    }

    /// 使用Rust库解压（备用方案）
    fn extract_with_rust_library(
        &self,
        archive_path: &Path,
        target_dir: &Path,
    ) -> Result<(), Error> {
        info!(
            "Extracting 7z archive using sevenz-rust library: {:?}",
            archive_path
        );

        // Report initial progress
        if let Some(ref cb) = self.progress_callback {
            cb(0.0);
        }

        sevenz_rust::decompress_file(archive_path, target_dir)
            .map_err(|e| Error::InstallationFailed(format!("7z extraction failed: {}", e)))?;

        // Report completion
        if let Some(ref cb) = self.progress_callback {
            cb(1.0);
        }

        info!("7z extraction completed successfully using sevenz-rust");
        Ok(())
    }

    /// 异步解压（在后台线程中执行）
    pub fn extract_async(
        self,
        archive_data: Vec<u8>,
        target_dir: PathBuf,
    ) -> Result<Arc<Mutex<ExtractionResult>>, Error> {
        let result = Arc::new(Mutex::new(ExtractionResult::new()));
        let result_clone = Arc::clone(&result);
        let progress_callback = self.progress_callback;

        thread::spawn(move || {
            {
                let mut extraction_result = result_clone.lock().unwrap();
                extraction_result.status = ExtractionStatus::Running;
            }

            // Set up a real extractor with progress reporting that updates both
            // the shared result and the user-provided callback
            let result_for_progress = Arc::clone(&result_clone);
            let mut extractor = SevenZipExtractor::new();
            extractor.set_progress_callback(move |progress| {
                if let Ok(mut r) = result_for_progress.lock() {
                    r.progress = progress;
                }
                if let Some(ref callback) = progress_callback {
                    callback(progress);
                }
            });

            match extractor.extract_7z_to_dir(&archive_data, &target_dir) {
                Ok(_) => {
                    let mut extraction_result = result_clone.lock().unwrap();
                    extraction_result.status = ExtractionStatus::Completed;
                    extraction_result.progress = 1.0;
                    info!("Extraction completed successfully");
                }
                Err(e) => {
                    let mut extraction_result = result_clone.lock().unwrap();
                    extraction_result.status = ExtractionStatus::Failed;
                    extraction_result.error = Some(e.to_string());
                    error!("Extraction failed: {}", e);
                }
            }
        });

        Ok(result)
    }
}

/// 解压结果
#[derive(Debug)]
pub struct ExtractionResult {
    pub status: ExtractionStatus,
    pub progress: f32,
    pub error: Option<String>,
}

impl ExtractionResult {
    pub fn new() -> Self {
        Self {
            status: ExtractionStatus::Pending,
            progress: 0.0,
            error: None,
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, ExtractionStatus::Completed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, ExtractionStatus::Failed)
    }

    pub fn get_progress(&self) -> f32 {
        self.progress
    }

    pub fn get_error(&self) -> Option<&String> {
        self.error.as_ref()
    }
}

/// 解压状态
#[derive(Debug, PartialEq)]
pub enum ExtractionStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl Default for SevenZipExtractor {
    fn default() -> Self {
        Self::new()
    }
}
