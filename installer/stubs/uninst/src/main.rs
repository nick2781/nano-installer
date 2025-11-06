// 轻量级卸载器 stub - 只使用 Win32 API，不依赖 Egui
// 预期大小：~200-300 KB

#![windows_subsystem = "windows"]

use std::path::PathBuf;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        UI::WindowsAndMessaging::*,
        System::Registry::*,
    },
};
use anyhow::{Result, Context};

fn main() -> Result<()> {
    // 1. 从注册表读取卸载信息
    let uninstall_info = match read_uninstall_info() {
        Ok(info) => info,
        Err(e) => {
            show_error(&format!("Failed to read uninstall information: {}", e));
            return Ok(());
        }
    };

    // 2. 显示确认对话框
    let message = format!(
        "Are you sure you want to uninstall {}?\n\nInstallation directory:\n{}",
        uninstall_info.app_name,
        uninstall_info.install_path
    );

    let result = unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(&message),
            &HSTRING::from("Uninstall"),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        )
    };

    if result != IDYES {
        return Ok(());
    }

    // 3. 执行卸载
    if let Err(e) = perform_uninstall(&uninstall_info) {
        show_error(&format!("Uninstallation failed: {}", e));
        return Ok(());
    }

    // 4. 显示完成消息
    let message = format!("{} has been successfully uninstalled.", uninstall_info.app_name);
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(&message),
            &HSTRING::from("Uninstall Complete"),
            MB_OK | MB_ICONINFORMATION,
        )
    };

    Ok(())
}

#[derive(Debug)]
struct UninstallInfo {
    app_name: String,
    install_path: String,
    registry_key: String,
}

fn read_uninstall_info() -> Result<UninstallInfo> {
    // 尝试从当前目录的配置文件读取，或者从注册表读取
    let exe_path = std::env::current_exe()?;
    let install_dir = exe_path.parent().ok_or_else(|| Error::from_win32())?;
    
    // 尝试读取 uninstall.json
    let config_path = install_dir.join("uninstall.json");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .context("Failed to parse uninstall.json")?;
        
        return Ok(UninstallInfo {
            app_name: json["app_name"].as_str().unwrap_or("Application").to_string(),
            install_path: install_dir.to_string_lossy().to_string(),
            registry_key: json["registry_key"].as_str().unwrap_or("").to_string(),
        });
    }

    // 回退：假设是从安装目录运行
    Ok(UninstallInfo {
        app_name: "Application".to_string(),
        install_path: install_dir.to_string_lossy().to_string(),
        registry_key: String::new(),
    })
}

fn perform_uninstall(info: &UninstallInfo) -> Result<()> {
    let install_path = PathBuf::from(&info.install_path);

    // 1. 删除桌面快捷方式
    remove_shortcuts(&info.app_name)?;

    // 2. 删除注册表项
    if !info.registry_key.is_empty() {
        remove_registry_entries(&info.registry_key)?;
    }

    // 3. 删除安装目录（将当前 exe 标记为待删除）
    remove_installation_directory(&install_path)?;

    Ok(())
}

fn remove_shortcuts(app_name: &str) -> Result<()> {
    // 删除桌面快捷方式
    if let Ok(desktop_path) = std::env::var("USERPROFILE") {
        let desktop = PathBuf::from(desktop_path).join("Desktop");
        let shortcut = desktop.join(format!("{}.lnk", app_name));
        let _ = std::fs::remove_file(shortcut); // 忽略错误
    }

    // 删除开始菜单快捷方式
    if let Ok(start_menu) = std::env::var("APPDATA") {
        let programs = PathBuf::from(start_menu).join("Microsoft\\Windows\\Start Menu\\Programs");
        let shortcut = programs.join(format!("{}.lnk", app_name));
        let _ = std::fs::remove_file(shortcut); // 忽略错误
    }

    Ok(())
}

fn remove_registry_entries(registry_key: &str) -> Result<()> {
    unsafe {
        // 删除 HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\{AppName}
        let uninstall_key = HSTRING::from(format!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}",
            registry_key
        ));
        
        let _ = RegDeleteTreeW(
            HKEY_LOCAL_MACHINE,
            &uninstall_key,
        );

        // 删除应用程序自己的注册表项
        let app_key = HSTRING::from(format!("Software\\{}", registry_key));
        let _ = RegDeleteTreeW(
            HKEY_LOCAL_MACHINE,
            &app_key,
        );
    }

    Ok(())
}

fn remove_installation_directory(install_path: &PathBuf) -> Result<()> {
    let current_exe = std::env::current_exe()?;

    // 遍历删除所有文件（除了当前 exe）
    remove_dir_contents(install_path, &current_exe)?;

    // 创建批处理脚本来删除自己和目录
    let batch_path = std::env::temp_dir().join("uninstall_cleanup.bat");
    let batch_content = format!(
        "@echo off\r\n\
         :retry\r\n\
         timeout /t 1 /nobreak >nul\r\n\
         del /f /q \"{}\"\r\n\
         if exist \"{}\" goto retry\r\n\
         rmdir /s /q \"{}\"\r\n\
         del /f /q \"%~f0\"\r\n",
        current_exe.display(),
        current_exe.display(),
        install_path.display()
    );

    std::fs::write(&batch_path, batch_content)?;

    // 启动批处理脚本
    unsafe {
        use windows::Win32::System::Threading::*;
        
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTF_USESHOWWINDOW;
        si.wShowWindow = SW_HIDE.0 as u16;

        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
        
        let cmd = HSTRING::from(format!("cmd.exe /c \"{}\"", batch_path.display()));
        
        let mut cmd_line = PWSTR(cmd.as_ptr() as *mut u16);
        
        CreateProcessW(
            None,
            cmd_line,
            None,
            None,
            false,
            CREATE_NO_WINDOW,
            None,
            None,
            &si,
            &mut pi,
        )?;

        CloseHandle(pi.hProcess)?;
        CloseHandle(pi.hThread)?;
    }

    Ok(())
}

fn remove_dir_contents(dir: &PathBuf, exclude_file: &PathBuf) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path == *exclude_file {
            continue; // 跳过当前执行的 exe
        }

        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path); // 递归删除子目录
        } else {
            let _ = std::fs::remove_file(&path); // 删除文件
        }
    }

    Ok(())
}

fn show_error(message: &str) {
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(message),
            &HSTRING::from("Uninstall Error"),
            MB_OK | MB_ICONERROR,
        )
    };
}

