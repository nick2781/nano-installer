/// 资源打包和解包模块（分段式设计）
///
/// 将项目资源分段打包，类似 NSIS：
/// - Segment 1: Config (JSON)
/// - Segment 2: UI Resources (layouts + assets 打包为 7z)
/// - Segment 3: Locales (所有语言包打包为 7z)
/// - Segment 4: Payload (应用程序 7z)
/// - Segment 5: Uninstaller (可执行文件)
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// 资源包魔数
const MAGIC: &[u8; 8] = b"NANORSRC";
const VERSION: u16 = 2; // 版本 2：分段式设计

/// 资源段类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SegmentType {
    Config = 1,      // installer_config.json (不压缩)
    UIResources = 2, // layouts + assets (7z 压缩)
    Locales = 3,     // 语言包 (7z 压缩)
    Payload = 4,     // 应用程序 (7z)
    Uninstaller = 5, // uninst.exe
    Scripts = 6,     // scripts/*.rhai (7z compressed)
}

/// 资源段
#[derive(Debug, Clone)]
pub struct ResourceSegment {
    pub segment_type: SegmentType,
    pub name: String,
    pub data: Vec<u8>,
    pub compressed: bool, // 是否压缩
}

/// 资源包（分段式）
#[derive(Debug)]
pub struct ResourceBundle {
    pub segments: Vec<ResourceSegment>,
}

impl ResourceBundle {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// 添加配置段
    pub fn add_config(&mut self, data: Vec<u8>) -> Result<()> {
        self.segments.push(ResourceSegment {
            segment_type: SegmentType::Config,
            name: "config".to_string(),
            data,
            compressed: false,
        });
        Ok(())
    }

    /// 添加 UI 资源段（layouts + assets 打包为 7z）
    pub fn add_ui_resources(&mut self, files: HashMap<String, Vec<u8>>) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        // 将 HashMap 转换为 Vec 以便打包
        let file_list: Vec<(String, Vec<u8>)> = files.into_iter().collect();

        // 创建内存中的 7z 归档
        let ui_data = Self::compress_files_to_7z(file_list)?;

        self.segments.push(ResourceSegment {
            segment_type: SegmentType::UIResources,
            name: "ui_resources.7z".to_string(),
            data: ui_data,
            compressed: true,
        });

