# Build and release

This page is for someone who works on the tools themselves. A project that only wants a setup
builds nothing: it downloads the builder from a release and runs it, as the
[quick start](QUICK_START.md) describes. Everything below is how the published executables are
produced.

## One release baseline

The CLI, the runtimes, and every setup you generate build for `x86_64-win7-windows-msvc`, which is
the minimum supported version. The optional GUI needs a Windows 10+ x64 host to
build, but it never enters a setup or a runtime.

```powershell
.\scripts\build.ps1
```

Release builds use the pinned `nightly-2025-11-08` toolchain with `rust-src`, `-Z build-std`, and
`panic=abort` against a static CRT. After building, the script unpacks real ZIP and 7z archives with
both runtimes and compares SHA-256 hashes; `scripts\smoke_backends.ps1` runs the same check on its
own.

The workflows name their images instead of using `-latest`.
A release is built on `windows-2025-vs2026`, and the suite that has to be green behind the tag runs
on the same image, so what a release is made of does not change on GitHub's schedule.

`windows-latest` has moved from one Windows generation to the
next without a line in this repository, and a released executable whose toolchain cannot be named
afterwards is one nobody can rebuild. The Ubuntu jobs name `ubuntu-24.04` for the same reason,
`ubuntu-latest` being announced to move to Ubuntu 26 on 2026-10-19. Moving any of these names is a
commit, and CI tests that commit like any other.

`scripts/audit_workflow_images.ps1` is what keeps them named: it fails when a job goes back to
`-latest`, or when an image is decided by an expression it cannot read. It also fails when
**the release job and the suite that backs a tag stop naming the same Windows image**. That last one
matters most, because the binaries that were audited are then not the ones being shipped.

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

The runtimes ship without product resources: the builder injects icons, version info, and the
application manifest per project.

The runtime is embedded in every setup, so its size is a release metric too.
On this machine the three stubs are 1.47-1.54 MB and the builder is 583 KB, which is why the
`release` profile is size-first (`opt-level = "z"`).
The build adds the linker's `/OPT:ICF=3`, because the MSVC linker does not fold at that level by
default. It also passes `panic = "immediate-abort"`, a nightly flag: a panic stops there instead of
building a message nobody reads, and turns off Rhai's `no_optimize` and `no_time` as well.
The visual builder
is built with a `gui` profile of its own, because `z` makes it larger.
A setup also carries only the pages and pictures the runtime inside it can open, and the uninstaller
takes the pictures its own pages name. An icon does not travel in the bundle at all: the build
injects it into the executable as a PE resource, which is where `LoadIconW` reads it from.

Building a setup also audits it:
`scripts/audit_application_manifest.ps1` reads the manifest resource back and checks the execution
level and DPI behaviour against the project configuration.
A setup that silently lost its elevation requirement fails the build instead of shipping.

The release workflow builds the builder, the GUI, and the runtimes, then uploads the five
executables as separate release assets. It does not produce an archive and does not build or publish
the TapTap example setup: an example setup is generated only by passing `-Project` locally. Passing
`-Msi` as well writes the installer package an estate deploys beside it, so you can validate the
payload, layout, and bundle. The script also extracts the project's `uninst.exe` and audits its
imports for the minimum supported version and its version resource on its own.

A release is published only from a commit whose suite passed: the tag is pushed by hand and the
release job does not run the suite itself. So the job asks GitHub whether the newest **whole-suite**
run of the commit the tag names succeeded — that is `scripts/audit_release_evidence.ps1`, and it
runs before anything expensive. A push run is always the whole suite, and a dispatched run counts as
one only when it was not given targets of its own.

The workflow records that in the name of the job that runs them: `ci.yml` calls that job
`Native Win7+ Build (narrowed)` when `suite_command` is set. A dispatched run of a commit whose
workflow file predates that marker is not counted at all, because afterwards there is no way to tell
the two apart. This is not a formality here, because a run of this repository is ended routinely
when a newer push supersedes it, so a commit's last word is often a cancellation rather than a
verdict.

It is also the way back: a tag whose push run was cancelled is published after the whole suite is
dispatched on the tag itself (`gh workflow run ci.yml --ref <tag>`), so no new commit has to be
pushed for it.

