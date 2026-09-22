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
- A release can ship only the files whose bytes changed. `--delta-from` names the payload archive of
  the release it replaces, the builder expands both archives and compares them file by file, and what
  did not change is left to a statement in the setup: the files it expects on the machine, with their
  byte counts and digests. An install checks those before it writes anything and refuses with "run
  the full setup" when one does not match; the files it kept stay in the manifest, so an uninstall
  takes them back.
- The first page renders background bitmaps and Unicode text natively, in a window with no border
  and rounded corners that you can drag, minimize, and close; it shows a hand cursor over
  clickable controls.
- Every artifact goes through a PE import audit for Windows 7 SP1 x64 compatibility.
- Progress pages report live status, and the finish page can launch the deployed application.
- Shortcuts and autostart entries are created during install and restored on uninstall, including
  through rollback.
- Uninstall terminates a running product, removes shortcuts, and deletes declared user data only
  when you clear the keep-data option.
- The uninstall entry carries the set a Windows installation list reads: the product name,
  version, publisher, install location, uninstall command and icon; the quiet command a script
  or an administrator runs to remove the product without a window (the deployed uninstaller
  with `--silent`); the size in kilobytes as a `REG_DWORD`, counted from the bytes on disk and
  including the uninstaller itself, floored at one so that a tiny product does not read as
  "size unknown"; and `NoModify` and `NoRepair`, both `REG_DWORD` 1, because there is no
  separate modify or repair step and a button that leads nowhere is worse than no button.
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
- A project script reads and writes every type the registry keeps: text, an expandable string, a
  multi-string, a DWORD, a QWORD and binary data, and it can ask whether a value is there and what
  type the machine stored. A key may end its hive with `32` or `64`, as in `HKLM32\...` or
  `HKCU64\...`, naming the copy a 32-bit or a 64-bit program reads; the view travels in the
  recorded key, so an uninstall takes back the copy the script wrote to.
- A script that runs a program can have what it wrote: `run_command_output` answers with the exit
  code, standard output and standard error, decoded as UTF-8 where those bytes are valid and in the
  machine's own ANSI code page otherwise, so `ipconfig` on a Chinese Windows reads as Chinese
  instead of as replacement characters. `run_command` waits the same way, and both end the program,
  with what it has written so far, when the user stops the task.
- A project script can install a service of its own: `service_install` puts it in place, pointed at a
  program inside the installation, `service_exists` and `service_running` ask the machine about it,
  and `service_start`, `service_stop`, `service_set_start_type` (`auto`, `delayed`, `manual` or
  `disabled`) and `service_delete` control it. What a script installs travels in the manifest: the
  uninstall stops and deletes it before the files it runs from are removed. A service of the same
  name that runs another program is refused rather than taken over. All of it needs an elevated
  process; a plain run gets `false` and Windows' own words in the log, and `service_install`
  installs without starting anything, so the product decides when its service runs.
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
  nothing, takes `--dir` and `--log`, refuses any other option, and reports through the exit code and
  standard error. A project that did not opt in is refused rather than installed or removed unattended.
- Every install and uninstall leaves a log of itself on disk: in the `nano-installer` directory of the
  temporary directory by default, named after the setup image, the moment and which of the two tasks it
  was, or in the file a windowless run names with `--log`. It opens with the machine -- whether the run
  is elevated, the Windows version and build, the architecture, the interface language -- then the
  product and the directory it went to, and then every step the run took and every line a project script
  wrote, up to a megabyte, where it says it stopped. A run that fails leaves the file behind and names
  it -- under the error in the wizard, on standard error for a windowless run -- which is what a user
  hands over; the file is not inside the installation, so it survives the directory a failed install
  removes again.
- A project can cut its content into components: `resources.payload_file` is what every run
  installs, each entry of `components.items` carries a ZIP or 7z archive of its own, a `Checkbox`
  of the same id on a page decides whether this run installs it, and a script asks the same
  question with `is_component_selected` and `selected_components`. A component marked `required`
  installs either way, and one the page carries no checkbox for, a silent run included, follows
  its `default`. One runtime unpacks every archive, so a component in another format fails the
  build, and two archives that carry one relative path fail at install time rather than
  overwriting each other in the order the project declares them.
