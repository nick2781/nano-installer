## What this changes

<!-- One or two sentences. If it fixes an issue, write "Fixes #123" on its own line. -->

## How it was checked

<!-- Which of these ran, and what the result was. Delete the ones that do not apply. -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [ ] `.\scripts\run_tests.ps1`
- [ ] `.\scripts\run_e2e_setup.ps1` (rebuilt the builder and all three stubs first)
- [ ] `.\scripts\build.ps1` and a setup installed on a machine, if the change reaches the packaged product
- [ ] `.\scripts\audit_win7_imports.ps1` if the change touches Win32 code or adds a dependency

## Things this project asks for

- [ ] No new dependency crosses a crate boundary it should not: LZMA stays out of the ZIP and uninstaller crates, ZIP stays out of the LZMA and uninstaller crates, archives stay out of the uninstaller, and eframe/egui stay in `nano-installer-gui`.
- [ ] Published executables still target `x86_64-win7-windows-msvc`; no new Win32, ANSI, or Windows 10-only artifact was introduced.
- [ ] If I changed a configuration key, both `docs/en/CONFIG_REFERENCE.md` and `docs/zh-CN/CONFIG_REFERENCE.md` say so.
- [ ] If I changed the plugin ABI, the change only appends to the host table and `include/nano_plugin.h` still matches `crates/nano-installer-core/src/plugin.rs` field for field.
- [ ] Documentation is in both `docs/en` and `docs/zh-CN`; no generated file under `target/` or an example `dist/` is committed.
