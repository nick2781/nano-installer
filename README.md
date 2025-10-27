# Nano Installer

> Modern Windows installer/uninstaller built with Rust + GPUI

[![Rust](https://img.shields.io/badge/Rust-1.75.0-orange.svg)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/Windows-7%20SP1%2B-blue.svg)](https://www.microsoft.com/windows)
[![License](https://img.shields.io/badge/license-TBD-green.svg)](LICENSE)

[中文文档](README_CN.md) | **English**

## Features

- 🌍 **Multi-language**: 5 languages with automatic detection
- 🎨 **Modern UI**: GPU-accelerated with GPUI + gpui-component
- 📱 **DPI Aware**: Automatic 1x/2x image scaling
- 📦 **7z Compression**: Embedded payload installation
- 🔐 **Code Signing**: Integrated signing support
- 📝 **Structured Logging**: Detailed logs with tracing
- 🚀 **Silent Install**: CLI support (`/S /D /L`)
- 🗑️ **Complete Uninstall**: Clean removal of all components

## Quick Start

### For New Developers

```bash
# 1. Read documentation index
cat docs/README.md              # Complete documentation map
cat docs/guides/START_HERE.md   # 5-minute guide
cat docs/status/FINAL_STATUS.md # Project status

# 2. Install Rust 1.75.0 (REQUIRED for Windows 7)
rustup install 1.75.0
rustup default 1.75.0

# 3. Build
cargo build --release

# 4. Test silent install
./target/release/installer.exe /S /D=C:\TestApp
```

### Documentation

| Document                                                       | Description                    |
| -------------------------------------------------------------- | ------------------------------ |
| [📚 Documentation Index](docs/README.md)                        | **Complete documentation map** |
| [docs/guides/START_HERE.md](docs/guides/START_HERE.md)         | Quick navigation guide         |
| [docs/guides/FINAL_STEP.md](docs/guides/FINAL_STEP.md)         | 5-minute implementation guide  |
| [docs/IMPLEMENTATION_STEPS.md](docs/IMPLEMENTATION_STEPS.md)   | Detailed steps                 |
| [docs/GPUI_COMPONENTS_GUIDE.md](docs/GPUI_COMPONENTS_GUIDE.md) | Component usage                |
| [docs/DPI_AWARE.md](docs/DPI_AWARE.md)                         | DPI management                 |
| [docs/RUST_VERSION.md](docs/RUST_VERSION.md)                   | ⚠️ Rust version requirements    |

## System Requirements

### Runtime
- Windows 7 SP1+ (64-bit)
- macOS (future support)

### Development
- **Rust 1.75.0** (⚠️ REQUIRED! Last version supporting Windows 7)
- Visual Studio Build Tools (Windows)
- Git

**⚠️ IMPORTANT**: Do NOT upgrade to Rust 1.76+! It requires Windows 10+.

See [docs/RUST_VERSION.md](docs/RUST_VERSION.md) for details.

## Project Stats

| Item          | Count          |
| ------------- | -------------- |
| Total Files   | 88             |
| Rust Files    | 50             |
| Lines of Code | 3500+          |
| Documentation | 26 files       |
| Image Assets  | 31 (DPI aware) |
| Languages     | 5              |

## Architecture

```
nano-installer/
├── src/
│   ├── bin/          # 3 executables
│   ├── ui/           # UI module (DPI aware)
│   ├── installer/    # Installation logic
│   ├── uninstaller/  # Uninstallation logic
│   ├── i18n/         # Multi-language (.pak format)
│   ├── resources/    # Resource management
│   ├── logger/       # Logging system
│   └── common/       # Common utilities
├── assets/           # 31 images (1x/2x)
├── locales/          # 5 language files
└── docs/             # 26 documentation files
```

## Build

```bash
# Development build
cargo build

# Release build
cargo build --release

# Build language packs
cargo run --bin langpack-builder -- --input locales --output dist/locales
```

## Usage

### Installer

```bash
# GUI mode
installer.exe

# Silent installation
installer.exe /S /D=C:\Program Files\MyApp /L=en-US
```

### Uninstaller

```bash
# GUI mode
uninstaller.exe

# Silent uninstall
uninstaller.exe /S
```

## Tech Stack

- **Core**: Rust 1.75.0 (⚠️ Precise version for Win7 support)
- **UI**: GPUI + gpui-component
- **Async**: tokio
- **Logging**: tracing
- **Windows API**: windows crate
- **Compression**: sevenz-rust (7z)

## Development

### Adding New Languages

1. Create `locales/{locale}.json` (e.g., `fr.json`)
2. Ensure all required keys are included
3. Run the language pack builder
4. Update `src/i18n/mod.rs`

### Testing

```bash
cargo test
```

### Windows 7 Testing

See [docs/GPUI_COMPATIBILITY.md](docs/GPUI_COMPATIBILITY.md) for:
- GPUI compatibility status
- Fallback to egui (if needed)
- Testing procedures

## Project Status

**Completion**: 95% ✅

**Completed**:
- ✅ Full architecture
- ✅ All backend logic
- ✅ Multi-language system
- ✅ UI framework
- ✅ DPI aware assets
- ✅ Complete documentation

**Remaining** (5%):
- ⏳ GPUI rendering implementation (2-3 days)
- ⏳ Windows 7 testing (1 day)

See [docs/status/FINAL_STATUS.md](docs/status/FINAL_STATUS.md) for detailed status.

## Contributing

This project is currently in active development. For contribution guidelines, please see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## License

TBD

## Acknowledgments

- **GPUI**: Modern GPU-accelerated UI framework
- **gpui-component**: Ready-to-use UI components
- **Rust Community**: Excellent ecosystem

---

**For detailed Chinese documentation, see [README_CN.md](README_CN.md)**
