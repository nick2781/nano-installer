use crate::common::error::Error;
use tracing::info;

#[cfg(windows)]
use winreg::RegKey;

/// 注册表操作
pub struct RegistryOps {
    product_name: String,
    publisher: String,
    version: String,
    install_path: String,
    exe_name: String,
    help_link: Option<String>,
}

impl RegistryOps {
    pub fn new(
        product_name: String,
        publisher: String,
        version: String,
        install_path: String,
        exe_name: String,
        help_link: Option<String>,
    ) -> Self {
        Self {
            product_name,
            publisher,
            version,
            install_path,
            exe_name,
            help_link,
        }
    }

    /// 写入安装路径到注册表
    pub fn write_install_path(&self) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            let key_path = format!("Software\\{}", self.product_name);
            let (key, _) = hklm.create_subkey(&key_path)
                .map_err(|e| Error::Registry(format!("Failed to create key: {}", e)))?;
            
            key.set_value("InstallPath", &self.install_path)
                .map_err(|e| Error::Registry(format!("Failed to write install path: {}", e)))?;
            
            info!("Written install path to registry: {}", self.install_path);
        }
        #[cfg(not(windows))]
        {
            info!("Registry operations not supported on non-Windows platforms");
        }
        Ok(())
    }

    /// 写入卸载信息到注册表
    pub fn write_uninstall_info(&self) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            let key_path = format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", self.product_name);
            let (key, _) = hklm.create_subkey(&key_path)
                .map_err(|e| Error::Registry(format!("Failed to create uninstall key: {}", e)))?;
            
            key.set_value("DisplayName", &self.product_name)?;
            key.set_value("DisplayVersion", &self.version)?;
            key.set_value("Publisher", &self.publisher)?;
            key.set_value("InstallLocation", &self.install_path)?;
            
            let uninstall_string = format!("\"{}\\uninst.exe\"", self.install_path);
            key.set_value("UninstallString", &uninstall_string)?;
            
            let display_icon = format!("\"{}\\{}.exe\"", self.install_path, self.exe_name);
            key.set_value("DisplayIcon", &display_icon)?;
            
            if let Some(help_link) = &self.help_link {
                key.set_value("HelpLink", help_link)?;
            }
            
            key.set_value("EstimatedSize", &100u32)?;
            
            info!("Written uninstall info to registry for: {}", self.product_name);
        }
        #[cfg(not(windows))]
        {
            info!("Registry operations not supported on non-Windows platforms");
        }
        Ok(())
    }

    /// 设置开机自启
    pub fn set_autostart(&self, enabled: bool) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
            let (key, _) = hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
                .map_err(|e| Error::Registry(format!("Failed to open Run key: {}", e)))?;
            
            if enabled {
                let exe_path = format!("\"{}\\{}.exe\"", self.install_path, self.exe_name);
                key.set_value(&self.product_name, &exe_path)
                    .map_err(|e| Error::Registry(format!("Failed to set autostart: {}", e)))?;
                info!("Set autostart enabled for: {}", self.product_name);
            } else {
                let _ = key.delete_value(&self.product_name);
                info!("Set autostart disabled for: {}", self.product_name);
            }
        }
        #[cfg(not(windows))]
        {
            info!("Registry operations not supported on non-Windows platforms");
        }
        Ok(())
    }

    /// 读取安装路径
    pub fn read_install_path(&self) -> Result<Option<String>, Error> {
        #[cfg(windows)]
        {
            let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            let key_path = format!("Software\\{}", self.product_name);
            
            match hklm.open_subkey(&key_path) {
                Ok(key) => {
                    match key.get_value::<String, _>("InstallPath") {
                        Ok(path) => Ok(Some(path)),
                        Err(_) => Ok(None),
                    }
                }
                Err(_) => Ok(None),
            }
        }
        #[cfg(not(windows))]
        {
            Ok(None)
        }
    }

    /// 删除安装路径键
    pub fn delete_install_path(&self) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            let key_path = format!("Software\\{}", self.product_name);
            let _ = hklm.delete_subkey_all(&key_path);
            info!("Deleted install path key: {}", key_path);
        }
        #[cfg(not(windows))]
        {
            info!("Registry operations not supported on non-Windows platforms");
        }
        Ok(())
    }

    /// 删除卸载信息
    pub fn delete_uninstall_info(&self) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            let key_path = format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", self.product_name);
            let _ = hklm.delete_subkey_all(&key_path);
            info!("Deleted uninstall info key: {}", key_path);
        }
        #[cfg(not(windows))]
        {
            info!("Registry operations not supported on non-Windows platforms");
        }
        Ok(())
    }

    /// 删除开机自启
    pub fn delete_autostart(&self) -> Result<(), Error> {
        #[cfg(windows)]
        {
            let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
            if let Ok((key, _)) = hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run") {
                let _ = key.delete_value(&self.product_name);
                info!("Deleted autostart for: {}", self.product_name);
            }
        }
        #[cfg(not(windows))]
        {
            info!("Registry operations not supported on non-Windows platforms");
        }
        Ok(())
    }
}