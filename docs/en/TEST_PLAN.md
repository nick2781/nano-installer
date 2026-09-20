# Test plan

## Automated checks

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
.\scripts\build.ps1 -Project examples\TapTap
```

The setup-level suite builds a setup and runs it, so it needs the runtime executables the builder
embeds. One script builds them, runs the suite, and writes the whole run to `target/e2e-report.txt`,
which the CI job keeps as an artifact:

```powershell
.\scripts\run_e2e_setup.ps1
.\scripts\run_e2e_setup.ps1 -RequireDesktop    # on a machine with a desktop session
```

The report names the commit it ran against, the commands, the requirements and the summary line, and
lists every case the suite printed with what that case holds, so a result can still be read after
the terminal that produced it is gone. The script sets
`NANO_INSTALLER_E2E_REQUIRE_STUBS=1` itself; `-RequireDesktop` adds the desktop requirement.

The whole workspace suite runs through a script too, and writes the same kind of report:

```powershell
.\scripts\run_tests.ps1
```

`target/test-report.txt` holds the commit, the toolchain, the command, the whole output, the result
line of every target and the totals, and `target/test-report.html` is the same run as a page: a
verdict, a figure per outcome, every case that failed first, a row per target, one row per case and
the output with its result lines coloured. A case row says what that case holds, which is the doc
comment above it in the sources or its own name when it carries none, and which behaviours and
settings [Test coverage](TEST_COVERAGE.md) says break when it fails. The page keeps its two readings
of one run tied together: a target's row links to the cases that ran in it, and the closing note
states the totals the case rows and the result lines agree on. A behaviour card lists every
behaviour that document names, with the cases this run ran for it and how those cases ended, so a
behaviour whose cases it names but which ran none reads not run; the part of the document that
admits no case covers a thing sits in the same card, which states what was not tested rather than
leaving it to be assumed. Once
`scripts/capture_setup_snapshots.ps1` has photographed a real setup, the page shows those pages too,
each captioned with its size, language, display scaling and the layout file it was drawn from, and
with what the capture measured on it; the capture photographs the wizard's first page only, so the
later pages have no photograph. That capture needs a desktop session, so a build agent's page has
none.

The CI job keeps both as the `test-report` artifact, and `run_e2e_setup.ps1` writes the same pair as
`target/e2e-report.html` and `target/e2e-report.txt`. The setup-level cases build and run real
installers, so they need the runtime executables the builder embeds and skip without them; a full
check runs both scripts.

Both reports are written in Chinese unless `-Language` says otherwise: `-Language en-US` writes
them in English, and the page's own language tag follows. The words come from
`scripts/report_text.json` and the behaviours beside a case from
`docs/<language>/TEST_COVERAGE.md`; case names, the doc comments above them and the tools' own
output are shown as they are written.

Every behaviour these documents promise sits next to the case that holds it in
[Test coverage](TEST_COVERAGE.md), layer by layer, including what no automated case reaches yet.

Unit tests cover bundle roundtrip, payload embedding, button hit testing, temporary-directory
deployment, manifest writing, refusing to overwrite an existing directory, upgrades and stale-file
cleanup, failure rollback, and uninstall rules for shortcuts and user data. Layout tests cover
nested flow containers and spacing, percentage sizing, progress bar clipping, falling back from an
out-of-range page, and the binding between progress pages and status text. Script tests cover
deploying files and the manifest from a script, rolling back a failing script, replaying the
manifest through `run_tracked_uninstall`, and the library fallback when a script skips cleanup.

The build script audits PE imports for the builder, all three runtimes, the setup, and the embedded
uninstaller. It also reads the manifest resource back out of the generated setup and the embedded
uninstaller, and fails if the declared execution level or DPI behaviour differs from what the
project configuration resolves to. One test writes to `HKCU`, so it is ignored by default in
restricted environments; run it explicitly inside an isolated VM.

`crates/nano-installer-core/tests/e2e_setup.rs` covers the seams the unit tests cannot see. A change
that packs the wrong layout, drops the payload, or forgets a resource passes every unit test and
still produces a setup that cannot install, because the defects live between two binaries. That is
why the suite writes its own project, builds a setup from it with the real builder, and then runs
that setup and the uninstaller it deployed.

- Build product: the footer the runtime reads, a payload appended behind a real PE, the version and
  manifest resources, and the runtime that matches the payload format.
- Install: the payload lands on disk and comes back byte for byte, the manifest lists what was
  written, the uninstall entry points at the deployed uninstaller, a configured `%TEMP%` path is
  expanded, `--dir` beats the configured path, and an unopted or mistyped run installs nothing.
- Upgrade: a second install over the first drops the files the new payload no longer ships and
  leaves a file the payload does not own alone.
- Uninstall: the product, the registration, and the directory go; a directory holding a file the
  user added survives.
- Window: the setup opens its wizard window, and the client area matches the size the project
  declares. A run without a window reaches none of that.
- Signed setup: a certificate table appended behind the bundle, which is what Authenticode writes
  when the release pipeline signs the file, keeps the runtime able to find its own resources.

Every case generates its fixture, carries no product payload and no third-party assets, installs
below the temporary directory, and registers under a registry key naming only that case, so parallel
cases cannot see each other and repeated runs do not collide. A case that fails still cleans up
after itself. Without the runtime stubs the suite prints a skip and passes, so a job that validates
setups sets `NANO_INSTALLER_E2E_REQUIRE_STUBS=1` to turn that skip into a failure. The window case
needs an interactive desktop session on top of that, which a process started as a service has no
window station for: it skips there, and `NANO_INSTALLER_E2E_REQUIRE_DESKTOP=1` turns that skip into
a failure.

Signing a built setup belongs to the release pipeline rather than to this suite: `scripts/sign.ps1`
signs a file with the certificate `NANO_INSTALLER_CERT_THUMBPRINT` names and verifies the result.
What the suite covers is the part no certificate changes, which is that the file still reads its own
resources once the signature has been appended.

## Screenshot snapshots

The snapshots need a desktop session for the same reason the window case does, so they are taken on
a machine rather than on a build agent.

`scripts/capture_setup_snapshots.ps1` builds the example setup and photographs the page it opens,
once per supported locale and once at 150% scaling:

```powershell
cargo build -p nano-installer-native-cli -p nano-installer-stub-lzma -p nano-installer-stub-zlib -p nano-installer-uninstaller
.\scripts\capture_setup_snapshots.ps1
```

The PNGs and a `manifest.json` land in `target/setup-snapshots`. Each snapshot is checked against
the project it came from, so what is verified is read out of `examples/TapTap` rather than written
into the script:

- The client area matches the page, and 150% scaling scales it.
- Every corner falls outside the window region, so the corners are cut.
- The logo and the tagline drew artwork instead of a flat rectangle.
- The install button and the version line drew text in the colour their layout declares.
- The page holds far more colours than a blank one.
- The button and the version line differ between locales, so the language reached the page.

Only the first page is photographed, so nothing has to be inside the payload. The example's payload
is a 150 MB archive the repository does not track, and a clone that never unpacked the example has
no such file, so the script builds against an empty archive of the same format and removes it again.

The example asks for elevation, and the consent prompt would sit in front of the window and stop an
unattended run. The script clears `install.require_admin` for the build and puts the file back byte
for byte afterwards; `-KeepElevation` builds the project as it stands and waits for a person to
answer the prompt.

Whether the glyphs read correctly is a judgement rather than an equation, so the snapshots are also
meant to be looked at. `scripts/review_setup_snapshots.ps1` hands each page and its expectation to a
vision model running on this machine, and writes `review.md` next to the PNGs:

```powershell
.\scripts\review_setup_snapshots.ps1                       # Ollama on http://127.0.0.1:11434/v1
.\scripts\review_setup_snapshots.ps1 -Endpoint http://127.0.0.1:1234/v1 -Model <name>
```

It talks to a local OpenAI-compatible endpoint only, and when nothing answers it says so and stops,
leaving the snapshots and their expectations for a person to read.

## Manual checks

The snapshots already cover the client area, the cut corners, the images, and whether the button and
the version line follow the language. What remains needs someone at the machine: start
`examples/TapTap/dist/TapTap_Setup.exe` and confirm:

- Background, logo, tagline, and button images are visible with correct transparency.
- The install button and version text use the expected locale.
- The empty area at the top drags the window.
- Minimize and close respond.
- Chinese, English, and Russian text render without mojibake.
- The path field supports drag selection, double-click word selection, copy/paste, and undo, and
  the folder icon picks a directory and writes it back into the field.
- An IME composition window appears at the caret and the candidate list sits just below it while
  typing Chinese or Japanese.
- The installation directory is gone as soon as the uninstall finishes, while files the user added
  are still kept.

## Windows 7 SP1 gate

Before you claim Windows 7 support, verify WIC PNG decoding, GDI text, mouse input, and window
behaviour on a clean Windows 7 SP1 x64 VM. In an isolated VM, test ZIP and 7z extraction, the
manifest, the uninstall entry, and the uninstall button using a fresh directory; then check
installed files, preservation of user-created files, failure rollback, and that the installation
directory is gone as soon as the uninstall finishes and the cleaner copy in the temporary directory
has exited.

Never test the TapTap install action on a daily workstation. The example sets
`install.require_admin`, so a correct build shows the UAC consent prompt before the setup window
appears; declining it must leave the machine untouched. To exercise a build that should never
prompt, clear `install.require_admin` and confirm the same setup starts without a consent prompt. To
exercise a user-writable directory, change the example's read-only `install.default_path` and
repack the setup.

## Production cases that cannot pass yet

- Authenticode signing chain.

Upgrades, failure rollback, shortcuts, autostart, the keep-data option, page transitions, and
progress display are implemented but not yet accepted on a real Windows 7 VM.
