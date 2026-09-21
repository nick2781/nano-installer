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
- A message, an error and a question raised by a project script (`show_message`, `show_error`,
  `ask_yes_no`) appear in the wizard window the way the product draws its own, and the button a
  person clicks is the answer the script carries on with. A run with nothing to draw in keeps the
  system box.
- Controls paint the `background`, `border-color`, and `border-radius` a layout declares, and the
  language menu takes Up/Down/Enter/Escape while it is open.
- A text field can be edited in place: clicking places a blinking caret, typing inserts, Backspace
  and Delete remove, and the arrow keys/Home/End move the caret. Dragging or double-clicking selects
  text with a highlight, Ctrl+A selects all, Ctrl+C/X/V copy, cut, and paste, Ctrl+Z/Y undo and redo
  with a run of typing collapsed into one step, and Ctrl+Backspace/Delete plus Ctrl+Left/Right work a
  word at a time. `readonly` fields stay display-only.
- A `TextInput` can carry the rules a project puts on the value it is given: `required`,
  `min-length` and `max-length` counted in characters, and `pattern`, a mask where `*` is any run of
  characters, `?` exactly one, matched against the whole value. A field with no rules passes, and so
  does an optional field the user left empty. `enabled-when` names such a field with `valid` or
  `invalid`, and a list of conditions separated by commas all have to hold. A `Label` whose
  `value-source` is `field-error:<field>` draws the message the page wrote for the first rule the
  value breaks, looked up in the project's own language, and draws nothing while the value is
  acceptable. An empty field still takes the caret, which is what lets a page ask for a directory
  and wait for it before the install starts.
- A `Select` is a choice the page offers, not only the language control: it lists its `<Option>`
  children, shows the words of the option in use, and records the `value` of the row a person clicks.
  A `RadioButton` belongs to a `group`, stands for one `value` in it, and marks the row its group
  starts on with `checked="true"`, so a click on any row replaces the value for the whole group.
  `enabled-when="<id>:<value>"` reads either of them, with the select's own id or the group's name,
  and the menu takes Up/Down/Enter/Escape the way the language list already did.
- Flow layout resolves `flex-basis` by sharing free space from that basis, honours `align-self` per
  item, lets a nested container size itself from its own children, and positions an absolutely
  placed element from `right`/`bottom` or the `inset` shorthand.
- A project can bundle the helper programs its script runs: `resources.tools_dir` puts that
  directory into the setup as it stands, subdirectories included, and `get_tools_dir()` unpacks it
  beside the script and returns the path. A project that bundles none, and a script asking a setup
  built without them, get an empty string and a log warning rather than a failed install.
- A project script can write one of the user's environment variables (`set_env`, `remove_env`) and
  claim a file type (`register_file_association`, `unregister_file_association`). Both go through
  the manifest: the install records the value it wrote or the keys it created, and the uninstall
  takes those keys and values back. `Environment` is recorded as a value, so PATH survives an
  uninstall.
- A container that declares `flex-wrap` moves items onto the next line when a row is full; without
  it, an overflowing row still compresses its shrinkable items.
- A flow container that declares `scrollable="true"` and an `id` keeps the size it was given and
  lays its children out at the size they ask for, cutting off whatever does not fit: a row the list
  has scrolled past is neither drawn nor clickable, so a button below the list answers the click it
  would otherwise have taken. The wheel moves it, 48 pixels a notch, and the scrollbar along the
  trailing edge says which part of the list is in view; a click on either side of its thumb pages
  the view by one of its own extents.
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
  Eleven of its twenty-nine cases open the wizard window and drive it: one measures the client area
  it drew, one walks the page actions a project declares, one stops a running task from a cancel
  button, one types a directory into the field a page asks for and starts the install with it, one
  clicks the row a radio group's install button waits for, one rolls the wheel over a list and
  clicks the row it brings into view, one answers the card a project script puts up and reads back
  what the script wrote after each answer, one moves the pointer onto a button and holds it down
  and reads the three pictures the button declares back out of the window, one reads which of the
  three standard pointers the window answers with over a button, a field and the page, one walks
  the language menu with the arrow keys, Enter and Escape, and one clicks a browse button and
  closes the shell's folder dialog again. They need an interactive desktop session, so they skip
  where there is none and `NANO_INSTALLER_E2E_REQUIRE_DESKTOP=1` makes the skip a failure; the
  cursor case asks that the session be showing a pointer as well, which a hosted runner is not, and
  it prints its skip there. The other eighteen pass on Windows 11 and in CI.
- A setup stays a setup after signing: a certificate table appended behind the bundle, which is what
  Authenticode writes into the file, no longer hides the footer the runtime reads its resources from.
- The builder refuses a configuration key it does not read. A setting that once parsed and then
  changed nothing cannot ship as if it were doing its job; the message names the key and the
  setting that takes its place.
- A task that is running can be stopped. A `cancel` button, or the close question answered with
  Yes, makes the task give up at the checkpoint after the step it is on and undo what it wrote,
  rather than leave a half-installed product behind, and a project script sees the same request
  through `is_cancelled`. `is_cancelled` used to answer `false` for ever.
- `scripts/capture_setup_snapshots.ps1` builds the example setup and photographs every page the
  project declares: the wizard's first page in Chinese, English, and Russian, plus one at each higher
  scaling (150% and 200% by default), and one picture of each of the other pages. Every snapshot is then checked against its own
  page: the client area, the corners of a page that declares a rounded one, every image and label the
  layout places by absolute coordinates, the page's colours, and that no two pages came out as the
  same picture -- plus, on the first page, a button and version line that differ per locale. It runs
  on a machine with a desktop session, and it places the page it wants first for each build, writing
  the project's configuration back byte for byte, so a clone that never unpacked the example's payload
  builds against an empty archive of the same format. A page is drawn rather than reached: the
  uninstaller's pages are in the pictures without an uninstall having run.

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
