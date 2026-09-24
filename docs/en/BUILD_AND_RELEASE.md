# Build and release

## One release baseline

The CLI, the runtimes, and every setup you generate use `x86_64-win7-windows-msvc`, with Windows 7
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

The runtimes ship without product resources; the builder injects icons, version info, and the
application manifest per project.

Building a setup also audits it: `scripts/audit_application_manifest.ps1` reads the manifest
resource back and checks the execution level and DPI behaviour against the project configuration,
so a setup that silently lost its elevation requirement fails the build instead of shipping.

The release workflow builds the builder, the GUI, and the runtimes, then uploads the five
executables as separate release assets. It does not produce an archive and does not build or publish
the TapTap example setup. Pass `-Project` locally to generate an example setup, and `-Msi` to also write
the installer package an estate deploys beside it, so you can validate the payload, layout, and
bundle; the script also extracts the project's `uninst.exe` and audits its Windows 7 imports and
version resource on its own.

The example payload `examples/TapTap/payload/app.7z` is not stored in the repository, so the CI job
above builds only the toolchain. Setup-level validation runs in its own `setup-end-to-end` job:
`crates/nano-installer-core/tests/e2e_setup.rs` writes a project of its own, builds a setup from it,
and then runs that setup and the uninstaller it deployed. The fixture carries no product payload and
no third-party assets, and it installs below the temporary directory under a per-case registry key,
so the job needs no VM and touches no shared state.

That job sets `NANO_INSTALLER_E2E_REQUIRE_STUBS=1`, which turns "the runtime stubs are missing" from
a skip into a failure. Without it a job that built nothing would report every case as skipped and
still pass.

A run also ends the runs it replaces, in the `supersede` job that both build jobs need. `Test
workspace` occasionally never finishes, and the cause is the build agent rather than a commit: a step
is over once its output has closed, something the step started sometimes holds that output open, and
no timeout on the runner's side reaches such a step -- while it sits there it also holds the
pipeline's concurrency, so the next push waits behind it rather than building. The same commit passes
when it is dispatched again, and the other job of the same wedged run is green, so the job
force-cancels the in-progress and queued runs of the same branch before the build jobs start. It is
allowed to fail: a cancel the API refuses must not turn a build that would be green into a red one.

One more thing is worth knowing about a step that stops answering: the cache. A run that is cancelled
never reaches the step that saves it, so the next run restores whatever was saved last -- and a save that
was interrupted leaves a `target/` tree that cargo can stop on, which looks exactly like a suite that stops
answering. The cache steps in `ci.yml` therefore carry `key: ci-v2`: a new key throws the old caches away,
and it is the cheapest thing to try (the run after that key change finished its suite in 103 s, where the
six runs before it never finished at all).

The deadline inside `run_tests.ps1` and `run_e2e_setup.ps1` is what ends a step whose command never
ends, and it lists the processes still alive before it takes the tree down, so a hang leaves a record
of what it hung on. That list is read from the process table rather than from WMI: a
`Get-CimInstance Win32_Process` can itself block for as long as the machine is unwell, and a
diagnosis that hangs the step it is diagnosing leaves no log at all -- which is how a branch meant to
end a wedged step became part of the wedge. Command lines, which only WMI has, are given up for that;
a pid, a name, a start time and a window title are enough to name a holder. The takedown itself is
bounded too, in `Stop-ProcessTree`: `taskkill /T` waits on the very tree that may be holding
everything up, so a tree it cannot finish off within two minutes is left to the job the step joined.
A watchdog outside the step (`Start-StepWatchdog`) ends the step's own process when its time is up,
because a step that stops answering never ends itself. If a run does stop answering, the `CI janitor` workflow checks every fifteen minutes: a CI run that has
been going for more than thirty minutes gets a fresh run dispatched on its own branch, whose `supersede` job
ends the wedged one, so the pipeline recovers without anyone watching it or pushing a commit.
Commands are started differently as well: `NanoStepCommand` in `scripts/step_job.ps1` creates the
process with `CreateProcess`, passes it **none** of this process's handles, and gives it no console
at all -- the shell redirects the command's output to a file. A build agent gives its step a pipe for
output and calls the step finished when that pipe
closes, while `Process.Start` hands every inheritable handle to the child -- so one process that
outlives the command would keep the step open however long it lives.