`ci.yml` runs on branch pushes, on pull requests and when it is dispatched, not on tag pushes.
A tag push is a release event, and a run started by it would be a *newer* whole-suite run that is
still going, so the gate would refuse the very commit the tag names.
That is what happened when v2026.9.28 was cut: the release went out after that run finished and the
job was run again, since testing a commit twice buys nothing.

The job also writes `SHA256SUMS.txt` (`scripts/release_manifest.ps1`) and attaches it to the release
beside the five executables, one digest per file, in the form `sha256sum -c` and
`shasum -c` read, so a download can be checked with

```bash
sha256sum -c SHA256SUMS.txt
```

Downloading is the one step of an install nobody can check afterwards, and the product already
refuses a dependency whose bytes do not match the digest its project wrote down, so the assets this
repository ships deserve the same.

The example payload `examples/TapTap/payload/app.7z` is not stored in the repository, so the CI job
above builds only the toolchain. Setup-level validation runs in its own `setup-end-to-end` job:
`crates/nano-installer-core/tests/e2e_setup.rs` writes a project of its own, builds a setup from it,
and then runs that setup and the uninstaller it deployed. The fixture carries no product payload and
no third-party assets, and it installs below the temporary directory under a per-case registry key,
so the job needs no VM and touches no shared state.

That job sets `NANO_INSTALLER_E2E_REQUIRE_STUBS=1`, which turns "the runtime stubs are missing" from
a skip into a failure; without it a job that built nothing would report every case as skipped and
still pass.

A run also ends the runs it replaces, which is the `supersede` job, and both build jobs need it.
`Test workspace` occasionally never finishes, and the cause is the build agent rather than a commit.
A step is over once its output has closed, but something the step started sometimes holds that
output open, and no timeout on the runner's side reaches such a step.

While it sits there it also holds the pipeline's concurrency, so the next push waits behind it
rather than building. The same commit passes when it is dispatched again, and the other job of the
same wedged run is green, so the job force-cancels the in-progress and queued runs of the same
branch before the build jobs start.

It is allowed to fail: a cancel the API refuses must not turn a build
that would be green into a red one.

A dispatched run may also choose the targets the suite runs (`suite_command`). That is how a target
a wedged run stopped on is taken apart on the agent that reproduces the problem, which a developer's
machine is not. The build job runs the suite elevated, so cases that install a service or write a
machine-wide key take their round trip there rather than their refusal. A developer's plain run
never reaches those branches.

One more thing is worth knowing about a step that stops answering: the cache. A run that is
cancelled never reaches the step that saves it, and the next run restores whatever was saved last. A
save that was interrupted leaves a `target/` tree that cargo can stop on, which looks exactly like a
suite that stops answering. The cache steps in `ci.yml` carry a versioned key that is bumped by hand
(`ci-v3` at the time of writing).

A new key throws the old caches away, and it is the cheapest thing to try: the run after
that key change finished its suite in 103 s, where the six runs before it never finished at all.

The deadline inside `run_tests.ps1` and `run_e2e_setup.ps1` ends a step whose command never ends. It
lists the processes still alive before it takes the tree down, so a hang leaves a record of what it
hung on. That list is read from the process table rather than from WMI, because a
`Get-CimInstance Win32_Process` can itself block for as long as the machine is unwell. A diagnosis
that hangs the step it is diagnosing leaves no log at all, which is how a branch meant to end a
wedged step became part of the wedge.

Command lines, which only WMI has, are given up for that, because
a pid, a name, a start time and a window title are enough to name a holder.

The takedown itself is bounded too, in `Stop-ProcessTree`.
`taskkill /T` waits on the very tree that may be holding everything up, so a tree it cannot finish
off within two minutes is left to the job the step joined. A watchdog outside the step
(`Start-StepWatchdog`, whose work is `scripts/step_observer.ps1`) ends the step's **whole process
tree** when its time is up, because a step that stops answering never ends itself.
What the agent waits on is the step's output as much as the step:
a descendant that inherited that output keeps it open after the step's own process is gone.

That difference is measured, not assumed: with the step's process exiting at once and a descendant
living 25 s, the step's output closed 25.2 s later. `Stop-Process` on the step alone left the
descendant alive, where `taskkill /T /F` ended it. What a step that stops answering costs is
measured rather than promised. The two runs of 2026-09-28 that stopped answering both stayed
`in_progress`, 42m56s and 11m45s, until the next run's `supersede` job force-cancelled them.

