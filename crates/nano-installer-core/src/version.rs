use anyhow::{bail, Context, Result};
use std::path::Path;

const UNICODE_CODE_PAGE: u16 = 1200;

pub struct VersionInfo {
    pub product_name: String,
    pub product_version: String,
    pub file_description: String,
    pub file_version: String,
    pub company_name: Option<String>,
    pub copyright: Option<String>,
    pub internal_name: String,
    pub original_filename: String,
    pub language_id: u16,
}

pub fn replace_exe_version_info(exe_path: &Path, info: &VersionInfo) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::LibraryLoader::{
        BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW,
    };

    let version_data = build_version_info(info)?;
    let exe_path_wide: Vec<u16> = exe_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let handle = BeginUpdateResourceW(PCWSTR::from_raw(exe_path_wide.as_ptr()), false)
            .with_context(|| format!("failed to open PE resources: {}", exe_path.display()))?;
        let update_result = UpdateResourceW(
            handle,
            resource_id(16),
            resource_id(1),
            info.language_id,
            Some(version_data.as_ptr().cast()),
            version_data.len() as u32,
        )
        .context("failed to write version resource");
        if let Err(error) = update_result {
            let _ = EndUpdateResourceW(handle, true);
            return Err(error);
        }
        EndUpdateResourceW(handle, false)
            .with_context(|| format!("failed to commit PE resources: {}", exe_path.display()))?;
    }
    Ok(())
}

pub fn language_id(locale: &str) -> u16 {
    match locale.to_ascii_lowercase().as_str() {
        "zh-cn" => 0x0804,
        "zh-tw" => 0x0404,
        "en-us" => 0x0409,
        "es" | "es-es" => 0x0c0a,
        "id" | "id-id" => 0x0421,
        "ja" | "ja-jp" => 0x0411,
        "ko" | "ko-kr" => 0x0412,
        "pt" | "pt-br" => 0x0416,
        "ru" | "ru-ru" => 0x0419,
        "th" | "th-th" => 0x041e,
        "vi" | "vi-vn" => 0x042a,
        _ => 0x0409,
    }
}

fn resource_id(id: u16) -> windows::core::PCWSTR {
    windows::core::PCWSTR::from_raw(id as usize as *const u16)
}

fn build_version_info(info: &VersionInfo) -> Result<Vec<u8>> {
    let file_version = parse_version(&info.file_version)?;
    let product_version = parse_version(&info.product_version)?;
    let strings = [
        ("FileDescription", info.file_description.as_str()),
        ("FileVersion", info.file_version.as_str()),
        ("InternalName", info.internal_name.as_str()),
        ("OriginalFilename", info.original_filename.as_str()),
        ("ProductName", info.product_name.as_str()),
        ("ProductVersion", info.product_version.as_str()),
    ]
    .into_iter()
    .chain(
        info.company_name
            .as_deref()
            .map(|value| ("CompanyName", value)),
    )
    .chain(
        info.copyright
            .as_deref()
            .map(|value| ("LegalCopyright", value)),
    )
    .collect::<Vec<_>>();
    if strings
        .iter()
        .any(|(key, value)| key.contains('\0') || value.contains('\0'))
    {
        bail!("version resource strings must not contain NUL characters");
    }

    let mut buffer = Vec::new();
    let root = begin_block(&mut buffer, 52, 0, "VS_VERSION_INFO");
    pad_to_dword(&mut buffer);
    write_fixed_file_info(&mut buffer, file_version, product_version);
    pad_to_dword(&mut buffer);

    let string_file_info = begin_block(&mut buffer, 0, 1, "StringFileInfo");
    pad_to_dword(&mut buffer);
    let string_table = begin_block(
        &mut buffer,
        0,
        1,
        &format!("{:04X}{:04X}", info.language_id, UNICODE_CODE_PAGE),
    );
    pad_to_dword(&mut buffer);
    for (key, value) in strings {
        let value_length = u16::try_from(value.encode_utf16().count())
            .context("version resource string is too long")?;
        let string = begin_block(&mut buffer, value_length, 1, key);
        pad_to_dword(&mut buffer);
        write_wstring(&mut buffer, value);
        finish_block(&mut buffer, string)?;
        pad_to_dword(&mut buffer);
    }
    finish_block(&mut buffer, string_table)?;
    pad_to_dword(&mut buffer);
    finish_block(&mut buffer, string_file_info)?;
    pad_to_dword(&mut buffer);

    let var_file_info = begin_block(&mut buffer, 0, 1, "VarFileInfo");
    pad_to_dword(&mut buffer);
    let translation = begin_block(&mut buffer, 4, 0, "Translation");
    pad_to_dword(&mut buffer);
    write_u16(&mut buffer, info.language_id);
    write_u16(&mut buffer, UNICODE_CODE_PAGE);
    finish_block(&mut buffer, translation)?;
    pad_to_dword(&mut buffer);
    finish_block(&mut buffer, var_file_info)?;
    pad_to_dword(&mut buffer);
    finish_block(&mut buffer, root)?;
    Ok(buffer)
}

