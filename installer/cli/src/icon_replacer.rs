/// PE 文件图标和版本信息替换模块
/// 
/// 使用 Windows API 替换 exe 文件的图标资源和版本信息

use anyhow::{Context, Result};
use std::path::Path;

/// 版本信息结构
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub product_name: String,
    pub product_version: String,
    pub file_description: String,
    pub file_version: String,
    pub company_name: Option<String>,
    pub copyright: Option<String>,
}

#[cfg(windows)]
pub fn replace_exe_icon(exe_path: &Path, icon_path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::LibraryLoader::{BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW};
    use windows::Win32::UI::WindowsAndMessaging::{RT_GROUP_ICON, RT_ICON};
    
    // 读取图标文件
    let icon_data = std::fs::read(icon_path)
        .context("Failed to read icon file")?;
    
    // 转换路径为 Windows 宽字符
    let exe_path_wide: Vec<u16> = exe_path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    unsafe {
        // 开始更新资源
        let handle = BeginUpdateResourceW(
            PCWSTR::from_raw(exe_path_wide.as_ptr()),
            false, // 不删除现有资源
        ).context("Failed to begin update resource")?;
        
        // 解析 ICO 文件格式
        let icon_dir = parse_ico_file(&icon_data)?;
        
        // 更新图标资源
        for (idx, entry) in icon_dir.entries.iter().enumerate() {
            let icon_id = (idx + 1) as u16;
            
            // 更新单个图标资源 (RT_ICON)
            let icon_bytes = &icon_data[entry.offset as usize..(entry.offset + entry.size) as usize];
            UpdateResourceW(
                handle,
                RT_ICON,
                PCWSTR::from_raw(icon_id as *const u16),
                0x0409, // LANG_ENGLISH
                Some(icon_bytes.as_ptr() as *const _),
                icon_bytes.len() as u32,
            ).context("Failed to update icon resource")?;
        }
        
        // 更新图标组资源 (RT_GROUP_ICON)
        let group_icon_data = create_group_icon_data(&icon_dir);
        UpdateResourceW(
            handle,
            RT_GROUP_ICON,
            PCWSTR::from_raw(1 as *const u16), // 使用 ID 1
            0x0409, // LANG_ENGLISH
            Some(group_icon_data.as_ptr() as *const _),
            group_icon_data.len() as u32,
        ).context("Failed to update group icon resource")?;
        
        // 提交更改
        EndUpdateResourceW(handle, false)
            .context("Failed to end update resource")?;
    }
    
    Ok(())
}

#[cfg(not(windows))]
pub fn replace_exe_icon(_exe_path: &Path, _icon_path: &Path) -> Result<()> {
    anyhow::bail!("Icon replacement is only supported on Windows")
}

// ICO 文件格式结构
#[derive(Debug)]
struct IconDir {
    entries: Vec<IconDirEntry>,
}

#[derive(Debug)]
struct IconDirEntry {
    width: u8,
    height: u8,
    color_count: u8,
    reserved: u8,
    planes: u16,
    bit_count: u16,
    size: u32,
    offset: u32,
}

fn parse_ico_file(data: &[u8]) -> Result<IconDir> {
    use std::io::{Cursor, Read};
    
    let mut cursor = Cursor::new(data);
    let mut buf = [0u8; 6];
    cursor.read_exact(&mut buf)?;
    
    // 验证 ICO 头
    let reserved = u16::from_le_bytes([buf[0], buf[1]]);
    let icon_type = u16::from_le_bytes([buf[2], buf[3]]);
    let count = u16::from_le_bytes([buf[4], buf[5]]);
    
    if reserved != 0 || icon_type != 1 {
        anyhow::bail!("Invalid ICO file format");
    }
    
    let mut entries = Vec::new();
    for _ in 0..count {
        let mut entry_buf = [0u8; 16];
        cursor.read_exact(&mut entry_buf)?;
        
        entries.push(IconDirEntry {
            width: entry_buf[0],
            height: entry_buf[1],
            color_count: entry_buf[2],
            reserved: entry_buf[3],
            planes: u16::from_le_bytes([entry_buf[4], entry_buf[5]]),
            bit_count: u16::from_le_bytes([entry_buf[6], entry_buf[7]]),
            size: u32::from_le_bytes([entry_buf[8], entry_buf[9], entry_buf[10], entry_buf[11]]),
            offset: u32::from_le_bytes([entry_buf[12], entry_buf[13], entry_buf[14], entry_buf[15]]),
        });
    }
    
    Ok(IconDir { entries })
}