What survives is the watchdog's record, a commit status the commit keeps, and not the
step's own ending, because the watchdog runs inside the step's own job (`KillOnJobClose`).
The system takes it down the moment the step's script ends, so it never gets to post that the step
is gone. Two statuses stopping at the same instant is a reading in itself: the step died rather than
the machine froze.

If a run does stop answering, the `CI janitor` workflow checks every fifteen
minutes: a CI run that has been going for more than thirty minutes gets a fresh run dispatched on
its own branch. That run's `supersede` job ends the wedged one, so the pipeline recovers without
anyone watching it or pushing a commit.

Commands are started differently as well.
`NanoStepCommand` in `scripts/step_job.ps1` creates the process with `CreateProcess`: it passes the
child **none** of this process's handles, and gives it no console, while the shell
redirects the command's output to a file.
A build agent gives its step a pipe for output and calls the step finished when that pipe closes.
`Process.Start` hands every inheritable handle to the child, so one process that outlives the
command would keep the step open however long it lives.

That watchdog is also where the record of a run comes from, and that is a change of design rather
than a detail. The phase report used to be a commit status the suite posted itself, and the wedges
ended at the first such post made while a command was running, because a status posted from inside
the suite is a network call the suite waits on.

That call has no bound Windows PowerShell enforces: `Invoke-RestMethod` is implemented on
`HttpWebRequest`, whose `Timeout` covers getting the response and not reading its body, so a
response that starts and never finishes is a call that never returns however short the timeout is.
Measured here against a server that writes its headers and then sends nothing, it was still waiting
after 45 s with `-TimeoutSec 10`. A step that is inside that call says nothing, not even that its
own deadline passed, and those runs left three statuses and no more.

The suite now writes where it is into a crumb file: it is a local append, made **before** each thing
that could block, so a step stuck inside one of them has already said where it was. The watchdog
reads that file and the output file of the command the step is running, and posts both, so a wedged
run leaves all three contexts behind. `suite phase` is what the step said. `suite output` is the
last line of the command it is running, which for a test harness is the name of the test that never
answered. `suite step` is whether the step's process is alive, how much processor time it has used,
how long it has been quiet, and how much room the workspace volume has left.

A machine that is gone posts nothing at all, and that
silence is itself the finding.

The posting itself waits at most ten seconds per report.
The wait is in this process's own hands rather than left to the client, because a request whose
connection is black-holed can sit in a call that never returns however the client's timeout is set.

The reports it gave up on are counted and named in the next one (`N post(s) unanswered`), so a
machine whose network has gone says so instead of going quiet. The watcher was soaked here against a
server that accepts every request and answers none: it ran to its own deadline, kept watching, and
reported the losses.

The library's unit cases now run one at a time (`-- --test-threads=1`), a choice that has a control
behind it rather than a hunch.

Three runs of 2026-09-28 dispatched the same target on the agent with one thing changed at a time.
Default threads with `--nocapture` stopped answering twice, 11 s and 4 s after the target began. One
thread with `--nocapture` passed in 5m50s, where one thread without it is what the suite has been
running green.

The thread count is the variable, and the extra output is not.
Five of the six runs that stopped answering earlier stopped inside that same target within seconds of
it starting. Several of its cases touch the machine rather than a temporary directory: one installs
a service, one writes a machine-wide key, one asks whether this process may do either.

The build
job's step is elevated and a developer's plain run is not, and a local two-thread run of that target
finishes in 11.8 s, which is why it cannot be reproduced here. That is still the suite not doing
those things at once rather than a proof that the agent's condition is gone, so the `suite_command`
input above stays for the next time it is looked at.

The suite also reads the step's console quietly: `run_tests.ps1 -Quiet` is what the workflow runs, and
the console gets two things: the verdict of each target, and the last of what it said when a target
fails.
The whole of every command goes into `target/test-report.txt` and `.html`, which is the artifact the
job uploads, because a report is a file and a console is a pipe that something may stop draining.
Reading a command's output back is bounded in the same spirit: the tail is the last 256 KB of what
it wrote. A command that never ends keeps writing, and a read that has to reach the end waits for
the command.

