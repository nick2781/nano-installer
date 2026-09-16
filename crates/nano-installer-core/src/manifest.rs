//! Application manifest the builder injects into a setup and its uninstaller.
//!
//! Windows reads this manifest before the process starts, so it is the only way
//! to ask for administrator rights and to tell the shell the window scales its
//! own pixels. Both settings come from the project configuration; anything the
//! project does not ask for keeps the Windows default behaviour.
//!
//! DPI awareness is declared twice on purpose. `dpiAware` is the setting
//! Windows 7, 8, and 8.1 read, where the only choice is whether the whole
//! process scales with the primary display. `dpiAwareness` is the setting
//! Windows 10 1607 and later prefer, where a window can follow the scaling of
//! the display it is actually on. Windows 7 ignores the newer element because
//! it is in a namespace that did not exist yet.

use anyhow::{Context, Result};
use std::path::Path;

/// Resource type 24 is `RT_MANIFEST`, and ID 1 is the manifest of an
/// executable rather than of a side-by-side assembly.
const RT_MANIFEST: u16 = 24;
const MANIFEST_RESOURCE_ID: u16 = 1;

/// Everything the manifest can ask Windows for on behalf of a generated binary.
pub struct ManifestSettings {
    /// Ask for administrator rights before the process starts.
    pub require_admin: bool,
    /// Scale the user interface with the display DPI.
    pub dpi_aware: bool,
}

impl ManifestSettings {
    /// Reads the two settings a project can declare.
    ///
    /// `dpi_aware` defaults to `true`, which matches the runtime default, and
    /// `require_admin` defaults to `false`, so a project that says nothing gets
    /// the behaviour of an ordinary executable: no prompt, and no rescaled
    /// window.
    pub fn from_config(config: &serde_json::Value) -> Self {
        Self {
            require_admin: config["install"]["require_admin"].as_bool() == Some(true),
            dpi_aware: config["ui"]["dpi_aware"].as_bool().unwrap_or(true),
        }
    }

    fn xml(&self) -> String {
        let level = if self.require_admin {
            "requireAdministrator"
        } else {
            "asInvoker"
        };
        let dpi_aware = if self.dpi_aware { "true" } else { "false" };
        // Per-monitor and PerMonitorV2 are declared as a list: Windows 10 1607
        // through 1703 understand PerMonitor, and 1703 and later take the
        // first entry they know, so this asks for the sharpest behaviour each
        // supported release can deliver. A project that turns scaling off asks
        // for it by name so a later release cannot silently scale it anyway.
        let dpi_awareness = if self.dpi_aware {
            "PerMonitorV2, PerMonitor"
        } else {
            "unaware"
        };
        // Built as lines rather than as one continued literal: a Rust string
        // continuation eats the indentation of the next line, which would run
        // two attributes together and produce a manifest Windows rejects.
        [
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#.to_string(),
            r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">"#
                .to_string(),
            r#"  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">"#.to_string(),
            r#"    <security>"#.to_string(),
            r#"      <requestedPrivileges>"#.to_string(),
            format!(r#"        <requestedExecutionLevel level="{level}" uiAccess="false"/>"#),
            r#"      </requestedPrivileges>"#.to_string(),
            r#"    </security>"#.to_string(),
            r#"  </trustInfo>"#.to_string(),
            r#"  <dependency>"#.to_string(),
            r#"    <dependentAssembly>"#.to_string(),
            r#"      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>"#
                .to_string(),
            r#"    </dependentAssembly>"#.to_string(),
            r#"  </dependency>"#.to_string(),
            r#"  <application xmlns="urn:schemas-microsoft-com:asm.v3">"#.to_string(),
            r#"    <windowsSettings>"#.to_string(),
            format!(
                r#"      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">{dpi_aware}</dpiAware>"#
            ),
            format!(
                r#"      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">{dpi_awareness}</dpiAwareness>"#
            ),
            r#"    </windowsSettings>"#.to_string(),
            r#"  </application>"#.to_string(),
            r#"</assembly>"#.to_string(),
            String::new(),
        ]
        .join("\n")
    }
}

/// Writes the manifest into `exe_path` as the executable manifest resource.
///
/// The resource is written in the neutral language, which is where Windows
/// looks for the manifest of an image no matter which locale the machine runs.
pub fn replace_exe_manifest(exe_path: &Path, settings: &ManifestSettings) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::LibraryLoader::{
        BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW,
    };

