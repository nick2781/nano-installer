//! Native folder picker used by installer layouts.

use std::path::{Path, PathBuf};

/// Show a native folder picker and return the selected directory.
pub fn pick_folder(title: &str, initial_directory: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    unsafe {
        match pick_folder_windows(title, initial_directory) {
            Ok(path) => path,
            Err(error) => {
                tracing::warn!("Failed to open folder picker: {}", error);
                None
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (title, initial_directory);
        None
    }
}

#[cfg(windows)]
unsafe fn pick_folder_windows(
    title: &str,
    initial_directory: &Path,
) -> windows::core::Result<Option<PathBuf>> {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileDialog, IShellItem, SHCreateItemFromParsingName, FOS_FORCEFILESYSTEM,
        FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };

    struct ComGuard(bool);
    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    let com_result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    let _com_guard = ComGuard(com_result.is_ok());

    let dialog: IFileDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
    let options = dialog.GetOptions()? | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST;
    dialog.SetOptions(options)?;
    dialog.SetTitle(&HSTRING::from(title))?;

    if initial_directory.is_dir() {
        let initial_path = HSTRING::from(initial_directory.to_string_lossy().as_ref());
        if let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(&initial_path, None) {
            let _ = dialog.SetFolder(&item);
        }
    }

    if dialog.Show(HWND::default()).is_err() {
        return Ok(None);
    }

    let item = dialog.GetResult()?;
    let raw_path = item.GetDisplayName(SIGDN_FILESYSPATH)?;
    let selected_path = raw_path.to_string().map(PathBuf::from);
    CoTaskMemFree(Some(raw_path.0.cast()));

    Ok(Some(selected_path?))
}
