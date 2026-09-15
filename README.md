<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>English</b> | <a href="README.zh-CN.md">简体中文</a></p>

Configuration-driven Windows x64 installer builder. The CLI builder and every setup it generates
share one native Win32 runtime with a Windows 7 SP1+ Unicode baseline. The optional authoring GUI
is a separate Windows 10+ crate; egui/eframe never reach a stub or a setup.

> **Status:** experimental native implementation, not production-ready. Validate installations only
> in an isolated VM, and read [production status](docs/PRODUCTION_STATUS.md) before integrating a
> product.

## Crates and artifacts

| Crate | Artifact | Scope |
| --- | --- | --- |
| `nano-installer-core` | library | Project inspection, bundle assembly, XML layout parsing, Win32 UI, install/uninstall orchestration |
| `nano-installer-cli` | `nano-installer-native-x64.exe` | CLI frontend over the core build API |
| `nano-installer-gui` | `nano-installer-gui-x64.exe` | Windows 10+ egui frontend over the same core API |
| `nano-installer-stub-lzma` | `stubs/lzma-stub-native.exe` | 7z/LZMA extraction; links only `sevenz-rust` |
| `nano-installer-stub-zlib` | `stubs/zlib-stub-native.exe` | ZIP/Deflate extraction; links only `zip` |
| `nano-installer-uninstaller` | `stubs/uninst-stub-native.exe` | Manifest-driven removal; links no archive backend |

All published executables target `x86_64-win7-windows-msvc`. The GUI is the only Windows 10+ binary
and is never embedded in a setup.

## Build

Requirements: Windows x64, MSVC and a Windows SDK (Visual Studio 2022 Build Tools), and the pinned
`nightly-2025-11-08` toolchain with the components declared in `rust-toolchain.toml`.

```powershell
.\scripts\build.ps1
```

The script builds CLI and stubs for `x86_64-win7-windows-msvc` with `-Z build-std=std,panic_abort`,
static CRT, and `panic=abort`, builds the GUI against the host target with `+crt-static`,
smoke-tests real ZIP and 7z extraction with SHA-256 checks, audits PE imports against the Windows 7
baseline, and writes:

```text
target/release/
  nano-installer-native-x64.exe
  nano-installer-gui-x64.exe
  stubs/
    lzma-stub-native.exe
    zlib-stub-native.exe
    uninst-stub-native.exe
```

`target/x86_64-win7-windows-msvc/` and `target/gui-build/` are Cargo caches, not additional release
outputs. Measure the released binaries after each build; their sizes are release metrics, not part
of this document.

## Build a project

A project is a directory with `installer_config.json`, XML layouts, bitmap assets, JSON locales, an
already-compressed ZIP or 7z payload, and optional scripts. Every configured path resolves relative
to the project directory.

```powershell
.\target\release\nano-installer-native-x64.exe build `
  --project .\examples\TapTap `
  --output .\examples\TapTap\dist\TapTap_Setup.exe
```

- `--output` defaults to `dist/<output.installer_name>` inside the project.
- `--stubs <directory>` overrides the sibling `stubs/` lookup; `NANO_INSTALLER_NATIVE_STUB_DIR`
  overrides both.
- The builder reads the payload signature, copies the matching stub, injects the configured setup
  icon and `VERSIONINFO`, appends the bundle, and appends a self-contained uninstaller. It never
  copies itself into the setup and never creates an intermediate `skins.zip`.
- The GUI inspects and builds through the same core API; it does not spawn the CLI.

`examples/TapTap` is a test project, not a published setup or the GUI default. Its payload,
`examples/TapTap/payload/app.7z`, is not tracked, because `.gitignore` excludes `*.7z`; put a 7z
archive there before building it. To build it and audit the embedded uninstaller's Win7 imports and
file version:

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

Do not run its install action on a workstation: it writes files and an uninstall registry key, and
its default destination is under `Program Files` with no automatic UAC request.

## Verification

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

The release script adds the archive extraction checks and the PE import audit. Neither replaces an
end-to-end install/remove run on a clean Windows 7 SP1 VM; one HKCU write test is ignored in
restricted environments.

## Runtime behaviour and open work

Installation stages the payload, refuses to overwrite an existing destination, records deployed
files in a manifest, and registers its uninstaller. Removal deletes only manifest-tracked files.

Still open: upgrades, complete cancellation and recovery, task progress and page transitions, Rhai
script execution, shortcuts and autostart, automatic UAC, code signing, and Windows 7 VM
acceptance. Runtime startup copies the whole embedded bundle into memory, so offset-based access or
memory mapping is required before production use.

## Documentation

- [Quick start](docs/QUICK_START.md)
- [Project structure](docs/PROJECT_STRUCTURE.md)
- [Configuration reference](docs/CONFIG_REFERENCE.md)
- [XML layout guide](docs/XML_LAYOUT_GUIDE.md)
- [Localization](docs/LOCALIZATION.md)
- [GUI builder](docs/GUI.md)
- [Build and release](docs/BUILD_AND_RELEASE.md)
- [Test plan](docs/TEST_PLAN.md)
- [Native architecture](docs/NATIVE_ARCHITECTURE.md)
- [Windows compatibility](docs/WINDOWS_COMPATIBILITY.md)
- [Production status](docs/PRODUCTION_STATUS.md)
- [Documentation index](docs/README.md)

## Rights and license

- Rust source code in this repository is licensed under the [MIT License](LICENSE).
- `examples/TapTap` is a validation project only. The TapTap trademarks, images, and copy there
  belong to **易玩（上海）网络科技有限公司** and their respective rights holders, and are not
  MIT-licensed.
- Generated setups and uninstallers carry the product resources configured for that project, under
  that product's own licensing.