- A project can declare what the machine has to have first. Each entry of `dependencies.items` says
  how to recognise it (a file, or a registry value that has to exist, equal a value or be at least a
  version), how to install it (an `.exe` the setup carries, or one fetched from a URL), and whether
  missing it is fatal. The run asks before it extracts the payload: what is there is left alone,
  what is missing is installed, a required one that cannot be installed stops the run and reports
  the dependency and the installer's exit code, and an optional one leaves a warning. A downloaded
  program has to match the SHA-256 the project recorded before it runs, and is removed again when it
  does not. What lands on the machine is the dependency itself and is not part of the uninstall
  manifest, so uninstalling the product leaves what the machine was missing in place. A script that
  wants to decide when for itself asks the same declaration and takes the same fetch route through
  `dependency_installed`, `install_dependency`, `download_file`, `download_file_with_hash` and
  `sha256_of_file`.
- A project script can decide which page comes next. With `scripts/pages.rhai` in the project
  defining `next_page(from)`, the runtime hands it the id of the page the user is on every time
  Next is clicked, and it answers with the page to go to: naming one takes the wizard there and
  skips whatever the project declared in between, while an empty answer, a page the project gave
  no `id`, or no such function at all keeps the declared order. A hook can only look -- the
  queries of `system`, `ui`, `registry` and `file` plus `get_mode()` and `log_*` are registered
  for it, and it runs under an operation ceiling of its own -- a million operations, against an
  install script's hundred million -- so it cannot change the machine, and a hook that fails or
  names a page that does not exist does not
  trap the user: the reason lands on the product's own card and the declared order walks on.
  Back walks the way the user came, so a page the hook skipped does not reappear; starting,
  returning from or ending a task clears that trail. A page id has to be unique inside one page
  list, and the build refuses a project that gives two pages the same one.
- The setup-level suite in `crates/nano-installer-core/tests/e2e_setup.rs` builds a setup from a
  project it writes itself and runs it against a real installation: files land on disk byte for
  byte, the manifest and the uninstall entry are written, an upgrade drops stale files and keeps
  files it does not own, and an uninstall removes the product, the registration, and the directory; a
  dependency the machine is missing is really installed, and a downloaded one is checked before it
  runs. What the setup itself carries is checked too: every bundle entry is read against the SHA-256
  the build recorded, so a setup damaged after the build is refused by entry name and nothing is
  created on the machine at all.
  Sixteen of its fifty-four cases open the wizard window and drive it: one measures the client
  area it drew, one walks the page actions a project declares, one stops a running task from a cancel
  button, one types a directory into the field a page asks for and starts the install with it, one
  clicks the row a radio group's install button waits for, one rolls the wheel over a list and
  clicks the row it brings into view, one reads the text the user typed into a page and the row a
  choice group was left on into the script, one ticks a component's checkbox and clears another's
  and reads back which of them landed, one answers the card a project script puts up and
  reads back what the script wrote after each answer, one moves the pointer onto a button and
  holds it down and reads the three pictures the button declares back out of the window, one
  reads which of the three standard pointers the window answers with over a button, a field and
  the page, one walks the language menu with the arrow keys, Enter and Escape, one walks the page's
  controls with Tab, acts on the one the focus ring is on with Space or Enter and comes round to
  the first control from the last, and one clicks a
  browse button and closes the shell's folder dialog again, one lets a page hook send the wizard
  past the licence page and Back the way the user came, and one watches a hook fail: the reason
  lands on the product's own card and the declared order walks on. They need an interactive
  desktop session, so they skip where there is none and `NANO_INSTALLER_E2E_REQUIRE_DESKTOP=1` makes the
  skip a failure; the cursor case asks that the session be showing a pointer as well, which a
  hosted runner is not, and it prints its skip there. Of the other thirty-eight, three read their
  answer back out of the machine rather than out of the primitive that wrote it: one checks every
  registry type a script named, and the copy of a key a view name selects, one checks the exit code
  and both streams of a command a script ran, and one checks that the service a script installed is
  really on the machine and gone again with the uninstall. The rest pass on Windows 11 and in CI.
