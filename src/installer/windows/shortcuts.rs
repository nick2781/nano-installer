// Windows 快捷方式创建

use crate::common::{Error, Result};
use std::path::PathBuf;

/// 创建桌面快捷方式
pub fn create_desktop_shortcut(app_name: &str, target_path: &str) -> Result<()> {
    let desktop = get_desktop_path()?;
    let shortcut_path = desktop.join(format!("{}.lnk", app_name));

    create_shortcut(&shortcut_path, target_path, "")?;

    tracing::info!("Desktop shortcut created: {:?}", shortcut_path);

    Ok(())
}

/// 创建开始菜单快捷方式
pub fn create_start_menu_shortcut(app_name: &str, target_path: &str) -> Result<()> {
    let start_menu = get_start_menu_path()?;
    let app_folder = start_menu.join(app_name);

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
    use windows::core::{BSTR, PCWSTR, ComInterface};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::Win32::System::Com::IPersistFile;

    unsafe {
        // 初始化 COM
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)?;

        // 创建 ShellLink 对象
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;

        // 设置目标路径
        let target_path_wide: Vec<u16> = target_path.encode_utf16().chain(Some(0)).collect();
        shell_link.SetPath(PCWSTR(target_path_wide.as_ptr()))?;

        // 设置参数（如果有）
        if !arguments.is_empty() {
            let args_wide: Vec<u16> = arguments.encode_utf16().chain(Some(0)).collect();
            shell_link.SetArguments(PCWSTR(args_wide.as_ptr()))?;
        }

        // 设置工作目录
        if let Some(work_dir) = std::path::Path::new(target_path).parent() {
            if let Some(work_dir_str) = work_dir.to_str() {
                let work_dir_wide: Vec<u16> = work_dir_str.encode_utf16().chain(Some(0)).collect();
                shell_link.SetWorkingDirectory(PCWSTR(work_dir_wide.as_ptr()))?;
            }
        }

        // 保存快捷方式
        let persist_file: IPersistFile = shell_link.cast()?;
        let shortcut_path_str = shortcut_path.to_string_lossy().to_string();
        let shortcut_path_wide: Vec<u16> = shortcut_path_str.encode_utf16().chain(Some(0)).collect();
        persist_file.Save(PCWSTR(shortcut_path_wide.as_ptr()), true)?;

        // 清理 COM
        CoUninitialize();
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

/// 删除快捷方式
pub fn remove_shortcut(shortcut_path: &std::path::Path) -> Result<()> {
    if shortcut_path.exists() {
        std::fs::remove_file(shortcut_path)?;
        tracing::info!("Removed shortcut: {:?}", shortcut_path);
    }
    Ok(())
}
