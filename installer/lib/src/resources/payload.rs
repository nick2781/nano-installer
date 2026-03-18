// Payload 提取器（从 exe 中提取嵌入的 7z 文件）

use crate::common::{Error, Result};
use regex::Regex;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Instant;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

/// Payload 提取器
pub struct PayloadExtractor;

impl PayloadExtractor {
    const BUNDLED_7ZA: &'static [u8] =
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/7za.exe"));

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

        Self::ensure_bundled_7za()
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

    /// 解压 7z 数据到目标目录（先尝试系统 7z，回退到 sevenz-rust）
    pub fn extract_7z_to_dir(data: &[u8], dest_dir: &Path) -> Result<()> {
        Self::extract_7z_to_dir_with_progress(data, dest_dir, |_| {})
    }

    /// 解压归档到目标目录，并回调真实进度（0.0 ~ 1.0）
    pub fn extract_7z_to_dir_with_progress<F>(
        data: &[u8],
        dest_dir: &Path,
        mut progress_callback: F,
    ) -> Result<()>
    where
        F: FnMut(f32),
    {
        tracing::info!("Extracting 7z archive to {:?}", dest_dir);
        std::fs::create_dir_all(dest_dir)?;
        progress_callback(0.0);

        // 检测文件格式: 7z 头 = [0x37, 0x7A], ZIP 头 = [0x50, 0x4B]
        let is_zip = data.len() >= 2 && data[0] == 0x50 && data[1] == 0x4B;

        if is_zip {
            Self::extract_zip_with_progress(data, dest_dir, &mut progress_callback)?;
        } else {
            Self::extract_7z_with_progress(data, dest_dir, &mut progress_callback)?;
        }

        progress_callback(1.0);
        tracing::info!("Extraction completed");
        Ok(())
    }

