# Project layout

## Repository

```text
crates/
├── nano-installer-core/        # bundling, XML layout, WIC/GDI rendering, shared runtime
├── nano-installer-cli/         # command-line builder
├── nano-installer-gui/         # Windows 10+ visual builder
├── nano-installer-stub-lzma/   # 7z runtime, links only sevenz-rust
├── nano-installer-stub-zlib/   # ZIP runtime, links only zip/deflate
└── nano-installer-uninstaller/ # uninstaller runtime, no archive backend
docs/
├── en/                         # English documentation
└── zh-CN/                      # Chinese documentation
examples/TapTap/                # validation project
examples/nsis-migration/        # a project migrated off NSIS, with the script that checks it
scripts/                        # build, smoke test, PE audit
```

The raw runtime stubs carry no product resources. The builder injects icons, version info, and
branding from your project configuration, so one runtime can produce setups for any product.

A project leaving NSIS starts at [Migrating from NSIS](MIGRATION_FROM_NSIS.md), which says what every
construct becomes here; `examples/nsis-migration/migrated/` is a buildable project written to it.

## A product project

```text
product-installer/
├── installer_config.json
├── layouts/                    # XML pages
├── assets/                     # backgrounds, buttons, icons
├── locales/                    # one JSON file per language
├── scripts/                    # optional install/uninstall logic
└── payload/app.7z              # or a ZIP archive
```

The builder resolves every configured path relative to the project folder, and collects the whole
`layouts`, `assets`, and `locales` directories, the optional `scripts` directory, and the payload
you name in the configuration.

## Build outputs

```text
target/release/
├── nano-installer-native-x64.exe
├── nano-installer-gui-x64.exe
└── stubs/
    ├── lzma-stub-native.exe
    ├── zlib-stub-native.exe
    └── uninst-stub-native.exe
```

A build asked for `--msi` writes the installer package an estate deploys beside the setup
it wrapped.

`target/x86_64-win7-windows-msvc/` is Cargo's cross-target cache, not a second set of release
files. Example setups land in `examples/TapTap/dist/` and are not part of a release.
