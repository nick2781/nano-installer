// Native project builder CLI. Runtime stubs live in a separate Cargo package.
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    use anyhow::{bail, Context};

    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("build")) {
        bail!("usage: nano-installer-native-x64.exe build --project <directory> [--output <exe>] [--stubs <directory>] [--delta-from <archive>] [--msi <package>]");
    }
    let mut project = None;
    let mut output = None;
    let mut stubs = None;
    let mut delta_from = None;
    let mut msi = None;
    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--project" => project = args.next().map(std::path::PathBuf::from),
            "--output" => output = args.next().map(std::path::PathBuf::from),
            "--stubs" => stubs = args.next().map(std::path::PathBuf::from),
            "--delta-from" => delta_from = args.next().map(std::path::PathBuf::from),
            "--msi" => msi = args.next().map(std::path::PathBuf::from),
            value => bail!("unsupported argument: {value}"),
        }
    }
    let mut request =
        nano_installer_core::BuildRequest::new(project.context("--project is required")?);
    request.output = output;
    request.stub_directory = stubs;
    request.delta_from = delta_from;
    request.msi = msi;
    let result = nano_installer_core::build_project_with_progress(request, |event| {
        eprintln!("{}", event.message);
    })?;
    println!("{}", result.summary.output_path.display());
    if let Some(msi) = result.msi {
        println!("{}", msi.output_path.display());
    }
    Ok(())
}
