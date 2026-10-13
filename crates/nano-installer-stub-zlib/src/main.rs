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
        let name = entry.name().context("failed to decode ZIP entry name")?;
        // ZIP 9 strips absolute prefixes; keep rejecting them before extraction.
        anyhow::ensure!(
            !std::path::Path::new(name.as_ref())
                .components()
                .any(|component| matches!(
                    component,
                    std::path::Component::Prefix(_) | std::path::Component::RootDir
                )),
            "unsafe ZIP entry path: {name}"
        );
        let relative = entry
            .enclosed_name()
            .with_context(|| format!("unsafe ZIP entry path: {name}"))?;
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
    use std::io::Write;

    #[test]
    fn zip_backend_rejects_invalid_archive() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let archive = temp.path().join("invalid.zip");
        std::fs::write(&archive, b"not a zip")?;
        assert!(super::extract_zip(&archive, &temp.path().join("out")).is_err());
        Ok(())
    }

    #[test]
    fn zip_backend_rejects_paths_outside_destination() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let archive = temp.path().join("unsafe.zip");
        let destination = temp.path().join("out");
        let outside = temp.path().join("outside.txt");
        let absolute = outside.to_string_lossy();
        for name in [
            "../outside.txt",
            "nested/../../outside.txt",
            r"..\outside.txt",
            r"C:outside.txt",
            absolute.as_ref(),
        ] {
            let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive)?);
            writer.start_file(name, zip::write::SimpleFileOptions::default())?;
            writer.write_all(b"must not be extracted")?;
            writer.finish()?;

            let error = super::extract_zip(&archive, &destination).unwrap_err();
            assert!(
                error.to_string().contains("unsafe ZIP entry path"),
                "{error:#}"
            );
            assert!(
                !outside.exists(),
                "an entry escaped the destination: {name}"
            );
            assert!(std::fs::read_dir(&destination)?.next().is_none());
        }
        Ok(())
    }
}
