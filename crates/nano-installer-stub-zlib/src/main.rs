#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = run() {
        // A windowless run reports to whoever started it. The other cases keep
        // the product-skinned notice, which is a no-op when there is no window.
        if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--extract"))
            || nano_installer_core::silent_mode()
        {
            eprintln!("error: {error:#}");
        } else {
            nano_installer_core::show_runtime_error(&error);
        }
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let Some(command) = args.next() else {
        return nano_installer_core::run_installer_runtime();
    };
    // A windowless run is the runtime's own entry point, which reads the whole
    // command line itself. Treating it as an unknown option here would report a
    // usage error through a box nobody can click, hanging an unattended run.
    if command == std::ffi::OsStr::new(nano_installer_core::SILENT_FLAG) {
        return nano_installer_core::run_installer_runtime();
    }
    if command == std::ffi::OsStr::new("--extract") {
        let archive = args.next().map(std::path::PathBuf::from).ok_or_else(|| {
            anyhow::anyhow!("usage: zlib-stub-native.exe --extract <archive.zip> <directory>")
        })?;
        let destination = args.next().map(std::path::PathBuf::from).ok_or_else(|| {
            anyhow::anyhow!("usage: zlib-stub-native.exe --extract <archive.zip> <directory>")
        })?;
        anyhow::ensure!(args.next().is_none(), "unexpected --extract arguments");
        return extract_zip(&archive, &destination);
    }
    anyhow::bail!(
        "unsupported zlib stub argument: {}",
        command.to_string_lossy()
    )
}

fn extract_zip(archive: &std::path::Path, destination: &std::path::Path) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::io::Write;

    let file = std::fs::File::open(archive)?;
    let mut archive = zip::ZipArchive::new(file).context("failed to open ZIP payload")?;
    std::fs::create_dir_all(destination)?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .context("failed to read ZIP entry")?;
        let relative = entry
            .enclosed_name()
            .map(std::path::Path::to_path_buf)
            .with_context(|| format!("unsafe ZIP entry path: {}", entry.name()))?;
        let output = destination.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut writer = std::io::BufWriter::new(std::fs::File::create(output)?);
        std::io::copy(&mut entry, &mut writer)?;
        writer.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn zip_backend_rejects_invalid_archive() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let archive = temp.path().join("invalid.zip");
        std::fs::write(&archive, b"not a zip")?;
        assert!(super::extract_zip(&archive, &temp.path().join("out")).is_err());
        Ok(())
    }
}
