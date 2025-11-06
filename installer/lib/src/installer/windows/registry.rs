// Windows 注册表操作

use crate::common::{Error, Result};
use winreg::enums::*;
use winreg::RegKey;

/// 写入卸载条目到注册表
pub fn write_uninstall_entry(
    app_name: &str,
    version: &str,
    install_path: &str,
    publisher: &str,
    uninstaller_path: &str,
) -> Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let uninstall_key = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

    let (key, _) = hklm
        .create_subkey(format!(r"{}\{}", uninstall_key, app_name))
        .map_err(|e| Error::Registry(format!("Failed to create uninstall key: {}", e)))?;

    // 写入基本信息
    key.set_value("DisplayName", &app_name)
        .map_err(|e| Error::Registry(format!("Failed to set DisplayName: {}", e)))?;

    key.set_value("DisplayVersion", &version)
        .map_err(|e| Error::Registry(format!("Failed to set DisplayVersion: {}", e)))?;

    key.set_value("Publisher", &publisher)
        .map_err(|e| Error::Registry(format!("Failed to set Publisher: {}", e)))?;

    key.set_value("InstallLocation", &install_path)
        .map_err(|e| Error::Registry(format!("Failed to set InstallLocation: {}", e)))?;

    key.set_value("UninstallString", &uninstaller_path)
        .map_err(|e| Error::Registry(format!("Failed to set UninstallString: {}", e)))?;

    // 可选：估算大小（KB）
    let install_size = estimate_dir_size(install_path);
    key.set_value("EstimatedSize", &(install_size / 1024))
        .map_err(|e| Error::Registry(format!("Failed to set EstimatedSize: {}", e)))?;

    // 显示图标
    let icon_path = format!("{},0", uninstaller_path);
    key.set_value("DisplayIcon", &icon_path)
        .map_err(|e| Error::Registry(format!("Failed to set DisplayIcon: {}", e)))?;

    // 无修改按钮（只有卸载）
    key.set_value("NoModify", &1u32)
        .map_err(|e| Error::Registry(format!("Failed to set NoModify: {}", e)))?;

    key.set_value("NoRepair", &1u32)
        .map_err(|e| Error::Registry(format!("Failed to set NoRepair: {}", e)))?;

    tracing::info!("Uninstall entry created in registry");

    Ok(())
}

/// 删除卸载条目
pub fn remove_uninstall_entry(app_name: &str) -> Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let uninstall_key = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

    let key = hklm
        .open_subkey_with_flags(uninstall_key, KEY_WRITE)
        .map_err(|e| Error::Registry(format!("Failed to open uninstall key: {}", e)))?;

    key.delete_subkey_all(app_name)
        .map_err(|e| Error::Registry(format!("Failed to delete uninstall key: {}", e)))?;

    tracing::info!("Uninstall entry removed from registry");

    Ok(())
}

/// 估算目录大小（字节）
fn estimate_dir_size(path: &str) -> u32 {
    use std::fs;

    let mut size = 0u64;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    size += metadata.len();
                } else if metadata.is_dir() {
                    if let Some(dir_path) = entry.path().to_str() {
                        size += estimate_dir_size(dir_path) as u64;
                    }
                }
            }
        }
    }

    size.min(u32::MAX as u64) as u32
}