        Ok(())
    }

    /// 添加语言包段（所有 .pak 文件打包为 7z）
    pub fn add_locales(&mut self, files: HashMap<String, Vec<u8>>) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        // 将 HashMap 转换为 Vec 以便打包
        let file_list: Vec<(String, Vec<u8>)> = files.into_iter().collect();

        // 创建内存中的 7z 归档
        let locales_data = Self::compress_files_to_7z(file_list)?;

        self.segments.push(ResourceSegment {
            segment_type: SegmentType::Locales,
            name: "locales.7z".to_string(),
            data: locales_data,
            compressed: true,
        });

        Ok(())
    }

    /// 添加 payload 段
    pub fn add_payload(&mut self, data: Vec<u8>) -> Result<()> {
        self.segments.push(ResourceSegment {
            segment_type: SegmentType::Payload,
            name: "payload.7z".to_string(),
            data,
            compressed: true, // payload 本身已经是 7z
        });
        Ok(())
    }

    /// 添加卸载器段
    pub fn add_uninstaller(&mut self, data: Vec<u8>) -> Result<()> {
        self.segments.push(ResourceSegment {
            segment_type: SegmentType::Uninstaller,
            name: "uninst.exe".to_string(),
            data,
            compressed: false,
        });
        Ok(())
    }

    /// 添加脚本段 (scripts/*.rhai 打包为 7z)
    pub fn add_scripts(&mut self, files: HashMap<String, Vec<u8>>) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        let file_list: Vec<(String, Vec<u8>)> = files.into_iter().collect();
        let compressed = Self::compress_files_to_7z(file_list)?;
        self.segments.push(ResourceSegment {
            segment_type: SegmentType::Scripts,
            name: "scripts.7z".to_string(),
            data: compressed,
            compressed: true,
        });
        Ok(())
    }

    /// 打包成二进制（分段格式）
    pub fn pack(&self) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // 魔数
        buffer.write_all(MAGIC)?;

        // 版本号
        buffer.write_all(&VERSION.to_le_bytes())?;

        // 段数量
        buffer.write_all(&(self.segments.len() as u32).to_le_bytes())?;

        // 目录表偏移（预留位置）
        let toc_offset_pos = buffer.len();
        buffer.write_all(&[0u8; 8])?; // u64 占位

        // 写入所有段数据
        let mut toc_entries = Vec::new();
        for segment in &self.segments {
            let data_offset = buffer.len() as u64;
            let data_size = segment.data.len() as u64;

            buffer.write_all(&segment.data)?;

            toc_entries.push((
                segment.segment_type,
                segment.name.clone(),
                data_offset,
                data_size,
                segment.compressed,
            ));
        }

        // 记录目录表位置
        let toc_offset = buffer.len() as u64;

        // 写入目录表
        for (segment_type, name, offset, size, compressed) in toc_entries {
            buffer.write_all(&[segment_type as u8])?;

            let name_bytes = name.as_bytes();
            buffer.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
            buffer.write_all(name_bytes)?;

            buffer.write_all(&offset.to_le_bytes())?;
            buffer.write_all(&size.to_le_bytes())?;
            buffer.write_all(&[compressed as u8])?;
        }

        // 回写目录表偏移
        {
            let mut cursor = Cursor::new(&mut buffer);
            cursor.seek(SeekFrom::Start(toc_offset_pos as u64))?;
            cursor.write_all(&toc_offset.to_le_bytes())?;
        }

        // CRC32 校验（整个包）
        let crc = crc32fast::hash(&buffer);
        buffer.write_all(&crc.to_le_bytes())?;

        Ok(buffer)
    }

    /// 从二进制解包（分段格式）
    pub fn unpack(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);

        // 读取魔数
        let mut magic = [0u8; 8];
        cursor.read_exact(&mut magic)?;
        if &magic != MAGIC {
            bail!("Invalid resource bundle magic number");
        }

        // 读取版本
        let mut version_bytes = [0u8; 2];
        cursor.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);

        if version != VERSION && version != 1 {
            // 兼容旧版本
            bail!("Unsupported resource bundle version: {}", version);
        }

        // 读取段数量
        let mut count_bytes = [0u8; 4];
        cursor.read_exact(&mut count_bytes)?;
        let segment_count = u32::from_le_bytes(count_bytes);

        // 读取 TOC 偏移
        let mut toc_offset_bytes = [0u8; 8];
        cursor.read_exact(&mut toc_offset_bytes)?;
        let toc_offset = u64::from_le_bytes(toc_offset_bytes);

        // 跳到 TOC
        cursor.seek(SeekFrom::Start(toc_offset))?;

        // 读取 TOC 条目
        let mut segments = Vec::new();
        for _ in 0..segment_count {
            let mut type_byte = [0u8; 1];
            cursor.read_exact(&mut type_byte)?;
            let segment_type = match type_byte[0] {
                1 => SegmentType::Config,
                2 => SegmentType::UIResources,
                3 => SegmentType::Locales,
                4 => SegmentType::Payload,
                5 => SegmentType::Uninstaller,
                6 => SegmentType::Scripts,
                _ => {
                    tracing::warn!("Unknown segment type: {}, skipping", type_byte[0]);
                    continue;
                }
            };

            let mut name_len_bytes = [0u8; 2];
            cursor.read_exact(&mut name_len_bytes)?;
            let name_len = u16::from_le_bytes(name_len_bytes);

            let mut name_bytes = vec![0u8; name_len as usize];
            cursor.read_exact(&mut name_bytes)?;
            let name = String::from_utf8(name_bytes)?;

            let mut offset_bytes = [0u8; 8];
            cursor.read_exact(&mut offset_bytes)?;
            let offset = u64::from_le_bytes(offset_bytes);

            let mut size_bytes = [0u8; 8];
            cursor.read_exact(&mut size_bytes)?;
            let size = u64::from_le_bytes(size_bytes);

            let mut compressed_byte = [0u8; 1];
            cursor.read_exact(&mut compressed_byte)?;
            let compressed = compressed_byte[0] != 0;

            // 读取段数据
            let segment_data = data[offset as usize..(offset + size) as usize].to_vec();

            segments.push(ResourceSegment {
                segment_type,
                name,
                data: segment_data,
                compressed,
            });
        }

        tracing::debug!("unpack: Total segments parsed: {}", segments.len());
        for (i, seg) in segments.iter().enumerate() {
            tracing::debug!(
                "  Segment {}: type={:?}, name={}, size={}, compressed={}",
                i + 1,
                seg.segment_type,
                seg.name,
                seg.data.len(),
                seg.compressed
            );
        }

        Ok(Self { segments })
    }

    /// 获取指定类型的段
    pub fn get_segment(&self, segment_type: SegmentType) -> Option<&ResourceSegment> {
        self.segments
            .iter()
            .find(|s| s.segment_type == segment_type)
    }

    /// 压缩目录到 7z（内部实现，使用简单的 tar + gzip 模拟）
    fn compress_directories_to_7z(dirs: &[(&str, &Path)], base_dir: &Path) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());

        // 简单的打包格式：
        // [文件数量: u32]
        // 对于每个文件：
        //   [路径长度: u16][路径: UTF-8][数据长度: u64][数据]

        let mut file_list = Vec::new();

        for (prefix, dir) in dirs {
            if !dir.exists() {
                continue;
            }
            Self::collect_files(dir, base_dir, prefix, &mut file_list)?;
        }

        // 写入文件数量
        encoder.write_all(&(file_list.len() as u32).to_le_bytes())?;

        // 写入每个文件
        for (path, data) in file_list {
            let path_bytes = path.as_bytes();
            encoder.write_all(&(path_bytes.len() as u16).to_le_bytes())?;
            encoder.write_all(path_bytes)?;
            encoder.write_all(&(data.len() as u64).to_le_bytes())?;
            encoder.write_all(&data)?;
        }

        let compressed = encoder.finish()?;
        Ok(compressed)
    }

    /// 压缩文件列表到 7z
    /// 使用内置 LZMA 压缩文件（类似 7z 格式）
    ///
    /// 使用 lzma-rs 进行压缩，无需外部工具
    fn compress_files_to_7z(files: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>> {
        use lzma_rs::lzma_compress;

        eprintln!(
            "      [DEBUG] compress_files_to_7z: Compressing {} files",
            files.len()
        );

        // 首先将所有文件打包成一个未压缩的归档
        let mut uncompressed = Vec::new();

        // 写入文件数量
        uncompressed.write_all(&(files.len() as u32).to_le_bytes())?;

        eprintln!(
            "      [DEBUG] compress_files_to_7z: Wrote file count: {}",
            files.len()
        );

        // 写入每个文件
        for (name, data) in &files {
            let name_bytes = name.as_bytes();
            uncompressed.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
            uncompressed.write_all(name_bytes)?;
            uncompressed.write_all(&(data.len() as u64).to_le_bytes())?;
            uncompressed.write_all(data)?;
        }

        // 使用 LZMA 压缩整个归档
        let mut compressed = Vec::new();
        lzma_compress(&mut Cursor::new(&uncompressed), &mut compressed)
            .context("Failed to compress with LZMA")?;

        eprintln!(
            "      [DEBUG] compress_files_to_7z: Compressed {} bytes -> {} bytes",
            uncompressed.len(),
            compressed.len()
        );

        Ok(compressed)
    }

    /// 备用压缩方法：使用 gzip（如果 7za.exe 不可用）
    fn compress_files_to_gzip(files: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());

        // 写入文件数量
        encoder.write_all(&(files.len() as u32).to_le_bytes())?;

        // 写入每个文件
        for (name, data) in files {
            let name_bytes = name.as_bytes();
            encoder.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
            encoder.write_all(name_bytes)?;
            encoder.write_all(&(data.len() as u64).to_le_bytes())?;
            encoder.write_all(&data)?;
        }

        let compressed = encoder.finish()?;
        Ok(compressed)
    }

    /// 解压段数据（支持 LZMA 和 gzip 格式）
    pub fn decompress_segment(
        &self,
        segment_type: SegmentType,
    ) -> Result<HashMap<String, Vec<u8>>> {
        tracing::debug!("Decompressing segment: {:?}", segment_type);

        let segment = self
            .get_segment(segment_type)
            .context("Segment not found")?;

        tracing::debug!(
            "Segment found: name={}, compressed={}, size={} bytes",
            segment.name,
            segment.compressed,
            segment.data.len()
        );

        if !segment.compressed {
            // 不压缩的段，直接返回
            let mut map = HashMap::new();
            map.insert(segment.name.clone(), segment.data.clone());
            tracing::debug!("Segment not compressed, returning as-is");
            return Ok(map);
        }

        // 尝试作为 LZMA 格式解压
        tracing::debug!("Trying LZMA decompression...");
        if let Ok(files) = Self::decompress_lzma(&segment.data) {
            tracing::debug!(
                "LZMA decompression succeeded, {} files extracted",
                files.len()
            );
            return Ok(files);
        }

        // 回退到 gzip 格式（兼容旧版本）
        tracing::debug!("LZMA failed, trying gzip...");
        let result = Self::decompress_gzip(&segment.data);
        if let Ok(ref files) = result {
            tracing::debug!(
                "Gzip decompression succeeded, {} files extracted",
                files.len()
            );
        } else {
            tracing::error!("All decompression methods failed");
        }
        result
    }

    /// 解压 LZMA 格式数据
    fn decompress_lzma(data: &[u8]) -> Result<HashMap<String, Vec<u8>>> {
        use lzma_rs::lzma_decompress;

        tracing::debug!(
            "Starting LZMA decompression, compressed size: {} bytes",
            data.len()
        );

        // 使用 LZMA 解压
        let mut decompressed = Vec::new();
        lzma_decompress(&mut Cursor::new(data), &mut decompressed)
            .context("Failed to decompress LZMA data")?;

        tracing::debug!("LZMA decompressed to {} bytes", decompressed.len());

        // 解析归档格式
        let mut cursor = Cursor::new(&decompressed);
        let mut files = HashMap::new();

        // 读取文件数量
        let mut count_bytes = [0u8; 4];
        cursor.read_exact(&mut count_bytes)?;
        let file_count = u32::from_le_bytes(count_bytes);

        tracing::debug!("Archive contains {} files", file_count);

        // 读取每个文件
        for i in 0..file_count {
            let mut path_len_bytes = [0u8; 2];
            cursor.read_exact(&mut path_len_bytes)?;
            let path_len = u16::from_le_bytes(path_len_bytes);

            let mut path_bytes = vec![0u8; path_len as usize];
            cursor.read_exact(&mut path_bytes)?;
            let path = String::from_utf8(path_bytes)?;

            let mut size_bytes = [0u8; 8];
            cursor.read_exact(&mut size_bytes)?;
            let size = u64::from_le_bytes(size_bytes);

            tracing::debug!("  File {}/{}: {} ({} bytes)", i + 1, file_count, path, size);

            let mut file_data = vec![0u8; size as usize];
            cursor.read_exact(&mut file_data)?;

            files.insert(path, file_data);
        }

        tracing::debug!("Successfully extracted {} files", files.len());
        Ok(files)
    }

    /// 解压 gzip 格式数据（备用/兼容旧版本）
    fn decompress_gzip(data: &[u8]) -> Result<HashMap<String, Vec<u8>>> {
        use flate2::read::GzDecoder;

        let mut decoder = GzDecoder::new(Cursor::new(data));
        let mut files = HashMap::new();

        // 读取文件数量
        let mut count_bytes = [0u8; 4];
        decoder.read_exact(&mut count_bytes)?;
        let file_count = u32::from_le_bytes(count_bytes);

        // 读取每个文件
        for _ in 0..file_count {
            let mut path_len_bytes = [0u8; 2];
            decoder.read_exact(&mut path_len_bytes)?;
            let path_len = u16::from_le_bytes(path_len_bytes);

            let mut path_bytes = vec![0u8; path_len as usize];
            decoder.read_exact(&mut path_bytes)?;
            let path = String::from_utf8(path_bytes)?;

            let mut size_bytes = [0u8; 8];
            decoder.read_exact(&mut size_bytes)?;
            let size = u64::from_le_bytes(size_bytes);

            let mut data = vec![0u8; size as usize];
            decoder.read_exact(&mut data)?;

            files.insert(path, data);
        }

        Ok(files)
    }

    /// 收集目录中的所有文件
    fn collect_files(
        dir: &Path,
        base_dir: &Path,
        prefix: &str,
        files: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let relative_path = path
                    .strip_prefix(base_dir)
                    .context("Failed to get relative path")?;
                let name = format!(
                    "{}/{}",
                    prefix,
                    relative_path.to_string_lossy().replace('\\', "/")
                );
                let data = fs::read(&path)?;
                files.push((name, data));
            } else if path.is_dir() {
                Self::collect_files(&path, base_dir, prefix, files)?;
            }
        }
        Ok(())
    }
}