    fn extract_zip_with_progress<F>(
        data: &[u8],
        dest_dir: &Path,
        progress_callback: &mut F,
    ) -> Result<()>
    where
        F: FnMut(f32),
    {
        tracing::info!("Detected ZIP format, using zip crate with progress");
        let stage_started = Instant::now();
        let cursor = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| Error::Archive(format!("ZIP open failed: {}", e)))?;
        let open_elapsed = stage_started.elapsed();
        let scan_started = Instant::now();
        let mut total_bytes = 0u64;
        for index in 0..archive.len() {
            if let Ok(entry) = archive.by_index(index) {
                if !entry.is_dir() {
                    total_bytes += entry.size();
                }
            }
        }
        let scan_elapsed = scan_started.elapsed();
        let total_bytes = total_bytes.max(1);
        let mut extracted_bytes = 0u64;
        let extract_started = Instant::now();

        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|e| Error::Archive(format!("ZIP entry open failed: {}", e)))?;
            let out_path = dest_dir.join(entry.name());

            if entry.is_dir() {
                std::fs::create_dir_all(&out_path)?;
                continue;
            }

            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file = std::fs::File::create(&out_path)?;
            let mut writer = BufWriter::new(file);
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = entry
                    .read(&mut buffer)
                    .map_err(|e| Error::Archive(format!("ZIP extraction failed: {}", e)))?;
                if read == 0 {
                    break;
                }
                writer.write_all(&buffer[..read])?;
                extracted_bytes += read as u64;
                progress_callback((extracted_bytes as f32 / total_bytes as f32).clamp(0.0, 1.0));
            }
        }

        tracing::info!(
            "ZIP extraction timings: open={:.3}s scan={:.3}s extract={:.3}s total_bytes={}",
            open_elapsed.as_secs_f64(),
            scan_elapsed.as_secs_f64(),
            extract_started.elapsed().as_secs_f64(),
            total_bytes
        );
        Ok(())
    }

    fn ensure_bundled_7za() -> Result<std::path::PathBuf> {
        static BUNDLED_7ZA_PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

        if let Some(path) = BUNDLED_7ZA_PATH.get() {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        let oid = &Self::calculate_sha256(Self::BUNDLED_7ZA)[..16];
        let out_dir = std::env::temp_dir().join("nano-installer").join("tools");
        std::fs::create_dir_all(&out_dir)?;
        let out_path = out_dir.join(format!("7za-{}.exe", oid));
        if !out_path.exists() {
            std::fs::write(&out_path, Self::BUNDLED_7ZA)?;
        }

        let _ = BUNDLED_7ZA_PATH.set(out_path.clone());
        Ok(out_path)
    }

    fn parse_7za_progress_fragment(fragment: &str) -> Option<f32> {
        static PROGRESS_RE: OnceLock<Regex> = OnceLock::new();
        let regex =
            PROGRESS_RE.get_or_init(|| Regex::new(r"(?:^|[^0-9])([0-9]{1,3})%").unwrap());

        regex
            .captures_iter(fragment)
            .filter_map(|caps| caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok()))
            .filter(|value| *value <= 100)
            .last()
            .map(|value| value as f32 / 100.0)
    }

    fn extract_7z_with_progress<F>(
        data: &[u8],
        dest_dir: &Path,
        progress_callback: &mut F,
    ) -> Result<()>
    where
        F: FnMut(f32),
    {
        tracing::info!("Using bundled 7za.exe with progress");
        let stage_started = Instant::now();
        let temp_dir = tempfile::tempdir()?;
        let archive_path = temp_dir.path().join("payload.7z");
        std::fs::write(&archive_path, data)?;

        let seven_zip = Self::get_7za_path()?;
        let mut command = Command::new(&seven_zip);
        command
            .arg("x")
            .arg(&archive_path)
            .arg(format!("-o{}", dest_dir.display()))
            .arg("-y")
            .arg("-bso0")
            .arg("-bse0")
            .arg("-bsp1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW.0);
        let mut child = command
            .spawn()
            .map_err(|e| Error::Archive(format!("Failed to launch 7za.exe: {}", e)))?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Archive("7za.exe stdout pipe missing".to_string()))?;
        let prepare_elapsed = stage_started.elapsed();
        let extract_started = Instant::now();
        let mut raw_output = String::new();
        let mut carry = String::new();
        let mut last_progress = 0.0f32;
        let mut buffer = [0u8; 4096];

        loop {
            let read = stdout
                .read(&mut buffer)
                .map_err(|e| Error::Archive(format!("Failed to read 7za progress: {}", e)))?;
            if read == 0 {
                break;
            }

            let chunk = String::from_utf8_lossy(&buffer[..read]);
            raw_output.push_str(&chunk);
            carry.push_str(&chunk);

            if let Some(progress) = Self::parse_7za_progress_fragment(&carry) {
                if progress > last_progress {
                    last_progress = progress;
                    progress_callback(progress);
                }
            }

            if carry.len() > 1024 {
                let split_at = carry.len() - 256;
                carry = carry.split_off(split_at);
            }
        }

        let status = child
            .wait()
            .map_err(|e| Error::Archive(format!("Failed to wait for 7za.exe: {}", e)))?;
        if !status.success() {
            return Err(Error::Archive(format!(
                "7za extraction failed: status={} output={}",
                status,
                raw_output.trim()
            )));
        }

        tracing::info!(
            "7z extraction timings: prepare={:.3}s extract={:.3}s",
            prepare_elapsed.as_secs_f64(),
            extract_started.elapsed().as_secs_f64(),
        );
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

#[cfg(test)]
mod tests {
    use super::PayloadExtractor;
    use crate::installer::artifacts::snapshot_install_tree;
    use std::path::PathBuf;
    use std::sync::Once;
    use std::time::Instant;

    static TRACING_INIT: Once = Once::new();

    fn init_tracing() {
        TRACING_INIT.call_once(|| {
            let _ = tracing_subscriber::fmt()
                .with_env_filter("info")
                .with_test_writer()
                .try_init();
        });
    }

    #[test]
    fn parses_7za_progress_updates() {
        assert_eq!(
            PayloadExtractor::parse_7za_progress_fragment(" 27% 68"),
            Some(0.27)
        );
        assert_eq!(
            PayloadExtractor::parse_7za_progress_fragment(" 99% 620 - libGLESv2.dll"),
            Some(0.99)
        );
    }

    #[test]
    fn ignores_non_progress_7za_output() {
        assert_eq!(
            PayloadExtractor::parse_7za_progress_fragment(
                "  0M Scan D:\\taptap-pc\\nano-installer\\examples\\TapTap-v2\\payload\\"
            ),
            None
        );
        assert_eq!(PayloadExtractor::parse_7za_progress_fragment(""), None);
    }

    #[test]
    #[ignore = "manual timing benchmark"]
    fn benchmark_taptap_v2_payload_extract_timing() {
        init_tracing();

        let payload_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("examples")
            .join("TapTap-v2")
            .join("payload")
            .join("app.7z");
        assert!(
            payload_path.exists(),
            "missing payload fixture: {}",
            payload_path.display()
        );

        let payload_on_disk = std::fs::read(&payload_path).expect("read payload");
        let clone_started = Instant::now();
        let payload = payload_on_disk.clone();
        let clone_elapsed = clone_started.elapsed();

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let extract_started = Instant::now();
        PayloadExtractor::extract_7z_to_dir_with_progress(&payload, temp_dir.path(), |_| {})
            .expect("extract payload");
        let extract_elapsed = extract_started.elapsed();

        let snapshot_started = Instant::now();
        let (files, dirs) = snapshot_install_tree(temp_dir.path()).expect("snapshot install tree");
        let snapshot_elapsed = snapshot_started.elapsed();

        println!(
            "TIMING_SUMMARY clone={:.3}s extract_total={:.3}s snapshot={:.3}s files={} dirs={}",
            clone_elapsed.as_secs_f64(),
            extract_elapsed.as_secs_f64(),
            snapshot_elapsed.as_secs_f64(),
            files.len(),
            dirs.len()
        );
    }
}
