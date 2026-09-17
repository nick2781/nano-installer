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

The example payload `examples/TapTap/payload/app.7z` is not stored in the repository, so the CI
job above builds only the toolchain. Setup-level validation runs in its own `setup-end-to-end` job:
`crates/nano-installer-core/tests/e2e_setup.rs` writes a project of its own, builds a setup from it,
and then runs that setup and the uninstaller it deployed. The fixture carries no product payload and
no third-party assets, and it installs below the temporary directory under a per-case registry key,
so the job needs no VM and touches no shared state.

That job sets `NANO_INSTALLER_E2E_REQUIRE_STUBS=1`, which turns "the runtime stubs are missing" from
a skip into a failure. Without it a job that built nothing would report every case as skipped and
still pass.

Every job also runs `scripts/audit_test_targets.ps1`, which asks Cargo which packages the workspace
has and fails if a `tests/*.rs` file sits outside all of them. A `tests/` directory next to the
virtual manifest looks like an integration suite but is never compiled, so its cases never run;
that is not hypothetical here, and the check exists because it happened.

## Version numbers

A release number is the day it is cut, using the CalVer scheme from <https://calver.org/>: a full
year, an unpadded month, and an unpadded day, such as `2026.9.17`, tagged `v2026.9.17`. The
calendar day is the project's stated UTC+08:00, which CalVer allows as long as the project says
which day it counts, so a release cut late in a Beijing evening keeps that day.

`scripts/release_version.ps1` is the single source for that rule:

```powershell
# The version to use today
.\scripts\release_version.ps1
# Check the version in Cargo.toml
.\scripts\release_version.ps1 -Version 2026.9.17
# Check a tag: the day, the commit, and that it matches Cargo.toml
.\scripts\release_version.ps1 -Tag v2026.9.17 -Version 2026.9.17 -Commit <sha>
```

`scripts/build.ps1` checks the `Cargo.toml` version when a build starts, and the release job checks
the tag with `-Tag`/`-Version`/`-Commit` before it builds, so the tag has to name the version in
`Cargo.toml`; a release whose binaries report a different version than the tag fails instead of
shipping. **A second release on one calendar day takes a modifier**,
such as `v2026.9.17-r2`, rather than yesterday's date nudged forward or a fourth number — CalVer
recommends at most three numeric segments. A zero-padded date (`2026.09.17`), a date that does not
exist (`2026.13.1`), a date before the commit being released, and a date after today are all
rejected, so a release can no longer be numbered for a day that has not happened.

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