/// 将资源包追加到 exe 文件
pub fn append_bundle_to_exe(exe_path: &Path, bundle_data: &[u8]) -> Result<()> {
    use std::fs::OpenOptions;

    eprintln!(
        "      [DEBUG] append_bundle_to_exe: Opening {} to append {} bytes",
        exe_path.display(),
        bundle_data.len()
    );

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(exe_path)
        .context("Failed to open exe file")?;

    let initial_size = file.metadata()?.len();
    eprintln!(
        "      [DEBUG] append_bundle_to_exe: Initial file size: {} bytes",
        initial_size
    );

    file.write_all(bundle_data)
        .context("Failed to write bundle data")?;

    file.sync_all()?;

    let final_size = file.metadata()?.len();
    eprintln!(
        "      [DEBUG] append_bundle_to_exe: Final file size: {} bytes (added {} bytes)",
        final_size,
        final_size - initial_size
    );

    Ok(())
}

/// 从 exe 文件中提取资源包
pub fn extract_bundle_from_exe(exe_path: &Path) -> Result<ResourceBundle> {
    let exe_data = fs::read(exe_path).context("Failed to read exe file")?;

    // 查找所有魔数位置
    let mut magic_positions = Vec::new();
    for (i, window) in exe_data.windows(MAGIC.len()).enumerate() {
        if window == MAGIC {
            magic_positions.push(i);
        }
    }

    if magic_positions.is_empty() {
        bail!("Resource bundle magic number not found in exe");
    }

    tracing::debug!("Found {} NANORSRC magic(s) in exe", magic_positions.len());

    // 从后往前尝试解析，找到第一个有效且段数量 >= 4 的资源包
    // （安装器应该有 5 个段，卸载器只有 3 个段）
    for magic_pos in magic_positions.iter().rev() {
        tracing::debug!("Trying to parse bundle at position {}", magic_pos);
        let bundle_data = &exe_data[*magic_pos..];

        match ResourceBundle::unpack(bundle_data) {
            Ok(bundle) => {
                tracing::debug!(
                    "Successfully parsed bundle with {} segments",
                    bundle.segments.len()
                );
                // 安装器应该有至少 4 个段（Config, UIResources, Locales, Payload）
                // 卸载器只有 3 个段（Config, UIResources, Locales）
                if bundle.segments.len() >= 4 {
                    tracing::info!(
                        "Found installer bundle with {} segments",
                        bundle.segments.len()
                    );
                    return Ok(bundle);
                }
            }
            Err(e) => {
                tracing::debug!("Failed to parse bundle at position {}: {}", magic_pos, e);
            }
        }
    }

    // 如果没找到 >= 4 段的，使用最后一个能解析的
    for magic_pos in magic_positions.iter().rev() {
        let bundle_data = &exe_data[*magic_pos..];
        if let Ok(bundle) = ResourceBundle::unpack(bundle_data) {
            tracing::warn!(
                "Using fallback bundle with {} segments",
                bundle.segments.len()
            );
            return Ok(bundle);
        }
    }

    bail!("No valid resource bundle found in exe")
}

