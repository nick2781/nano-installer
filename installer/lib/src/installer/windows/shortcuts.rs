// Windows 快捷方式创建

use crate::common::{Error, Result};
use std::path::PathBuf;

/// 创建桌面快捷方式
pub fn create_desktop_shortcut(app_name: &str, target_path: &str) -> Result<()> {
    let shortcut_path = get_desktop_shortcut_path(app_name)?;

    create_shortcut(&shortcut_path, target_path, "")?;

    tracing::info!("Desktop shortcut created: {:?}", shortcut_path);

    Ok(())
}

/// 创建开始菜单快捷方式
pub fn create_start_menu_shortcut(
    app_name: &str,
    start_menu_folder: &str,
    target_path: &str,
) -> Result<()> {
    let app_folder = get_start_menu_folder_path(start_menu_folder)?;

    std::fs::create_dir_all(&app_folder)?;

    let shortcut_path = app_folder.join(format!("{}.lnk", app_name));

    create_shortcut(&shortcut_path, target_path, "")?;

    tracing::info!("Start menu shortcut created: {:?}", shortcut_path);

    Ok(())
}

/// 创建快捷方式（使用 COM API）
#[cfg(windows)]
fn create_shortcut(
    shortcut_path: &std::path::Path,
    target_path: &str,
    arguments: &str,
) -> Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let shortcut_path = shortcut_path.to_string_lossy().replace('\'', "''");
    let target_path = target_path.replace('\'', "''");
    let arguments = arguments.replace('\'', "''");
    let working_directory = std::path::Path::new(target_path.as_str())
        .parent()
        .map(|path| path.to_string_lossy().replace('\'', "''"))
        .unwrap_or_default();

    let script = format!(
        "$ws = New-Object -ComObject WScript.Shell; \
         $s = $ws.CreateShortcut('{shortcut}'); \
         $s.TargetPath = '{target}'; \
         $s.Arguments = '{arguments}'; \
         $s.WorkingDirectory = '{working_dir}'; \
         $s.Save()",
        shortcut = shortcut_path,
        target = target_path,
        arguments = arguments,
        working_dir = working_directory,
    );

    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| {
            Error::Io(std::io::Error::other(format!(
                "Failed to launch PowerShell: {}",
                e
            )))
        })?;

    if !output.status.success() {
        return Err(Error::Unknown(format!(
            "Failed to create shortcut {}: {}",
            shortcut_path,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(())
}

#[cfg(not(windows))]
fn create_shortcut(
    _shortcut_path: &std::path::Path,
    _target_path: &str,
    _arguments: &str,
) -> Result<()> {
    Ok(())
}

/// 获取桌面路径
fn get_desktop_path() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Foundation::MAX_PATH;
        use windows::Win32::UI::Shell::{SHGetFolderPathW, CSIDL_DESKTOPDIRECTORY};

        unsafe {
            let mut path = [0u16; MAX_PATH as usize];
            SHGetFolderPathW(
                HWND::default(),
                CSIDL_DESKTOPDIRECTORY as i32,
                None,
                0,
                &mut path,
            )?;

            let len = path.iter().position(|&c| c == 0).unwrap_or(path.len());
            let path_str = String::from_utf16_lossy(&path[..len]);
            Ok(PathBuf::from(path_str))
        }
    }

    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME")?;
        Ok(PathBuf::from(format!("{}/Desktop", home)))
    }
}

/// 获取桌面快捷方式完整路径
pub fn get_desktop_shortcut_path(app_name: &str) -> Result<PathBuf> {
    Ok(get_desktop_path()?.join(format!("{}.lnk", app_name)))
}

/// 获取开始菜单路径
fn get_start_menu_path() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Foundation::MAX_PATH;
        use windows::Win32::UI::Shell::{SHGetFolderPathW, CSIDL_PROGRAMS};

        unsafe {
            let mut path = [0u16; MAX_PATH as usize];
            SHGetFolderPathW(HWND::default(), CSIDL_PROGRAMS as i32, None, 0, &mut path)?;

            let len = path.iter().position(|&c| c == 0).unwrap_or(path.len());
            let path_str = String::from_utf16_lossy(&path[..len]);
            Ok(PathBuf::from(path_str))
        }
    }

    #[cfg(not(windows))]
    {
        Ok(PathBuf::from("/usr/share/applications"))
    }
}

/// 获取开始菜单文件夹路径
pub fn get_start_menu_folder_path(folder_name: &str) -> Result<PathBuf> {
    Ok(get_start_menu_path()?.join(folder_name))
}

/// 获取开始菜单快捷方式完整路径
pub fn get_start_menu_shortcut_path(app_name: &str, folder_name: &str) -> Result<PathBuf> {
    Ok(get_start_menu_folder_path(folder_name)?.join(format!("{}.lnk", app_name)))
}

/// 删除快捷方式
pub fn remove_shortcut(shortcut_path: &std::path::Path) -> Result<()> {
    if shortcut_path.exists() {
        std::fs::remove_file(shortcut_path)?;
        tracing::info!("Removed shortcut: {:?}", shortcut_path);
    }
    Ok(())
}
