fn main() {
    if let Err(error) = nano_installer_native::run_builder_cli() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
