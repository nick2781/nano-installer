// Payload 提取器（从 exe 中提取嵌入的 7z 文件）

use crate::common::{Error, Result};
use std::io::{Read, Write};
use std::path::Path;

/// Payload 提取器
pub struct PayloadExtractor;

impl PayloadExtractor {
    /// 获取 7za.exe 的路径
    fn get_7za_path() -> Result<std::path::PathBuf> {
        // 首先尝试从当前 exe 所在目录的 tools 子目录
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let tools_7za = exe_dir.join("tools").join("7za.exe");
                if tools_7za.exists() {
                    return Ok(tools_7za);
                }
            }
        }
        
        // 尝试从项目根目录的 tools 子目录（开发环境）
        if let Ok(cargo_manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let project_7za = std::path::PathBuf::from(cargo_manifest_dir).join("tools").join("7za.exe");
            if project_7za.exists() {
                return Ok(project_7za);
            }
        }
        
        // 如果都不存在，尝试查找当前目录
        let current_7za = std::path::PathBuf::from("tools").join("7za.exe");
        if current_7za.exists() {
            return Ok(current_7za);
        }
        
        Err(Error::Archive("7za.exe not found in tools directory".to_string()))
    }

    /// 从当前 exe 中提取嵌入的 payload
    pub fn extract_embedded_payload() -> Result<Vec<u8>> {
        // 读取当前 exe 文件
        let exe_path = std::env::current_exe()?;
        let exe_data = std::fs::read(&exe_path)?;

        // 在 exe 末尾查找 payload 标记
        // 标记格式: [8字节魔数][8字节payload大小][payload数据]
        const MAGIC: &[u8; 8] = b"PAYLOAD\0";

        // 从文件末尾向前搜索
        if exe_data.len() < 16 {
            return Err(Error::Archive("Exe file too small".to_string()));
        }

        // 读取文件末尾的大小信息
        let size_offset = exe_data.len() - 8;
        let size_bytes = &exe_data[size_offset..];
        let payload_size = u64::from_le_bytes(size_bytes.try_into().unwrap()) as usize;

        // 验证大小合理性
        if payload_size == 0 || payload_size > exe_data.len() {
            return Err(Error::Archive("Invalid payload size".to_string()));
        }

        // 读取魔数
        let magic_offset = size_offset - 8;
        let magic_bytes = &exe_data[magic_offset..magic_offset + 8];

        if magic_bytes != MAGIC {
            return Err(Error::Archive("Payload magic not found".to_string()));
        }

        // 提取 payload 数据
        let payload_offset = magic_offset - payload_size;
        let payload_data = exe_data[payload_offset..magic_offset].to_vec();

        tracing::info!("Extracted payload: {} bytes", payload_data.len());

        Ok(payload_data)
    }

    /// 解压 7z 数据到目标目录
    pub fn extract_7z_to_dir(data: &[u8], dest_dir: &Path) -> Result<()> {
        use std::io::Cursor;

        tracing::info!("Extracting 7z archive to {:?}", dest_dir);

        std::fs::create_dir_all(dest_dir)?;

        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let temp_7z = temp_dir.join(format!("payload_{}.7z", uuid::Uuid::new_v4()));

        {
            let mut file = std::fs::File::create(&temp_7z)?;
            file.write_all(data)?;
        }

        // 使用系统7z命令解压
        let output = std::process::Command::new("7z")
            .arg("x")
            .arg(&temp_7z)
            .arg(format!("-o{}", dest_dir.display()))
            .arg("-y") // 自动确认
            .output()
            .map_err(|e| Error::Archive(format!("Failed to run 7z command: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Archive(format!(
                "7z extraction failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // 删除临时文件
        let _ = std::fs::remove_file(&temp_7z);

        tracing::info!("Extraction completed");

        Ok(())
    }

    /// 计算 SHA256
    pub fn calculate_sha256(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

/// 用于构建时嵌入 payload 的工具
pub mod builder {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::path::Path;

    /// 将 payload 附加到 exe 文件
    pub fn append_payload_to_exe(
        exe_path: &Path,
        payload_path: &Path,
        output_path: &Path,
    ) -> std::io::Result<()> {
        const MAGIC: &[u8; 8] = b"PAYLOAD\0";

        // 读取原始 exe
        let exe_data = std::fs::read(exe_path)?;

        // 读取 payload
        let payload_data = std::fs::read(payload_path)?;
        let payload_size = payload_data.len() as u64;

        // 写入新的 exe
        let mut output = File::create(output_path)?;

        // 写入原始 exe
        output.write_all(&exe_data)?;

        // 写入 payload 数据
        output.write_all(&payload_data)?;

        // 写入魔数
        output.write_all(MAGIC)?;

        // 写入 payload 大小
        output.write_all(&payload_size.to_le_bytes())?;

        output.flush()?;

        println!("Payload appended: {} bytes", payload_size);
        println!("Output file: {:?}", output_path);

        Ok(())
    }
}