fn create_group_icon_data(icon_dir: &IconDir) -> Vec<u8> {
    let mut data = Vec::new();
    
    // GRPICONDIR 头
    data.extend_from_slice(&0u16.to_le_bytes()); // reserved
    data.extend_from_slice(&1u16.to_le_bytes()); // type (1 = icon)
    data.extend_from_slice(&(icon_dir.entries.len() as u16).to_le_bytes()); // count
    
    // GRPICONDIRENTRY 条目
    for (idx, entry) in icon_dir.entries.iter().enumerate() {
        data.push(entry.width);
        data.push(entry.height);
        data.push(entry.color_count);
        data.push(entry.reserved);
        data.extend_from_slice(&entry.planes.to_le_bytes());
        data.extend_from_slice(&entry.bit_count.to_le_bytes());
        data.extend_from_slice(&entry.size.to_le_bytes());
        data.extend_from_slice(&((idx + 1) as u16).to_le_bytes()); // nID (资源 ID)
    }
    
    data
}

/// 更新 exe 的版本信息
#[cfg(windows)]
pub fn replace_version_info(exe_path: &Path, version_info: &VersionInfo) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::LibraryLoader::{BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW};
    
    // RT_VERSION = 16
    const RT_VERSION: PCWSTR = PCWSTR(16 as *const u16);
    
    // 转换路径为 Windows 宽字符
    let exe_path_wide: Vec<u16> = exe_path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    unsafe {
        // 开始更新资源
        let handle = BeginUpdateResourceW(
            PCWSTR::from_raw(exe_path_wide.as_ptr()),
            false,
        ).context("Failed to begin update resource")?;
        
        // 创建版本信息数据
        let version_data = create_version_info_data(version_info)?;
        
        // 更新版本资源
        UpdateResourceW(
            handle,
            RT_VERSION,
            PCWSTR::from_raw(1 as *const u16), // VS_VERSION_INFO 使用 ID 1
            0x0409, // LANG_ENGLISH
            Some(version_data.as_ptr() as *const _),
            version_data.len() as u32,
        ).context("Failed to update version resource")?;
        
        // 提交更改
        EndUpdateResourceW(handle, false)
            .context("Failed to end update resource")?;
    }
    
    Ok(())
}

#[cfg(not(windows))]
pub fn replace_version_info(_exe_path: &Path, _version_info: &VersionInfo) -> Result<()> {
    anyhow::bail!("Version info replacement is only supported on Windows")
}

/// 创建 VS_VERSION_INFO 结构数据
fn create_version_info_data(info: &VersionInfo) -> Result<Vec<u8>> {
    use crate::version_info_builder::VersionInfoBuilder;
    
    // 解析版本号
    let file_ver = parse_version(&info.file_version);
    let product_ver = parse_version(&info.product_version);
    
    let mut builder = VersionInfoBuilder::new()
        .file_version(file_ver.0, file_ver.1, file_ver.2, file_ver.3)
        .product_version(product_ver.0, product_ver.1, product_ver.2, product_ver.3)
        .add_string("ProductName", &info.product_name)
        .add_string("ProductVersion", &info.product_version)
        .add_string("FileDescription", &info.file_description)
        .add_string("FileVersion", &info.file_version);
    
    if let Some(company) = &info.company_name {
        eprintln!("  [DEBUG] Adding CompanyName: {}", company);
        builder = builder.add_string("CompanyName", company);
    } else {
        eprintln!("  [DEBUG] No CompanyName provided");
    }
    
    if let Some(copyright) = &info.copyright {
        eprintln!("  [DEBUG] Adding LegalCopyright: {}", copyright);
        builder = builder.add_string("LegalCopyright", copyright);
    }
    
    let data = builder.build();
    eprintln!("  [DEBUG] Version info data size: {} bytes", data.len());
    Ok(data)
}

fn parse_version(version_str: &str) -> (u16, u16, u16, u16) {
    let parts: Vec<u16> = version_str
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    (
        parts.get(0).copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
        parts.get(3).copied().unwrap_or(0),
    )
}

