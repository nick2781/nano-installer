# Test plan

## Automated checks

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
.\scripts\build.ps1 -Project examples\TapTap
```

The setup-level cases build a setup and then run it, so they need the runtime executables the builder
embeds. One script builds and tests in one go, and writes the whole run to `target/e2e-report.txt`.
The CI job keeps that report as an artifact:

```powershell
.\scripts\run_e2e_setup.ps1
.\scripts\run_e2e_setup.ps1 -RequireDesktop    # on a machine with a desktop session
```

The report names the commit it ran against, the commands, the requirements it turned on, and the
summary line. It lists every case the suite printed and what that case holds. You can still read the
result after the terminal that produced it is gone. The script sets
`NANO_INSTALLER_E2E_REQUIRE_STUBS=1` itself. `-RequireDesktop` adds one more requirement: a desktop
session.

The whole workspace suite runs through a script too, and writes the same kind of report:

```powershell
.\scripts\run_tests.ps1
```

`target/test-report.txt` holds the commit, the toolchain, the command, the whole output, the result
line of every target and the totals. `target/test-report.html` is the same run as a page: a verdict,
a figure per outcome, every case that failed first, a row per target, one row per case, and the
output with its result lines coloured.

A case row says what that case holds. This language's page of case descriptions is
[Test cases](TEST_CASES.md). Where that table has no row, the doc comment above the case in the
sources answers instead, or the case's own name when it carries none. The row also lists the
behaviours and settings [Test coverage](TEST_COVERAGE.md) says break when the case fails.

The page ties its two readings of one run together. A target's row links to the cases that ran in
it, and the closing note states the totals the case rows and the result lines agree on.

The page also has a behaviour card. It lists every behaviour that document names. Each one comes with
the cases this run ran for it, how those cases ended, and the layer that ran each one. A core library
case drives the library in process; a setup-level case builds and installs a real setup. A behaviour
the document names cases for, but that ran none this run, reads not run. The part of the document that
admits no case covers a thing sits in the same card. So a behaviour that was not tested does not
leave the page.

Once `scripts/capture_setup_snapshots.ps1` has photographed a real setup, the page shows those pages
too, one section per page. Each page carries its size, language, display scaling, the layout file it
was drawn from, and what the capture measured on it. The capture photographs every page the project
declares, and it needs a desktop session, so a build agent shows none.

The CI job keeps both files as the `test-report` artifact. `run_e2e_setup.ps1` writes the same pair
as `target/e2e-report.html` and `target/e2e-report.txt`. The setup-level cases build and run real
installers, so they need the runtime executables the builder embeds. Without them they skip. A full
check runs both scripts.

Both reports are written in Chinese by default. `-Language` changes that: `-Language en-US` writes
them in English, and the page's own language tag follows. The words come from
`scripts/report_text.json`, and the behaviours beside a case from
`docs/<language>/TEST_COVERAGE.md`. Case names, the doc comments above them, and the tools' own
output are shown as they are written.

Every behaviour these documents promise sits next to the case that holds it in
[Test coverage](TEST_COVERAGE.md), layer by layer. What no automated case reaches yet is in there
too.

The migration guide has an automated check of its own. `scripts/check_nsi_migration.ps1` reads the
tables in `docs/en/MIGRATION_FROM_NSIS.md` and `docs/zh-CN/MIGRATION_FROM_NSIS.md`. It looks at every
NSIS command, directive and `${...}` variable used by `examples/nsis-migration/legacy.nsi`. Each one
has to have a row that names what it becomes. The two languages have to agree command for command and
answer for answer, and a disagreement fails outright. With `-FailOnUnknown`, which is how CI runs it,
a construct with no row fails too. So an example using something new cannot slip through. What it
checks is that every construct was classified, not that a classification is right.

Unit tests reach bundle roundtrip, entry digest checking, payload embedding, button hit testing,
temporary-directory deployment, manifest writing, refusing to overwrite an existing directory,
upgrades and stale-file cleanup, failure rollback, and the uninstall rules for shortcuts and user
data. Layout tests reach nested flow containers and spacing, percentage sizing, progress bar
clipping, falling back from an out-of-range page, and the binding between progress pages and status
text. Script tests reach deploying files and the manifest from a script, rolling back a failing
script, replaying the manifest through `run_tracked_uninstall`, and the library fallback when a
script skips cleanup. The page hook reaches the page it picks for a given `from`, the wizard and the
machine it reads, and the refusal when it answers with something other than text. When a script has
no such function, the page order does not change.

The build script audits PE imports for the builder, all three runtimes, the setup, and the embedded
uninstaller. It also reads the manifest resource back out of the generated setup and the embedded
uninstaller. It fails when the declared execution level or DPI behaviour differs from what the
project configuration resolves to. One test writes to `HKCU`. Restricted environments ignore it by default,
so run it explicitly inside an isolated VM.

`crates/nano-installer-core/tests/e2e_setup.rs` covers the seams the unit tests cannot see. A change
that packs the wrong layout, drops the payload, or forgets a resource passes every unit test and
still produces a setup that cannot install. Those defects live between two binaries. That is why the
suite writes its own project, builds a setup from it with the real builder, and then runs that setup
and the uninstaller it deployed.

- Build product: the footer the runtime reads, a payload appended behind a real PE, the version and
  manifest resources, the runtime that matches the payload format, and the step a project's own
  finalize command takes on the finished files. A setup it accepted still installs; one it refused
  stops the build and is not left on disk.
- Install: the payload lands on disk and comes back byte for byte. The manifest lists what was
  written. The uninstall entry points at the deployed uninstaller. It carries the size and the quiet
  command Windows' installed-programs list wants, plus the two flags that say there is no repair or
  modify step. A configured `%TEMP%` path is expanded. `--dir` beats the configured path. A run with
  an option it does not support, or a mistyped argument, installs no product file. The log a run
  writes -- in the temporary directory, or in the file `--log` names -- records the product, the
  machine and every step. On failure it is the file a support ticket gets.
- Upgrade: a second install over the first drops the files the new payload no longer ships. It
  leaves a file the payload does not own alone.
- Uninstall: the product, the registration and the directory go. A directory holding a file the user
  added survives.
- Window: the setup opens its wizard window, and the client area matches the size the project
  declares. A run without a window reaches none of that.
- Keyboard: Tab walks the controls in the order the page declares them. The ring shows which one the
  keyboard is on. Space and Enter do what that control does -- here a checkbox flips and a
  button walks to the next page. Tab from the last one wraps round to the first. A page that
  declares no control leaves the keyboard nothing to do. Only one direction sits in this case: a key
  message the suite posts carries no modifier, and the runtime asks Windows for it. So walking
  backwards is held still by a unit case instead.
- High contrast: with the setting on, the page, the card, the card's edge and the focus ring are
  painted with the colours the machine keeps for a window, a control face, a frame and a highlight.
  They are not the colours the pages declare. The picture a checkbox carries stays the pixels the
  project drew. The setting belongs to whoever is at the machine, so the case states the answer for
  the run instead of turning it on. It drives both answers itself. Past that only the message is
  covered: Windows tells the window when the user turns the setting on or off, or picks another
  scheme. The case checks that the window takes such a message and keeps painting the frame it
  already had. That answer cannot change underneath it.
- Signed setup: a certificate table appended behind the bundle keeps the runtime able to find its own
  resources, which is what Authenticode writes when the release pipeline signs the file.

Every case generates its fixture, carries no product payload and no third-party assets, installs
below the temporary directory, and registers under a registry key naming only that case. So parallel
cases cannot see each other, and repeated runs do not collide. A case that fails still cleans up
after itself. Without the runtime stubs the suite prints a skip and passes, so a job that validates
setups sets `NANO_INSTALLER_E2E_REQUIRE_STUBS=1` to turn that skip into a failure. The cases that
drive a window need an interactive desktop session on top of that. A process started as a service
has no window station, so they skip there, and `NANO_INSTALLER_E2E_REQUIRE_DESKTOP=1` turns that
skip into a failure. The cursor case asks for more than a window: only a session that is showing a
pointer can answer it. A hosted runner has a window station and draws windows, but has no mouse and
reports a null cursor, so that case prints its own skip there. A locked desktop is a different
matter: `LogonUI` runs in the session and the lock screen sits in front. The pointer reads back as
the arrow. Moving it in and out changes no window. Those cases fail rather than skip. What they
ask about is what a window does once the pointer really moved, and a skip would say nothing about
it.

Signing a built setup belongs to the release pipeline rather than to this suite. `scripts/sign.ps1`
signs a file with the certificate `NANO_INSTALLER_CERT_THUMBPRINT` names and verifies the result. The
builder offers the two hooks and hands the finished files to the command a project writes down
(uninstaller first, setup second), and stops when that command fails. The suite holds both ends of
that: one case shows the command really ran on the finished files and that a handled setup still
installs; one shows a command that refuses stops the build and leaves no setup behind. What the
suite covers is the part no certificate changes: the file still reads its own resources once the
signature has been appended.

## Screenshot snapshots

The snapshots need a desktop session for the same reason the window cases do, so they are taken on a
machine rather than on a build agent.

`scripts/capture_setup_snapshots.ps1` builds the example setup and photographs every page the
project declares. The wizard's first page is the one a reader meets. It is photographed once per
supported locale and once at each higher scaling (150% and 200% by default). Every other page is
photographed once. A setup opens the page `wizard.pages[0]` names and nothing else, so the script
places the page it wants first for that build. Afterwards it writes the project's configuration back
byte for byte:

```powershell
cargo build -p nano-installer-native-cli -p nano-installer-stub-lzma -p nano-installer-stub-zlib -p nano-installer-uninstaller
.\scripts\capture_setup_snapshots.ps1
```

The PNGs and a `manifest.json` land in `target/setup-snapshots`. Each snapshot is checked against
the project it came from, so the checks below are read out of `examples/TapTap` rather than written
into the script:

- The client area matches the page, and scaling scales it by its own factor. Each page follows its
  own layout, so the uninstaller's pages come out 574x358.
- A page whose layout declares a rounded corner comes out with every corner outside the window
  region, so its corners are cut.
- Every image and label the layout places by absolute coordinates drew something. An image holds more
  than one colour, and a label holds text pixels in the colour its layout declares.
- The page holds far more colours than a blank one.
- No two pages came out as the same picture, so each snapshot really is the page it asked for.
- On the first page, the button and the version line differ between locales, so the language reached
  the page.

A page is drawn rather than reached. The uninstaller's pages appear without an uninstall having run.
What a page shows once the flow gets there is held by the cases under Pages and controls. Nothing has
to be inside the payload either, since a page is drawn before anything is unpacked. The example's
payload is a 150 MB archive the repository does not track. A clone that never unpacked the example
has no such file. So the script builds against an empty archive of the same format and removes it
again.

The example asks for elevation, and the consent prompt would sit in front of the window and stop an
unattended run. The script clears `install.require_admin` for the build and puts the file back byte
for byte afterwards. `-KeepElevation` builds the project as it stands and waits for a person to
answer the prompt.

Whether the glyphs read correctly is a judgement rather than an equation, so the snapshots are also
meant to be looked at. `scripts/review_setup_snapshots.ps1` hands each page and its expectation to a
vision model running on this machine, and writes `review.md` next to the PNGs:

```powershell
.\scripts\review_setup_snapshots.ps1                       # Ollama on http://127.0.0.1:11434/v1
.\scripts\review_setup_snapshots.ps1 -Endpoint http://127.0.0.1:1234/v1 -Model <name>
```

It talks to a local OpenAI-compatible endpoint only. When nothing answers it says so and stops,
leaving the snapshots and their expectations for a person to read.

## Manual checks

The snapshots already cover the client area, the rounded corners, the images and labels a layout
places, and whether the first page's button and version line follow the language. What remains needs
someone at the machine. Start `examples/TapTap/dist/TapTap_Setup.exe` and confirm:

- Background, logo, tagline, and button images are visible with correct transparency.
- The install button and version text use the expected locale.
- The empty area at the top drags the window.
- Minimize and close respond.
- Chinese, English, and Russian text render without mojibake.
- The path field supports drag selection, double-click word selection, copy/paste, and undo. The
  folder icon picks a directory and writes it back into the field.
- An IME composition window appears at the caret and the candidate list sits just below it while
  typing Chinese or Japanese.
- The installation directory is gone as soon as the uninstall finishes, while files the user added
  are still kept.

## The minimum supported version gate

Before you claim Windows 7 support, run it once on the minimum supported version. Use a clean x64
virtual machine. Check WIC PNG decoding, GDI text, mouse input, and window behaviour there. Then, in
an isolated VM, test ZIP and 7z extraction, the manifest, the uninstall entry, and the uninstall
button with a fresh directory. Then check a few things: whether the installed files and the files
the user made survived, whether a failure rolls back, whether the installation directory is gone as
soon as the uninstall finishes, and whether the cleaner copy in the temporary directory has exited.

Never test the TapTap install action on a daily workstation. The example sets
`install.require_admin`, so a correct build shows the UAC consent prompt before the setup window
appears. Declining it must leave the machine untouched. To exercise a build that should never
prompt, clear `install.require_admin` and confirm the same setup starts without a consent prompt. To
exercise a user-writable directory, change the example's read-only `install.default_path` and
repack the setup.

## Production cases that cannot pass yet

- Authenticode signing chain.

Upgrades, failure rollback, shortcuts, autostart, the keep-data option, page transitions, and
progress display are implemented. They just have not been through a real run on a Windows 7 VM yet.