    let data = settings.xml().into_bytes();
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
            resource_id(RT_MANIFEST),
            resource_id(MANIFEST_RESOURCE_ID),
            0,
            Some(data.as_ptr().cast()),
            data.len() as u32,
        )
        .context("failed to write the application manifest");
        if let Err(error) = update_result {
            let _ = EndUpdateResourceW(handle, true);
            return Err(error);
        }
        EndUpdateResourceW(handle, false)
            .with_context(|| format!("failed to commit PE resources: {}", exe_path.display()))?;
    }
    Ok(())
}

/// A resource type or name carried in the low word of a `PCWSTR`.
fn resource_id(id: u16) -> windows::core::PCWSTR {
    windows::core::PCWSTR::from_raw(id as usize as *const u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the manifest resource back out of a PE image, the way Windows does
    /// when it starts the process, so a test proves the bytes really landed.
    fn manifest_in_image(path: &Path) -> Option<String> {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::System::LibraryLoader::{
            FindResourceW, LoadLibraryExW, LoadResource, LockResource, SizeofResource,
            LOAD_LIBRARY_AS_DATAFILE,
        };

        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            let module = LoadLibraryExW(
                PCWSTR::from_raw(wide.as_ptr()),
                None,
                LOAD_LIBRARY_AS_DATAFILE,
            )
            .ok()?;
            let resource = FindResourceW(
                module,
                resource_id(MANIFEST_RESOURCE_ID),
                resource_id(RT_MANIFEST),
            );
            if resource.is_invalid() {
                return None;
            }
            let size = SizeofResource(module, resource);
            let handle = LoadResource(module, resource).ok()?;
            let pointer = LockResource(handle);
            if pointer.is_null() || size == 0 {
                return None;
            }
            let bytes = std::slice::from_raw_parts(pointer.cast::<u8>(), size as usize);
            Some(String::from_utf8_lossy(bytes).into_owned())
        }
    }

    #[test]
    fn the_manifest_reaches_a_real_executable() -> anyhow::Result<()> {
        // The test binary is a real PE image, so it stands in for a stub: the
        // resource has to be readable back exactly as Windows would read it.
        let directory = tempfile::tempdir()?;
        let image = directory.path().join("manifest-probe.exe");
        std::fs::copy(std::env::current_exe()?, &image)?;
        assert!(manifest_in_image(&image).is_none());

        let settings = ManifestSettings {
            require_admin: true,
            dpi_aware: false,
        };
        replace_exe_manifest(&image, &settings)?;

        let written = manifest_in_image(&image).expect("manifest resource is missing");
        assert!(written.contains("requireAdministrator"));
        assert!(written.contains(">false</dpiAware>"));
        // Turning scaling off has to say so in the newer element too, or
        // Windows 10 would scale the window anyway.
        assert!(written.contains(">unaware</dpiAwareness>"));
        // The image must stay loadable: Windows rejects an executable whose
        // manifest is malformed rather than starting it.
        assert!(roxmltree::Document::parse(&written).is_ok());
        Ok(())
    }

    #[test]
    fn manifest_asks_for_elevation_only_when_the_project_does() {
        let quiet = ManifestSettings::from_config(&serde_json::json!({}));
        assert!(!quiet.require_admin);
        assert!(quiet.dpi_aware);
        assert!(quiet.xml().contains("level=\"asInvoker\""));
        assert!(quiet.xml().contains("<dpiAware"));

        // Scaling on: both elements agree, and the newer one names the sharpest
        // mode Windows 10 1607 through 1703 knows before 1703 takes over.
        let xml = quiet.xml();
        assert!(xml.contains(">true</dpiAware>"));
        assert!(xml.contains(">PerMonitorV2, PerMonitor</dpiAwareness>"));

        let loud = ManifestSettings::from_config(&serde_json::json!({
            "install": { "require_admin": true },
            "ui": { "dpi_aware": false },
        }));
        assert!(loud.require_admin);
        assert!(!loud.dpi_aware);
        let xml = loud.xml();
        assert!(xml.contains("level=\"requireAdministrator\""));
        assert!(xml.contains(">false</dpiAware>"));
        assert!(xml.contains(">unaware</dpiAwareness>"));
        // The manifest must stay well-formed XML for the Windows loader.
        assert_eq!(xml.matches("<assembly ").count(), 1);
        assert_eq!(xml.matches("</assembly>").count(), 1);
        assert!(xml.starts_with("<?xml"));
        assert!(xml.trim_end().ends_with("</assembly>"));
    }
}
