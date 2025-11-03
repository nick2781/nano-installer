use crate::common::error::Error;
use std::path::Path;
use tracing::info;

/// 快捷方式创建器
pub struct ShortcutCreator {
    product_name: String,
    exe_path: String,
    exe_args: Option<String>,
    icon_path: Option<String>,
    description: Option<String>,
}

impl ShortcutCreator {
    pub fn new(
        product_name: String,
        exe_path: String,
        exe_args: Option<String>,
        icon_path: Option<String>,
        description: Option<String>,
    ) -> Self {
        Self {
            product_name,
            exe_path,
            exe_args,
            icon_path,
            description,
        }
    }

    /// 创建桌面快捷方式
    pub fn create_desktop_shortcut(&self) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let desktop_path = self.get_desktop_path()?;
            let shortcut_path = desktop_path.join(format!("{}.lnk", self.product_name));
            
            self.create_shortcut(&shortcut_path)?;
            info!("Created desktop shortcut: {:?}", shortcut_path);
        }
        #[cfg(not(windows))]
        {
            info!("Shortcut creation not supported on non-Windows platforms");
        }
        Ok(())
    }

    /// 创建开始菜单快捷方式
    pub fn create_start_menu_shortcut(&self) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let start_menu_path = self.get_start_menu_path()?;
            let product_folder = start_menu_path.join(&self.product_name);
            
            // 创建产品文件夹
            std::fs::create_dir_all(&product_folder)
                .map_err(|e| Error::InstallationFailed(format!("Failed to create start menu folder: {}", e)))?;
            
            let shortcut_path = product_folder.join(format!("{}.lnk", self.product_name));
            self.create_shortcut(&shortcut_path)?;
            info!("Created start menu shortcut: {:?}", shortcut_path);
        }
        #[cfg(not(windows))]
        {
            warn!("Shortcut creation not supported on non-Windows platforms");
        }
        Ok(())
    }

    /// 删除桌面快捷方式
    pub fn remove_desktop_shortcut(&self) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let desktop_path = self.get_desktop_path()?;
            let shortcut_path = desktop_path.join(format!("{}.lnk", self.product_name));
            
            if shortcut_path.exists() {
                std::fs::remove_file(&shortcut_path)
                    .map_err(|e| Error::UninstallationFailed(format!("Failed to remove desktop shortcut: {}", e)))?;
                info!("Removed desktop shortcut: {:?}", shortcut_path);
            }
        }
        #[cfg(not(windows))]
        {
            warn!("Shortcut removal not supported on non-Windows platforms");
        }
        Ok(())
    }

    /// 删除开始菜单快捷方式
    pub fn remove_start_menu_shortcut(&self) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let start_menu_path = self.get_start_menu_path()?;
            let product_folder = start_menu_path.join(&self.product_name);
            
            if product_folder.exists() {
                std::fs::remove_dir_all(&product_folder)
                    .map_err(|e| Error::UninstallationFailed(format!("Failed to remove start menu folder: {}", e)))?;
                info!("Removed start menu folder: {:?}", product_folder);
            }
        }
        #[cfg(not(windows))]
        {
            warn!("Shortcut removal not supported on non-Windows platforms");
        }
        Ok(())
    }

    #[cfg(windows)]
    fn get_desktop_path(&self) -> Result<std::path::PathBuf, Error> {
        // 使用环境变量获取桌面路径
        if let Ok(desktop) = std::env::var("USERPROFILE") {
            Ok(std::path::PathBuf::from(desktop).join("Desktop"))
        } else {
            Err(Error::InstallationFailed("Failed to get desktop path".to_string()))
        }
    }

    #[cfg(windows)]
    fn get_start_menu_path(&self) -> Result<std::path::PathBuf, Error> {
        // 使用环境变量获取开始菜单路径
        if let Ok(program_data) = std::env::var("PROGRAMDATA") {
            Ok(std::path::PathBuf::from(program_data).join("Microsoft").join("Windows").join("Start Menu").join("Programs"))
        } else {
            Err(Error::InstallationFailed("Failed to get start menu path".to_string()))
        }
    }

    #[cfg(windows)]
    fn create_shortcut(&self, shortcut_path: &Path) -> Result<(), Error> {
        // 简化的快捷方式创建 - 使用PowerShell
        let ps_script = format!(
            r#"
            $WshShell = New-Object -comObject WScript.Shell
            $Shortcut = $WshShell.CreateShortcut("{}")
            $Shortcut.TargetPath = "{}"
            $Shortcut.WorkingDirectory = "{}"
            $Shortcut.Save()
            "#,
            shortcut_path.display(),
            self.exe_path,
            std::path::Path::new(&self.exe_path).parent().unwrap_or(std::path::Path::new("C:\\")).display()
        );

        let output = std::process::Command::new("powershell")
            .arg("-Command")
            .arg(&ps_script)
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    Ok(())
                } else {
                    Err(Error::InstallationFailed(format!("Failed to create shortcut: {}", String::from_utf8_lossy(&result.stderr))))
                }
            }
            Err(e) => Err(Error::InstallationFailed(format!("Failed to run PowerShell: {}", e))),
        }
    }
}