- A setup stays a setup after signing: a certificate table appended behind the bundle, which is what
  Authenticode writes into the file, no longer hides the footer the runtime reads its resources from.
- A project's own command can run on the finished files, which is NSIS's `!finalize` and
  `!uninstfinalize`: the builder calls `finalize.uninstaller` while the uninstaller is still a file of
  its own, before the setup embeds it, and `finalize.installer` once the setup is complete and
  closed. `%1` in the command stands for that file's path, every line it prints goes into the build
  log, and a non-zero exit code stops the build and leaves the refused setup off disk. Signing is
  what that is usually for, and the builder signs nothing itself.
- What the setup carries is checked entry by entry. Every bundle entry records the SHA-256 the build
  computed for it, and a read is checked against that: a truncated download, a bad sector, or a hand
  that changed the file stops the run before anything is unpacked, names the entry and both digests,
  and sends the user for a fresh copy of the setup instead of installing bytes nobody vouched for. A
  payload streamed out of the setup is hashed as it goes past, and a copy that fails the check is
  removed, so the step that would have unpacked it has nothing to run against.
- A large payload costs no memory. A setup is made by appending the payload to itself in 1 MiB blocks, and
  each block goes both into the file and into the SHA-256 the bundle records for it, so the peak working
  set does not follow the payload: measured here, a payload from 8 MiB to 1024 MiB took from 1.5 s to
  4.7 s to build while the peak working set stayed at 2.5 MiB in all four sizes.
  `scripts/measure_build.ps1` reproduces those numbers; see [Build and
  release](BUILD_AND_RELEASE.md#build-time-and-memory).
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

## Signing

The builder does not sign anything, and it should not. A setup and its uninstaller are signed by the
release pipeline that publishes them, exactly as an NSIS installer is signed by the product that
builds it: NSIS only offers `!finalize` and `!uninstfinalize`, hooks that hand the generated file to
a command, and signs nothing itself. Those two hooks have their counterparts here now: a project
writes its commands into `finalize.installer` and `finalize.uninstaller`, the builder calls them on
the uninstaller while it is still a file of its own and on the setup once it is complete, every line
they print goes into the build log, and a non-zero exit code stops the build and deletes the setup it
refused. `scripts/sign.ps1` is how that command is usually written: it signs with the certificate
`NANO_INSTALLER_CERT_THUMBPRINT` names and verifies the result. The division of labour has not moved
-- the certificate and its key stay in the pipeline.

What this project owes that pipeline is an artifact that signing does not break, because Authenticode
appends its certificate table behind everything the build wrote and the bundle footer stops being the
last thing in the file. The runtime searches the tail of the file for the footer instead of reading
its final bytes, `a_setup_with_a_signature_appended_still_installs` holds that in place, and a setup
signed with signtool and a locally issued certificate was installed on Windows 11. The path through
the real hooks was walked end to end as well: a probe project pointed both `finalize` settings at
`scripts/sign.ps1`, and the setup it produced and the uninstaller extracted from that setup both pass
`signtool verify /pa` (with a DigiCert timestamp); `scripts/audit_embedded_uninstaller.ps1` still
finds the footer, extracts the uninstaller and matches its digest on the signed setup; and a signed
setup installs, registers its uninstall entry and removes itself again.

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

- A dependency is downloaded through the WinHTTP stack Windows ships, with TLS 1.1/1.2 turned on for
  https. On Windows 7 SP1 that also needs the machine's own schannel to support TLS 1.2 (KB3140245
  and later); without it the download fails and says so rather than falling back to plain text. An
  http URL is unaffected.

Once signing and the Windows 7 acceptance run are done, you can onboard a product using the
`examples/TapTap` structure.
