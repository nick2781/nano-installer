// 平台相关功能

use crate::common::Result;

/// Return the effective system DPI, falling back to the Windows baseline.
///
/// GDI is used here because these APIs predate Windows 7. Keeping DPI detection
/// in one place also prevents runtime stubs from importing newer DPI APIs.
pub fn system_dpi() -> u32 {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, ReleaseDC, LOGPIXELSX};

        let desktop = HWND::default();
        let device_context = GetDC(desktop);
        if device_context.0.is_null() {
            return 96;
        }

        let dpi = GetDeviceCaps(device_context, LOGPIXELSX);
        ReleaseDC(desktop, device_context);

        if dpi > 0 {
            dpi as u32
        } else {
            96
        }
    }

    #[cfg(not(windows))]
    {
        96
    }
}

/// 检测系统语言
pub fn detect_system_locale() -> String {
    #[cfg(windows)]
    {
        detect_windows_locale()
    }

    #[cfg(not(windows))]
    {
        "en-US".to_string()
    }
}

#[cfg(windows)]
fn detect_windows_locale() -> String {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    use windows::Win32::System::SystemServices::LOCALE_NAME_MAX_LENGTH;

    unsafe {
        let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH as usize];
        let len = GetUserDefaultLocaleName(&mut buffer);

        if len > 0 {
            let locale = String::from_utf16_lossy(&buffer[..len as usize - 1]);

            // 映射到支持的语言
            match locale.as_str() {
                s if s.starts_with("en") => "en-US".to_string(),
                s if s.starts_with("zh-CN") || s.starts_with("zh-Hans") => "zh-CN".to_string(),
                s if s.starts_with("zh-TW") || s.starts_with("zh-Hant") => "zh-TW".to_string(),
                s if s.starts_with("ja") => "ja".to_string(),
                s if s.starts_with("vi") => "vi".to_string(),
                _ => "en-US".to_string(), // 默认回退到英语
            }
        } else {
            "en-US".to_string()
        }
    }
}

/// 检查是否以管理员权限运行
#[cfg(windows)]
pub fn is_elevated() -> Result<bool> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = HANDLE::default();
        let current_process = GetCurrentProcess();

        OpenProcessToken(current_process, TOKEN_QUERY, &mut token)?;

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut return_length = 0u32;

        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );

        CloseHandle(token)?;

        result?;
        Ok(elevation.TokenIsElevated != 0)
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> Result<bool> {
    Ok(false)
}

/// 请求管理员权限重启
#[cfg(windows)]
pub fn request_elevation(args: &[String]) -> Result<()> {
    use windows::core::w;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy().to_string();
    // Skip args[0] (exe path) — ShellExecuteW takes file and params separately
    let args_str = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        String::new()
    };

    unsafe {
        let verb = w!("runas");
        let file = exe_path_str
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<u16>>();
        let params = args_str.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();

        ShellExecuteW(
            HWND::default(),
            PWSTR(verb.as_ptr() as *mut u16),
            PWSTR(file.as_ptr() as *mut u16),
            PWSTR(params.as_ptr() as *mut u16),
            PWSTR::null(),
            SW_SHOWNORMAL,
        );
    }

    std::process::exit(0);
}

#[cfg(not(windows))]
pub fn request_elevation(_args: &[String]) -> Result<()> {
    Ok(())
}

/// 获取程序文件目录
pub fn get_program_files_dir() -> String {
    #[cfg(windows)]
    {
        std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string())
    }

    #[cfg(not(windows))]
    {
        "/opt".to_string()
    }
}

/// 获取用户数据目录
pub fn get_user_data_dir(app_name: &str) -> Result<String> {
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA")
            .unwrap_or_else(|_| r"C:\Users\Public\AppData\Roaming".to_string());
        Ok(format!(r"{}\{}", appdata, app_name))
    }

    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME")?;
        Ok(format!("{}/Library/Application Support/{}", home, app_name))
    }
}
