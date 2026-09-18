# Production status

**The current implementation is not ready for production distribution.** Install actions write
files and registry entries, so validate them in a disposable virtual machine only.

## Verified

- The builder and the runtime are separate binaries; a setup never contains the builder.
- ZIP and 7z payloads are detected from the file signature and routed to the matching runtime.
- Both runtimes unpack real archives correctly, checked against expected SHA-256 hashes.
- The uninstaller uses its own runtime and page list and links no archive backend.
- The CLI and the Windows 10+ GUI drive the same build API; GUI dependencies stay out of the
  runtime.
- A setup carries the payload, project resources, and a self-contained uninstaller with its own
  icon and version resources.
- Installing extracts the payload, deploys files and the uninstaller into a new directory, writes
  the manifest and the uninstall entry; uninstalling removes only what the manifest records.
- Reinstalling over an existing installation upgrades in place, drops files the new payload no
  longer ships, and restores the previous version if anything fails.
- The first page renders background bitmaps and Unicode text natively, in a window with no border
  and rounded corners that you can drag, minimize, and close; it shows a hand cursor over
  clickable controls.
- Every artifact goes through a PE import audit for Windows 7 SP1 x64 compatibility.
- Progress pages report live status, and the finish page can launch the deployed application.
- Shortcuts and autostart entries are created during install and restored on uninstall, including
  through rollback.
- Uninstall terminates a running product, removes shortcuts, and deletes declared user data only
  when you clear the keep-data option.
- `scripts/install.rhai` and `scripts/uninstall.rhai` run in an embedded Rhai engine whose
  primitives reuse the built-in deployment, rollback, and manifest code; a failing script rolls
  back, and a script that skips manifest cleanup falls back to the library removal.
- Flow layout covers nested `VBox`, `HBox`, and `Content` containers with padding, margins,
  percentage sizing, `justify-content`, and `align-items`.
- Markdown link markup in a label opens its configured URL, `close_confirm` asks its question in the
  product's own skin before closing, and the folder picker writes the chosen directory back into the
  layout's TextInput.
- Controls paint the `background`, `border-color`, and `border-radius` a layout declares, and the
  language menu takes Up/Down/Enter/Escape while it is open.
- A text field can be edited in place: clicking places a blinking caret, typing inserts, Backspace
  and Delete remove, and the arrow keys/Home/End move the caret. Dragging or double-clicking selects
  text with a highlight, Ctrl+A selects all, Ctrl+C/X/V copy, cut, and paste, Ctrl+Z/Y undo and redo
  with a run of typing collapsed into one step, and Ctrl+Backspace/Delete plus Ctrl+Left/Right work a
  word at a time. `readonly` fields stay display-only.
- Flow layout resolves `flex-basis` by sharing free space from that basis, honours `align-self` per
  item, lets a nested container size itself from its own children, and positions an absolutely
  placed element from `right`/`bottom` or the `inset` shorthand.
- A container that declares `flex-wrap` moves items onto the next line when a row is full; without
  it, an overflowing row still compresses its shrinkable items.
- A setup carries an application manifest: `install.require_admin` makes Windows raise the consent
  prompt before the process starts, and `ui.dpi_aware` tells the shell whether the window scales
  itself. A project that sets neither keeps the ordinary invoker behaviour.
- The builder compares every locale file against the default locale and the keys the pages ask for,
  and reports a locale that is missing text or that `supported_locales` lists without a file.
- A finished uninstall leaves a cleaner copy in the temporary directory that immediately deletes
  the uninstaller and the emptied installation directory. A directory that still holds files the
  user added is kept, and the cleaner removes itself once it is done.
- A project that opts in with `advanced.silent_mode_support` runs an install with `--silent`, and
  one with `advanced.uninstall_mode_support` runs an uninstall the same way. A silent run opens
  nothing, takes `--dir`, refuses any other option, and reports through the exit code and standard
  error. A project that did not opt in is refused rather than installed or removed unattended.
- The setup-level suite in `crates/nano-installer-core/tests/e2e_setup.rs` builds a setup from a
  project it writes itself and runs it against a real installation: files land on disk byte for
  byte, the manifest and the uninstall entry are written, an upgrade drops stale files and keeps
  files it does not own, and an uninstall removes the product, the registration, and the directory.
  One of its fourteen cases opens the wizard window and measures the client area it drew, which
  needs an interactive desktop session, so that case skips where there is none and
  `NANO_INSTALLER_E2E_REQUIRE_DESKTOP=1` makes the skip a failure. The other thirteen pass on
  Windows 11 and in CI.
- A setup stays a setup after signing: a certificate table appended behind the bundle, which is what
  Authenticode writes into the file, no longer hides the footer the runtime reads its resources from.
- `scripts/capture_setup_snapshots.ps1` builds the example setup and photographs the page it opens in
  Chinese, English, and Russian, plus once at 150% scaling, then checks every snapshot against the
  project's own layout: the client area, the cut corners, artwork in the logo and the tagline, text
  drawn in the colour the layout declares, and a button and version line that differ per locale. It
  runs on a machine with a desktop session, and it photographs the first page only, so a clone that
  never unpacked the example's payload builds against an empty archive of the same format.

## Signing

The builder does not sign anything, and it should not. A setup and its uninstaller are signed by the
release pipeline that publishes them, exactly as an NSIS installer is signed by the product that
builds it: NSIS only offers `!finalize` and `!uninstfinalize`, hooks that hand the generated file to
a command, and signs nothing itself. `scripts/sign.ps1` does that job here, signing a file with the
certificate `NANO_INSTALLER_CERT_THUMBPRINT` names and verifying the result; the build never calls
it.

What this project owes that pipeline is an artifact that signing does not break, because Authenticode
appends its certificate table behind everything the build wrote and the bundle footer stops being the
last thing in the file. The runtime searches the tail of the file for the footer instead of reading
its final bytes, `a_setup_with_a_signature_appended_still_installs` holds that in place, and a setup
signed with signtool and a locally issued certificate was installed on Windows 11.

## Blocking a release

1. No acceptance run on a real Windows 7 SP1 machine. The suites and the snapshots run on Windows 11;
   Windows 7 SP1 is a platform a setup claims to support and the one nothing has been observed on,
   and no such machine is available at the moment.

Until that run happens, the automated results are what there is: they hold on Windows 11, and on
Windows 7 SP1 nothing has been observed.

## Known limitations

- The runtime stubs embed the Rhai engine, which raised each stub from roughly 0.57-0.66 MB to
  about 1.8-1.9 MB.
- An elevated setup and its uninstaller run at high integrity, so a product that writes only to
  `%LOCALAPPDATA%` should leave `install.require_admin` off.
- Display scaling on a multi-monitor desktop: a setup declares both `dpiAware` (read by
  Windows 7/8.1) and `dpiAwareness` (read by Windows 10 1607 and later, set to
  `PerMonitorV2, PerMonitor`). A window dragged to a display with a different scaling factor is laid
  out again for that display, so text and artwork stay sharp; on Windows 7 the system still scales
  the window for the primary display.

Once signing and the Windows 7 acceptance run are done, you can onboard a product using the
`examples/TapTap` structure.