impl ResourceBundle {
    /// 获取资源（兼容旧 API）
    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        // 先尝试直接匹配段名
        if let Some(segment) = self.segments.iter().find(|s| s.name == name) {
            return Some(segment.data.clone());
        }

        // 然后尝试解压并查找
        for segment in &self.segments {
            if segment.compressed {
                if let Ok(files) = self.decompress_segment(segment.segment_type) {
                    if let Some(data) = files.get(name) {
                        return Some(data.clone());
                    }
                }
            }
        }

        None
    }

    /// 根据类型获取资源列表（兼容旧 API）
    pub fn get_by_type(&self, resource_type: ResourceType) -> Vec<String> {
        let segment_type = match resource_type {
            ResourceType::Config => SegmentType::Config,
            ResourceType::Layout | ResourceType::Asset => SegmentType::UIResources,
            ResourceType::Locale => SegmentType::Locales,
            ResourceType::Payload => SegmentType::Payload,
            ResourceType::Uninstaller => SegmentType::Uninstaller,
        };

        if let Some(segment) = self.get_segment(segment_type) {
            if segment.compressed {
                if let Ok(files) = self.decompress_segment(segment_type) {
                    return files.keys().cloned().collect();
                }
            } else {
                return vec![segment.name.clone()];
            }
        }

        Vec::new()
    }
}

// 兼容旧 API（用于 CLI）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceType {
    Config = 1,
    Layout = 2,
    Asset = 3,
    Locale = 4,
    Payload = 5,
    Uninstaller = 6,
}

pub struct ResourceItem {
    pub resource_type: ResourceType,
    pub name: String,
    pub data: Vec<u8>,
}