/// Reads a Windows version resource, which is strictly numeric.
///
/// A project version may carry a release suffix — CalVer uses one for a second
/// release on the same day, as in `2026.9.17-r2` — but a PE version resource has
/// room for four numbers and nothing else. The suffix is dropped here rather
/// than rejected, which is the same treatment `project.file_version` documents
/// for a suffixed `project.version`.
fn parse_version(value: &str) -> Result<[u16; 4]> {
    let numeric = value.split(['-', '+']).next().unwrap_or(value).trim();
    if numeric.is_empty() {
        bail!("version must start with numeric components: {value}");
    }
    let parts: Vec<_> = numeric.split('.').collect();
    if parts.len() > 4 || parts.iter().any(|part| part.is_empty()) {
        bail!("version must contain one to four numeric components: {value}");
    }
    let mut version = [0u16; 4];
    for (index, part) in parts.into_iter().enumerate() {
        version[index] = part
            .parse::<u16>()
            .with_context(|| format!("invalid numeric version component in {value}"))?;
    }
    Ok(version)
}

pub(crate) fn validate_version(value: &str) -> Result<()> {
    parse_version(value).map(|_| ())
}

fn begin_block(buffer: &mut Vec<u8>, value_length: u16, value_type: u16, key: &str) -> usize {
    let start = buffer.len();
    write_u16(buffer, 0);
    write_u16(buffer, value_length);
    write_u16(buffer, value_type);
    write_wstring(buffer, key);
    start
}

fn finish_block(buffer: &mut [u8], start: usize) -> Result<()> {
    let length =
        u16::try_from(buffer.len() - start).context("version resource block is too long")?;
    buffer[start..start + 2].copy_from_slice(&length.to_le_bytes());
    Ok(())
}

fn write_fixed_file_info(buffer: &mut Vec<u8>, file_version: [u16; 4], product_version: [u16; 4]) {
    write_u32(buffer, 0xFEEF04BD);
    write_u32(buffer, 0x00010000);
    write_u32(
        buffer,
        ((file_version[0] as u32) << 16) | file_version[1] as u32,
    );
    write_u32(
        buffer,
        ((file_version[2] as u32) << 16) | file_version[3] as u32,
    );
    write_u32(
        buffer,
        ((product_version[0] as u32) << 16) | product_version[1] as u32,
    );
    write_u32(
        buffer,
        ((product_version[2] as u32) << 16) | product_version[3] as u32,
    );
    write_u32(buffer, 0x0000003F);
    write_u32(buffer, 0);
    write_u32(buffer, 0x00040004);
    write_u32(buffer, 1);
    write_u32(buffer, 0);
    write_u32(buffer, 0);
    write_u32(buffer, 0);
}

fn write_wstring(buffer: &mut Vec<u8>, value: &str) {
    for character in value.encode_utf16() {
        write_u16(buffer, character);
    }
    write_u16(buffer, 0);
}

fn write_u16(buffer: &mut Vec<u8>, value: u16) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(buffer: &mut Vec<u8>, value: u32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn pad_to_dword(buffer: &mut Vec<u8>) {
    while !buffer.len().is_multiple_of(4) {
        buffer.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::{build_version_info, language_id, parse_version, VersionInfo};

    #[test]
    fn parses_numeric_windows_versions() -> anyhow::Result<()> {
        assert_eq!(parse_version("1.2.3")?, [1, 2, 3, 0]);
        assert!(parse_version("1.2.beta").is_err());
        assert!(parse_version("1.2.3.4.5").is_err());
        // A PE version resource is numeric, but a CalVer version may carry the
        // suffix used for a second release on one day.
        assert_eq!(parse_version("2026.9.17-r2")?, [2026, 9, 17, 0]);
        assert_eq!(parse_version("2026.9.17+notes")?, [2026, 9, 17, 0]);
        assert!(parse_version("-r2").is_err());
        Ok(())
    }

    #[test]
    fn builds_unicode_version_resource() -> anyhow::Result<()> {
        let data = build_version_info(&VersionInfo {
            product_name: "TapTap".to_string(),
            product_version: "1.0.0".to_string(),
            file_description: "TapTap Installer".to_string(),
            file_version: "1.0.0".to_string(),
            company_name: Some("易玩（上海）网络科技有限公司".to_string()),
            copyright: Some("© 2025 TapTap".to_string()),
            internal_name: "TapTap".to_string(),
            original_filename: "TapTap_Setup.exe".to_string(),
            language_id: language_id("zh-CN"),
        })?;
        let company_utf16 = "易玩（上海）网络科技有限公司"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert!(data
            .windows(company_utf16.len())
            .any(|window| window == company_utf16));
        assert!(data.len() > 52);
        Ok(())
    }

    #[test]
    fn maps_default_locale_to_version_language() {
        assert_eq!(language_id("zh-CN"), 0x0804);
        assert_eq!(language_id("ru"), 0x0419);
        assert_eq!(language_id("unknown"), 0x0409);
    }
}