Every job also runs `scripts/audit_test_targets.ps1`: it asks Cargo which packages the workspace has
and fails if a `tests/*.rs` file sits outside all of them. A `tests/` directory next to the virtual
manifest looks like an integration suite but is never compiled, so its cases never run. That is not
hypothetical here, and the check exists because it happened.

## Version numbers

A release number is the day you cut it, using the CalVer scheme from <https://calver.org/>: a full
year, an unpadded month, and an unpadded day, such as `2026.9.17`, tagged `v2026.9.17`. The calendar
day is the project's stated UTC+08:00. CalVer allows that as long as the project says which day it
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
the tag with `-Tag`/`-Version`/`-Commit` before it builds. The tag has to name the version in
`Cargo.toml`, so a release whose binaries report a different version than the tag fails instead of
shipping.

A second release on one calendar day takes a modifier, such as `v2026.9.17-r2`, rather than
yesterday's date nudged forward or a fourth number, since CalVer recommends at most three numeric
segments. A zero-padded date (`2026.09.17`), a date that does not exist (`2026.13.1`), a date before
the commit being released, and a date after today are all rejected. You can no longer number a
release for a day that has not happened.

## Release notes

Release notes come from `CHANGELOG.md`: `scripts/changelog_notes.ps1` extracts the section matching
the pushed tag. Write that version's `## [YYYY.M.D]` section before you tag, because a missing or
empty section fails the release step instead of publishing an empty body.

`scripts/changelog_notes.ps1` is saved with a UTF-8 BOM because it contains a Chinese footer, and
Windows PowerShell decodes a BOM-less script with the ANSI code page. Without the BOM the footer
looks correct on a UTF-8 development machine and reaches the published notes as mojibake.

`scripts/audit_script_encoding.ps1` runs at the start of every build and fails if
a script carrying non-ASCII text has no BOM. `scripts/verify_release_notes.ps1` runs the generator
the way the release job does and compares the resulting footer with the decoded literal, which
catches the same regression through the published output. It can only fail where the ANSI code page
is not UTF-8, so CI is where it earns its keep.

Within a section, everything after `<!-- release-notes:end -->` is technical detail that stays in
the repository changelog, and the published notes carry the product-facing summary above the marker.
Pass `-Full` to publish the whole section.

The published half is what a reader sees beside a download.
It holds only what changed for them: a capability that appeared, a behaviour that improved, a defect
that is gone, a limit that was made honest. How the release job is wired is not part of that, so
repository script paths, workflow files, branch protection, dependency bots and the pipeline's own
tests all go below the marker, where they stay in the repository's record.

The product's own vocabulary is not repository vocabulary: a project keeps its install logic in
`scripts/`, and a note about the builder handing a finished setup to the project's own signing
pipeline describes the reader's pipeline, not this repository's. `scripts/audit_release_notes.ps1`
fails a section whose published half carries the repository's, and it names the line and the word it
refused; CI runs it beside the generator check, and the release job runs it again before it builds
the body. It reads every section in the file, and the bodies published before the rule existed were
rewritten to match it rather than left as they were.

The issue forms name a release in their version field, so that placeholder moves with the version
rather than advertising one two releases back; it is updated as part of cutting a release.

## Checking a published release

Everything the release job checks, it checks before publishing.
The tag names the version in `Cargo.toml`, the suite behind the commit is green, the binaries pass
the import audit for the minimum supported version, and the digest list is written from the files
that are uploaded. None of that says what a download turns out to be, which is what
`scripts/smoke_release.ps1` checks:

```powershell
.\scripts\smoke_release.ps1 -Tag v2026.9.29
.\scripts\smoke_release.ps1 -Tag v2026.9.29 -SkipUi
.\scripts\smoke_release.ps1 -Tag v2026.9.17 -AllowNoDigests -SkipUi
```

It fetches every asset of the tag, recomputes each digest against `SHA256SUMS.txt`, reads the version
resource out of the builder and the GUI and requires the version the tag names, and then — unless
`-SkipUi` — builds the TapTap example with the downloaded builder and the downloaded stubs and runs
every page it declares, checking each against the project's own layout. That last step is the one
worth having: the bytes a user receives draw the wizard, rather than a build from the sources. It
needs `gh` to be authenticated, and that step needs a desktop session.

