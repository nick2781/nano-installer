# Build and release

## One release baseline

The CLI, the runtimes, and every generated setup use `x86_64-win7-windows-msvc`, with Windows 7
SP1 x64 as the minimum. The optional GUI builds for a Windows 10+ x64 host and never enters a setup
or a runtime.

```powershell
.\scripts\build.ps1
```

Release builds use the pinned `nightly-2025-11-08` toolchain with `rust-src`, `-Z build-std`, and
`panic=abort` against a static CRT. After building, the script unpacks real ZIP and 7z archives with
both runtimes and compares SHA-256 hashes; `scripts\smoke_backends.ps1` runs the same check on its
own.

## Release contents

```text
target/release/
├── nano-installer-native-x64.exe      # builder
├── nano-installer-gui-x64.exe         # Windows 10+ visual builder
└── stubs/
    ├── lzma-stub-native.exe
    ├── zlib-stub-native.exe
    └── uninst-stub-native.exe
```

The runtimes ship without product resources; icons, version info, and the application manifest are
injected per project.

Building a setup also audits it: `scripts/audit_application_manifest.ps1` reads the manifest
resource back and checks the execution level and DPI behaviour against the project configuration, so
a setup that silently lost its elevation requirement fails the build instead of shipping.

The release workflow builds the builder, the GUI, and the runtimes, then uploads the five
executables as separate release assets. It does not produce an archive and does not build or
publish the TapTap example setup. Passing `-Project` locally is what generates an example setup, so
the payload, layout, and bundle can be validated; the script also extracts the project's
`uninst.exe` and audits its Windows 7 imports and version resource on its own.

The example payload `examples/TapTap/payload/app.7z` is not stored in the repository, so CI builds
only the toolchain. Automated tests with a real payload are planned for a separate job.

## Release notes

Release notes come from `CHANGELOG.md`: `scripts/changelog_notes.ps1` extracts the section matching
the pushed tag. Write that version's `## [YYYY.M.D]` section before tagging; a missing or empty
section fails the release step instead of publishing an empty body.

Within a section, everything after `<!-- release-notes:end -->` is technical detail that stays in
the repository changelog. The published notes carry the product-facing summary above the marker.
Pass `-Full` to publish the whole section.

`scripts/changelog_notes.ps1` is saved with a UTF-8 BOM because it contains a Chinese footer and
Windows PowerShell decodes a BOM-less script with the ANSI code page. Without the BOM the footer
looks correct on a UTF-8 development machine and reaches the published notes as mojibake.
`scripts/audit_script_encoding.ps1` runs at the start of every build and fails if a script carrying
non-ASCII text has no BOM. `scripts/verify_release_notes.ps1` runs the generator the way the release
job does and compares the resulting footer with the decoded literal, which catches the same
regression through the published output; it can only fail where the ANSI code page is not UTF-8, so
CI is where it earns its keep.

## Builder arguments

```text
nano-installer-native-x64.exe build --project <dir> [--output <exe>] [--stubs <dir>]
```

- `--project` is required.
- `--output` is optional and defaults to `dist/<output.installer_name>` in the project.
- `--stubs` points at a directory holding the three runtime executables.
- `NANO_INSTALLER_NATIVE_STUB_DIR` overrides the runtime search directory.

## Signing

The builder does not sign anything yet. A production release must sign the setup after the icon,
version resources, and bundle are written, and sign the uninstaller separately before it is
embedded. `scripts/sign.ps1` is a placeholder signer; the default build never calls it.
