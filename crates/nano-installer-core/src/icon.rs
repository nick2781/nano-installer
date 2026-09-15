use anyhow::{bail, Context, Result};
use std::path::Path;

const ICO_HEADER_SIZE: usize = 6;
const ICO_ENTRY_SIZE: usize = 16;

#[derive(Debug)]
struct IconDirectory {
    entries: Vec<IconEntry>,
}

#[derive(Debug)]
struct IconEntry {
    width: u8,
    height: u8,
    color_count: u8,
    reserved: u8,
    planes: u16,
    bit_count: u16,
    size: u32,
    offset: usize,
}

pub fn replace_exe_icon(exe_path: &Path, icon_path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::LibraryLoader::{
        BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{RT_GROUP_ICON, RT_ICON};

    let icon_data = std::fs::read(icon_path)
        .with_context(|| format!("failed to read installer icon: {}", icon_path.display()))?;
    let directory = parse_ico(&icon_data)
        .with_context(|| format!("invalid installer icon: {}", icon_path.display()))?;
    let group_data = create_group_icon_data(&directory);
    let exe_path_wide: Vec<u16> = exe_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = BeginUpdateResourceW(PCWSTR::from_raw(exe_path_wide.as_ptr()), false)
            .with_context(|| format!("failed to open PE resources: {}", exe_path.display()))?;
        let update_result = (|| -> Result<()> {
            for (index, entry) in directory.entries.iter().enumerate() {
                let end = entry.offset + entry.size as usize;
                let icon_bytes = &icon_data[entry.offset..end];
                UpdateResourceW(
                    handle,
                    RT_ICON,
                    resource_id((index + 1) as u16),
                    0x0409,
                    Some(icon_bytes.as_ptr().cast()),
                    entry.size,
                )
                .context("failed to write icon image resource")?;
            }
            UpdateResourceW(
                handle,
                RT_GROUP_ICON,
                resource_id(1),
                0x0409,
                Some(group_data.as_ptr().cast()),
                group_data.len() as u32,
            )
            .context("failed to write icon group resource")?;
            Ok(())
        })();

        if let Err(error) = update_result {
            let _ = EndUpdateResourceW(handle, true);
            return Err(error);
        }
        EndUpdateResourceW(handle, false)
            .with_context(|| format!("failed to commit PE resources: {}", exe_path.display()))?;
    }
    Ok(())
}

fn resource_id(id: u16) -> windows::core::PCWSTR {
    windows::core::PCWSTR::from_raw(id as usize as *const u16)
}

fn parse_ico(data: &[u8]) -> Result<IconDirectory> {
    if data.len() < ICO_HEADER_SIZE {
        bail!("ICO header is truncated");
    }
    if read_u16(data, 0) != 0 || read_u16(data, 2) != 1 {
        bail!("file is not an ICO image");
    }
    let count = read_u16(data, 4) as usize;
    if count == 0 {
        bail!("ICO contains no images");
    }
    let table_size = count
        .checked_mul(ICO_ENTRY_SIZE)
        .and_then(|size| size.checked_add(ICO_HEADER_SIZE))
        .context("ICO directory size overflow")?;
    if data.len() < table_size {
        bail!("ICO directory is truncated");
    }

    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let start = ICO_HEADER_SIZE + index * ICO_ENTRY_SIZE;
        let size = read_u32(data, start + 8);
        let offset = read_u32(data, start + 12) as usize;
        if size == 0 {
            bail!("ICO image {} is empty", index + 1);
        }
        let end = offset
            .checked_add(size as usize)
            .context("ICO image range overflow")?;
        if offset < table_size || end > data.len() {
            bail!("ICO image {} is outside the file", index + 1);
        }
        entries.push(IconEntry {
            width: data[start],
            height: data[start + 1],
            color_count: data[start + 2],
            reserved: data[start + 3],
            planes: read_u16(data, start + 4),
            bit_count: read_u16(data, start + 6),
            size,
            offset,
        });
    }
    Ok(IconDirectory { entries })
}

fn create_group_icon_data(directory: &IconDirectory) -> Vec<u8> {
    let mut data = Vec::with_capacity(ICO_HEADER_SIZE + directory.entries.len() * 14);
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&(directory.entries.len() as u16).to_le_bytes());
    for (index, entry) in directory.entries.iter().enumerate() {
        data.push(entry.width);
        data.push(entry.height);
        data.push(entry.color_count);
        data.push(entry.reserved);
        data.extend_from_slice(&entry.planes.to_le_bytes());
        data.extend_from_slice(&entry.bit_count.to_le_bytes());
        data.extend_from_slice(&entry.size.to_le_bytes());
        data.extend_from_slice(&((index + 1) as u16).to_le_bytes());
    }
    data
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::{create_group_icon_data, parse_ico};

    fn test_ico(offset: u32, size: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&[16, 16, 0, 0]);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&32u16.to_le_bytes());
        data.extend_from_slice(&size.to_le_bytes());
        data.extend_from_slice(&offset.to_le_bytes());
        data.extend_from_slice(&[1, 2, 3, 4]);
        data
    }

    #[test]
    fn parses_ico_and_creates_group_directory() -> anyhow::Result<()> {
        let directory = parse_ico(&test_ico(22, 4))?;
        let group = create_group_icon_data(&directory);
        assert_eq!(directory.entries.len(), 1);
        assert_eq!(&group[0..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(&group[18..20], &[1, 0]);
        Ok(())
    }

    #[test]
    fn rejects_ico_image_outside_file() {
        let error = parse_ico(&test_ico(23, 4)).unwrap_err();
        assert!(error.to_string().contains("outside the file"));
    }
}