The digest list is what makes a download checkable at all, and it began with `v2026.9.28`, so an
older release is checked with `-AllowNoDigests`, which tolerates its absence without pretending the
digests were verified.

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

A setup is made by appending the payload to itself: the builder reads it in 1 MiB blocks, and each
block goes both into the setup and into the SHA-256 the bundle records for it. Memory does not grow
with the payload, while the time a build takes grows roughly with the payload's bytes. Measured on
this machine, Windows 11, x86_64, with the builder and the runtimes built by
`cargo build --release`:

| Payload | Setup | Build time | Peak working set |
| --- | --- | --- | --- |
| 8 MiB | 11.9 MiB | 1.5 s | 2.5 MiB |
| 128 MiB | 131.9 MiB | 2.0 s | 2.5 MiB |
| 512 MiB | 516 MiB | 3.1 s | 2.5 MiB |
| 1024 MiB | 1028.1 MiB | 4.7 s | 2.5 MiB |

The four megabytes a setup carries beyond its payload are the three runtimes, the project's own
interface resources, and the embedded uninstaller; the working set is sampled every 20 ms, and the
table shows the highest sample each build reached.

```powershell
# Build the release builder, then measure
cargo build --release -p nano-installer-native-cli
.\scripts\measure_build.ps1 -PayloadMiB 8,128,512,1024
```

The script writes a project that builds, whose payload is one stored (uncompressed) entry of each
requested size. It builds each one and records its time and peak working set into
`target/build-cost.txt`, removing its intermediate files as it goes. The numbers depend on the
machine, the disk, and how compressible the payload is, and what they pin down is that memory does
not follow the payload, which is what a streamed copy buys.

## Update packages

```text
nano-installer-native-x64.exe build --project <dir> --delta-from <previous payload archive> [--output <exe>]
```

`--delta-from` names the payload archive of the release this one replaces, which is the archive the
project ships as `resources.payload_file`. The builder expands that archive and the project's
current payload with the runtime stubs, compares them by size and SHA-256, writes only the files
whose bytes changed into a ZIP of its own, and embeds that archive under the name the project gives
its payload. The setup therefore packs the ZIP runtime whatever format the project's own payload
uses, and the build reports how many files stay on the machine and how much the update carries.

An update package installs over one release and no other, and the runtime checks every file it did
not carry: that each is there, holds the recorded byte count and hashes to the recorded digest. It
decides before it writes anything, so a machine that does not match is told to run the full setup
instead of ending up with half of each version. The files the update left in place stay in the
manifest, so an uninstall takes them back with the rest.

Two limits.
A project that cuts its content into components has no single archive to compare against, and an
update build of one is refused. An update package covers payload files only: the layouts, assets,
locales and scripts always travel with the setup, so a release that changes its pages is still a
full setup.

`BuildRequest.delta_from` is the same thing through the API, and `BuildResult.update` reports what
the build kept and carried.

A setup installs the product; it does not replace itself, and NSIS ships no updater either: products
built with it decide on their own when to ask for a new version and then run a new setup. This
framework follows the same line, so there is no "check for updates" switch in the configuration. A
project that wants automatic updates builds the loop out of the primitives it already has:
`download_file_with_hash` fetches the new setup and checks its digest, and `run_command` runs it
silently. The version in the manifest says which release the machine holds.

## Installer packages

```text
nano-installer-native-x64.exe build --project <dir> --msi <package.msi>
```

`--msi` wraps the setup this build finishes in the package an estate deploys through Windows
Installer: Group Policy, Intune or Configuration Manager. The package is written after the project's
own command has had the setup, so the image it carries is signed exactly when the setup is signed.
The build reports the package's product code and the directory it installs into.

```powershell
# Install silently, into a directory of your choosing
msiexec /i TapTap.msi /qn /norestart INSTALLDIR="D:\Programs\TapTap"
# Remove it again; the package finds the product where it put it
msiexec /x TapTap.msi /qn /norestart
```

The package installs for the whole machine when the project asks for administrator rights
(`install.require_admin`) and for the calling user otherwise. With no directory of its own it
installs into `%ProgramFiles%\<name>` or `%LOCALAPPDATA%\Programs\<name>`. A newer package upgrades
the release an older one installed, and a project that does not support a silent run is refused
a package rather than given one that cannot install.

