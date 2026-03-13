// Windows 注册表操作

use crate::common::{Error, Result};
use winreg::enums::*;
use winreg::RegKey;

fn split_registry_path(path: &str) -> Result<(RegKey, String)> {
    let normalized = path.replace('/', "\\");
    let Some((hive, subkey)) = normalized.split_once('\\') else {
        return Err(Error::Registry(format!("Invalid registry path: {}", path)));
    };

    let root = match hive.to_uppercase().as_str() {
        "HKLM" | "HKEY_LOCAL_MACHINE" => RegKey::predef(HKEY_LOCAL_MACHINE),
        "HKCU" | "HKEY_CURRENT_USER" => RegKey::predef(HKEY_CURRENT_USER),
        "HKCR" | "HKEY_CLASSES_ROOT" => RegKey::predef(HKEY_CLASSES_ROOT),
        "HKU" | "HKEY_USERS" => RegKey::predef(HKEY_USERS),
        "HKCC" | "HKEY_CURRENT_CONFIG" => RegKey::predef(HKEY_CURRENT_CONFIG),
        other => {
            return Err(Error::Registry(format!(
                "Unsupported registry hive: {}",
                other
            )))
        }
    };

    Ok((root, subkey.to_string()))
}

pub fn create_key(path: &str) -> Result<RegKey> {
    let (root, subkey) = split_registry_path(path)?;
    let (key, _) = root
        .create_subkey(&subkey)
        .map_err(|e| Error::Registry(format!("Failed to create registry key {}: {}", path, e)))?;
    Ok(key)
}

pub fn set_string_value(path: &str, value_name: &str, value: &str) -> Result<()> {
    let key = create_key(path)?;
    key.set_value(value_name, &value).map_err(|e| {
        Error::Registry(format!(
            "Failed to set registry value {}\\{}: {}",
            path, value_name, e
        ))
    })?;
    Ok(())
}

pub fn set_u32_value(path: &str, value_name: &str, value: u32) -> Result<()> {
    let key = create_key(path)?;
    key.set_value(value_name, &value).map_err(|e| {
        Error::Registry(format!(
            "Failed to set registry value {}\\{}: {}",
            path, value_name, e
        ))
    })?;
    Ok(())
}

pub fn get_string_value(path: &str, value_name: &str) -> Result<Option<String>> {
    let (root, subkey) = split_registry_path(path)?;
    let key = match root.open_subkey(&subkey) {
        Ok(key) => key,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(Error::Registry(format!(
                "Failed to open registry key {}: {}",
                path, e
            )))
        }
    };

    match key.get_value::<String, _>(value_name) {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Registry(format!(
            "Failed to read registry value {}\\{}: {}",
            path, value_name, e
        ))),
    }
}

pub fn delete_key(path: &str) -> Result<()> {
    let (root, subkey) = split_registry_path(path)?;
    root.delete_subkey_all(&subkey)
        .map_err(|e| Error::Registry(format!("Failed to delete registry key {}: {}", path, e)))?;
    Ok(())
}

pub fn delete_value(path: &str, value_name: &str) -> Result<()> {
    let (root, subkey) = split_registry_path(path)?;
    let key = root
        .open_subkey_with_flags(&subkey, KEY_WRITE)
        .map_err(|e| Error::Registry(format!("Failed to open registry key {}: {}", path, e)))?;
    key.delete_value(value_name).map_err(|e| {
        Error::Registry(format!(
            "Failed to delete registry value {}\\{}: {}",
            path, value_name, e
        ))
    })?;
    Ok(())
}

/// 写入卸载条目到注册表
pub fn write_uninstall_entry(
    uninstall_key_path: &str,
    app_name: &str,
    version: &str,
    install_path: &str,
    publisher: &str,
    uninstaller_path: &str,
) -> Result<()> {
    let key = create_key(uninstall_key_path)?;

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
    let uninstall_key = format!(
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\{}",
        app_name
    );
    delete_key(&uninstall_key)?;

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
