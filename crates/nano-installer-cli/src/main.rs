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
        bail!("usage: nano-installer-native-x64.exe build --project <directory> [--output <exe>] [--stubs <directory>]");
    }
    let mut project = None;
    let mut output = None;
    let mut stubs = None;
    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--project" => project = args.next().map(std::path::PathBuf::from),
            "--output" => output = args.next().map(std::path::PathBuf::from),
            "--stubs" => stubs = args.next().map(std::path::PathBuf::from),
            value => bail!("unsupported argument: {value}"),
        }
    }
    let mut request =
        nano_installer_core::BuildRequest::new(project.context("--project is required")?);
    request.output = output;
    request.stub_directory = stubs;
    let result = nano_installer_core::build_project_with_progress(request, |event| {
        eprintln!("{}", event.message);
    })?;
    println!("{}", result.summary.output_path.display());
    Ok(())
}
