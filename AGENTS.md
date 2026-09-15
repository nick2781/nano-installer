# Repository Guidelines

## Project Layout

- `crates/nano-installer-core`: shared bundle, builder, XML layout, and Win32 runtime code.
- `crates/nano-installer-cli`: `nano-installer-native-x64.exe` project builder.
- `crates/nano-installer-gui`: Windows 10+ visual project builder; the only crate allowed to use eframe/egui.
- `crates/nano-installer-stub-lzma`: 7z/LZMA installer runtime only.
- `crates/nano-installer-stub-zlib`: ZIP/Deflate installer runtime only.
- `crates/nano-installer-uninstaller`: uninstaller runtime without archive backends.
- `examples/TapTap`: validation project; its branded resources belong to 易玩（上海）网络科技有限公司 and are not MIT-licensed.
- `docs`: user and architecture documentation.
- `scripts`: release build and PE compatibility checks.

## Build And Test

- Format: `cargo fmt --all -- --check`
- Test: `cargo test --locked --workspace`
- Lint: `cargo clippy --locked --workspace --all-targets -- -D warnings`
- Release: `.\scripts\build.ps1`
- Release plus TapTap setup: `.\scripts\build.ps1 -Project examples\TapTap`
- Backend smoke test: `.\scripts\smoke_backends.ps1`
- Example-only rebuild: `.\target\release\nano-installer-native-x64.exe build --project examples\TapTap`

All published executables target `x86_64-win7-windows-msvc`. Do not introduce a Win32, ANSI, or separate Windows 10 artifact.

## Change Boundaries

- Changes under `crates/**` or `scripts/build.ps1` require rebuilding the builder and all three stubs before rebuilding a setup.
- Changes limited to `examples/**` require only an example setup rebuild.
- Keep LZMA dependencies out of the ZIP and uninstaller crates, ZIP dependencies out of the LZMA and uninstaller crates, and archive dependencies out of the uninstaller.
- Keep eframe, egui, winit, and graphics backends confined to `nano-installer-gui`; they must not enter core, CLI, or stubs.
- Keep raw stubs free of icons and product resources. The builder injects project resources into generated executables.
- Do not edit or commit generated files under `target/` or example `dist/` directories.

## Verification

- Run the Win7 PE import audit for the builder, every stub, and each generated setup.
- Test archive extraction with real ZIP and 7z files after changing a backend.
- Treat generated setup size and per-stub size as release metrics; update documentation when they materially change.
- Preserve unrelated worktree changes and the untracked `docs/superpowers/` content.