A second release of the same day is the version written `2026.9.17-r2`, as
[Version numbers](#version-numbers) describes. It is the same version as far as Windows Installer is
concerned, because it compares three fields. A package whose product code and version are already on
the machine cannot be installed again, and the Installer refuses it with 1638, "another version of
this product is already installed". Such a release is a product of its own: the product code follows
the version as written. The package's own upgrade search counts the version it names among those to
take away, so the second release removes the first and then installs itself. One product is left on
the machine, and removing it is an ordinary removal.

An administrative install (`msiexec /a`) lays the image out without installing the product, because
the package is a delivery vehicle for the setup, not a second installation of it.
`BuildRequest.msi` and `BuildResult.msi` are the same thing through the API.

## Signing

The builder signs nothing itself; it hands the finished files to a command of the project's own. The
setup triggers `finalize.installer` after the icon, version resources, and bundle are written, and
the uninstaller triggers `finalize.uninstaller` while it is still a file of its own and before the
setup embeds it. The installer package triggers `finalize.package` once it is written, and only a
build asked for `--msi` writes one; that package is what Group Policy and Intune deploy, so it is
the file whose signature a machine checks. The
[configuration reference](CONFIG_REFERENCE.md#commands-that-run-on-the-finished-build) spells the
settings out, and a command that exits non-zero stops the build, leaving the file it refused off
disk.

`scripts/sign.ps1` is what a pipeline usually writes there: it signs with the certificate
`NANO_INSTALLER_CERT_THUMBPRINT` names, and timestamps against `http://timestamp.digicert.com`
unless `NANO_INSTALLER_TIMESTAMP_URL` says otherwise. It verifies its own work with
`signtool verify /pa`, and a certificate that is missing, or a signature that failed, ends the
build.

```json
"finalize": {
  "uninstaller": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign.ps1 -File \"%1\"",
  "installer": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign.ps1 -File \"%1\""
}
```

A signed setup is still a setup. Authenticode appends its certificate table behind everything the
build wrote, and the runtime looks for its bundle footer in the last megabyte of the file rather
than at its very end. So the setup installs and uninstalls as it always did.

## Rehearsing the signing flow

A release is signed with a certificate from a commercial authority, and one is not always available:
a free code-signing programme can decide that a project is not yet known well enough to be given
one, and a project without a certificate has nothing to put in `NANO_INSTALLER_CERT_THUMBPRINT`.
The flow can still be walked through end to end, because [Sigstore](https://www.sigstore.dev) issues
the certificate to the workflow itself rather than to anyone who holds a key.
`.github/workflows/sigstore-flow.yml` signs a file with cosign, which asks Fulcio for that
certificate and proves the request with the run's own OIDC token — which is why the job holds
`id-token: write`, and why no private key appears anywhere in it.

That certificate is good for ten minutes, so the workflow asserts the window instead of trusting it:
the job reads `notBefore` and `notAfter` out of the issued certificate and fails unless they differ
by exactly ten minutes, and it fails when the bundle holds no Rekor entry either. The two belong
together. Windows validates a chain as it stood when the file was signed, so a signature whose
certificate has expired is checked against a timestamp rather than against the current time, and a
short-lived certificate without one says nothing at all the next morning. Sigstore keeps that
record in a different place: the transparency log entry, and an RFC 3161 timestamp in the bundle.

None of this signs an executable, and none of it replaces the Authenticode signature. Sigstore's
output is a separate file — a bundle, with the detached signature and certificate beside it — so a
setup or a builder keeps exactly the bytes the build produced, with no certificate table appended to
them and nothing new for Windows to trust. The "unknown publisher" a user sees beside a download
does not change because of this workflow, and its three files are uploaded as that run's artifacts
and attached to no release.

The workflow is dispatched by hand and by nothing else, for the same reason: it has no `push`, `tag`
or `schedule` trigger, so a release can neither start it nor wait on it. The file it signs is
`include/nano_plugin.h`, a source file that every checkout holds, because what is tried here is the
pipeline rather than the artifact; a project's own setup is the file to sign once a certificate
exists, and `scripts/verify_signing.ps1` is what reads a real signature back.