Every job also runs `scripts/audit_test_targets.ps1`, which asks Cargo which packages the workspace
has and fails if a `tests/*.rs` file sits outside all of them. A `tests/` directory next to the
virtual manifest looks like an integration suite but is never compiled, so its cases never run; that
is not hypothetical here, and the check exists because it happened.

## Version numbers

A release number is the day you cut it, using the CalVer scheme from <https://calver.org/>: a full
year, an unpadded month, and an unpadded day, such as `2026.9.17`, tagged `v2026.9.17`. The calendar
day is the project's stated UTC+08:00, which CalVer allows as long as the project says which day it
counts, so a release cut late in a Beijing evening keeps that day.

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
shipping. A second release on one calendar day takes a modifier, such as `v2026.9.17-r2`, rather
than yesterday's date nudged forward or a fourth number: CalVer recommends at most three numeric
segments. A zero-padded date (`2026.09.17`), a date that does not exist (`2026.13.1`), a date
before the commit being released, and a date after today are all rejected, so you can no longer
number a release for a day that has not happened.

## Release notes

Release notes come from `CHANGELOG.md`: `scripts/changelog_notes.ps1` extracts the section matching
the pushed tag. Write that version's `## [YYYY.M.D]` section before you tag; a missing or empty
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
nano-installer-native-x64.exe build --project <dir> [--output <exe>] [--stubs <dir>] [--delta-from <archive>] [--msi <package>]
```

- `--project` is required.
- `--output` is optional and defaults to `dist/<output.installer_name>` in the project.
- `--stubs` points at a directory holding the three runtime executables.
- `--delta-from` names an earlier release's payload archive and builds an update package
  instead of a full setup; see below.
- `--msi` writes the installer package an estate deploys around the finished setup.
- `NANO_INSTALLER_NATIVE_STUB_DIR` overrides the runtime search directory.

## Build time and memory

A setup is made by appending the payload to itself: the builder reads it in 1 MiB blocks, and each block
goes both into the setup and into the SHA-256 the bundle records for it. Memory therefore does not grow
with the payload, while the time a build takes grows roughly with the payload's bytes. Measured on this
machine (Windows 11, x86_64, with the builder and the runtimes built by `cargo build --release`):

| Payload | Setup | Build time | Peak working set |
| --- | --- | --- | --- |
| 8 MiB | 11.9 MiB | 1.5 s | 2.5 MiB |
| 128 MiB | 131.9 MiB | 2.0 s | 2.5 MiB |
| 512 MiB | 516 MiB | 3.1 s | 2.5 MiB |
| 1024 MiB | 1028.1 MiB | 4.7 s | 2.5 MiB |

The four megabytes a setup carries beyond its payload are the three runtimes, the project's own
interface resources, and the embedded uninstaller. The working set is sampled every 20 ms, and the
table shows the highest sample each build reached.

```powershell
# Build the release builder, then measure
cargo build --release -p nano-installer-native-cli
.\scripts\measure_build.ps1 -PayloadMiB 8,128,512,1024
```

The script writes a project that builds and whose payload is one stored (uncompressed) entry of each
requested size, builds each one, and records its time and peak working set into `target/build-cost.txt`,
removing its intermediate files as it goes. The numbers depend on the machine, the disk, and how
compressible the payload is; what they pin down is that memory does not follow the payload, which is
what a streamed copy buys.

## Update packages

```text
nano-installer-native-x64.exe build --project <dir> --delta-from <previous payload archive> [--output <exe>]
```

`--delta-from` names the payload archive of the release this one replaces -- the archive the project
ships as `resources.payload_file`. The builder expands that archive and the project's current payload
with the runtime stubs, compares them by size and SHA-256, writes only the files whose bytes changed
into a ZIP of its own, and embeds that archive under the name the project gives its payload. The
setup therefore packs the ZIP runtime whatever format the project's own payload uses, and the build
reports how many files stay on the machine and how much the update carries.

An update package installs over one release and no other. The runtime checks the files it did not
carry -- that each is there, holds the recorded byte count and hashes to the recorded digest -- and
decides before it writes anything, so a machine that does not match is told to run the full setup
instead of ending up with half of each version. The files the update left in place stay in the
manifest, so an uninstall takes them back with the rest.

Two limits. A project that cuts its content into components has no single archive to compare
against, and an update build of one is refused. An update package covers payload files only: the
layouts, assets, locales and scripts always travel with the setup, so a release that changes its
pages is still a full setup.

`BuildRequest.delta_from` is the same thing through the API, and `BuildResult.update` reports what
the build kept and carried.

A setup installs the product; it does not replace itself. NSIS ships no updater either: products
built with it decide on their own when to ask for a new version and then run a new setup, and this
framework follows the same line, so there is no "check for updates" switch in the configuration.
A project that wants automatic updates builds the loop out of the primitives it already has:
`download_file_with_hash` fetches the new setup and checks its digest, `run_command` runs it
without a window, and the version in the manifest says which release the machine holds.

## Installer packages

```text
nano-installer-native-x64.exe build --project <dir> --msi <package.msi>
```

`--msi` wraps the setup this build finishes in the package an estate deploys
through Windows Installer: Group Policy, Intune or Configuration Manager. The
package is written after the project's own command has had the setup, so the
image it carries is signed exactly when the setup is signed, and the build
reports the package's product code and the directory it installs into.

```powershell
# Install without a window, into a directory of your choosing
msiexec /i TapTap.msi /qn /norestart INSTALLDIR="D:\Programs\TapTap"
# Remove it again; the package finds the product where it put it
msiexec /x TapTap.msi /qn /norestart
```

The package installs for the whole machine when the project asks for
administrator rights (`install.require_admin`) and for the calling user
otherwise; with no directory of its own it installs into
`%ProgramFiles%\<name>` or `%LOCALAPPDATA%\Programs\<name>`. A newer package
upgrades the release an older one installed, and a project that does not support
a windowless run is refused a package rather than given one that cannot install.

A second release of the same day -- the version written `2026.9.17-r2`, as
[Version numbers](#version-numbers) describes -- is the same version as far as Windows
Installer is concerned, because it compares three fields: a package whose product code and
version are already on the machine cannot be installed again, and the Installer refuses it
with 1638, "another version of this product is already installed". Such a release is
therefore a product of its own: the product code follows the version as written, and the
package's own upgrade search counts the version it names among those to take away, so the
second release removes the first and then installs itself. One product is left on the
machine, and removing it is an ordinary removal.

An administrative install (`msiexec /a`) lays the image out without installing
the product: the package is a delivery vehicle for the setup, not a second
installation of it. `BuildRequest.msi` and `BuildResult.msi` are the same thing
through the API.

## Signing

The builder signs nothing itself; it hands the finished files to a command of the project's own. The
setup triggers `finalize.installer` after the icon, version resources, and bundle are written, the
uninstaller triggers `finalize.uninstaller` while it is still a file of its own and before the setup
embeds it, and the installer package triggers `finalize.package` once it is written -- only a build
asked for `--msi` writes one, and that package is what Group Policy and Intune deploy, so it is the
file whose signature a machine checks. The [configuration
reference](CONFIG_REFERENCE.md#commands-that-run-on-the-finished-build) spells the settings out.
A command that exits non-zero stops the build, and the file it refused is not left on disk.

`scripts/sign.ps1` is what a pipeline usually writes there: it signs with the certificate
`NANO_INSTALLER_CERT_THUMBPRINT` names, timestamps against `http://timestamp.digicert.com` unless
`NANO_INSTALLER_TIMESTAMP_URL` says otherwise, and verifies its own work with `signtool verify /pa`.
A certificate that is missing, or a signature that failed, ends the build.

```json
"finalize": {
  "uninstaller": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign.ps1 -File \"%1\"",
  "installer": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign.ps1 -File \"%1\""
}
```

A signed setup is still a setup: Authenticode appends its certificate table behind everything the
build wrote, and the runtime looks for its bundle footer in the last megabyte of the file rather than
at its very end, so the setup installs and uninstalls as it always did.
