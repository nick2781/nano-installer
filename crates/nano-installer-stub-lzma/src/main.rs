#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = run() {
        if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--extract")) {
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
    if command == std::ffi::OsStr::new("--extract") {
        let archive = args.next().map(std::path::PathBuf::from).ok_or_else(|| {
            anyhow::anyhow!("usage: lzma-stub-native.exe --extract <archive.7z> <directory>")
        })?;
        let destination = args.next().map(std::path::PathBuf::from).ok_or_else(|| {
            anyhow::anyhow!("usage: lzma-stub-native.exe --extract <archive.7z> <directory>")
        })?;
        anyhow::ensure!(args.next().is_none(), "unexpected --extract arguments");
        return extract_7z(&archive, &destination);
    }
    anyhow::bail!(
        "unsupported LZMA stub argument: {}",
        command.to_string_lossy()
    )
}

fn extract_7z(archive: &std::path::Path, destination: &std::path::Path) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::io::Write;

    std::fs::create_dir_all(destination)?;
    let destination = destination.canonicalize()?;
    sevenz_rust::decompress_file_with_extract_fn(archive, &destination, |entry, reader, _| {
        let relative = safe_archive_path(entry.name()).map_err(sevenz_rust::Error::other)?;
        let output = destination.join(relative);
        if entry.is_directory() {
            std::fs::create_dir_all(&output)?;
        } else {
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::io::BufWriter::new(std::fs::File::create(&output)?);
            std::io::copy(reader, &mut file)?;
            file.flush()?;
        }
        Ok(true)
    })
    .with_context(|| format!("failed to extract {}", archive.display()))?;
    Ok(())
}

fn safe_archive_path(name: &str) -> Result<std::path::PathBuf, String> {
    use std::path::Component;

    let mut output = std::path::PathBuf::new();
    for component in std::path::Path::new(name).components() {
        match component {
            Component::Normal(value) => output.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("unsafe 7z entry path: {name}"));
            }
        }
    }
    if output.as_os_str().is_empty() {
        return Err("empty 7z entry path".to_string());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::safe_archive_path;

    #[test]
    fn rejects_unsafe_7z_paths() {
        assert!(safe_archive_path("app/bin.exe").is_ok());
        assert!(safe_archive_path("../outside.exe").is_err());
        assert!(safe_archive_path("C:/outside.exe").is_err());
    }
}
