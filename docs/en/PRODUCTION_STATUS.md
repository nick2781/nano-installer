# Production status

**Windows 7 SP1 x64, the minimum these artifacts claim, has been through one real run**: build 7601,
in a virtual machine thrown away afterwards, where a setup the current builder produced was installed,
uninstalled, upgraded, and run twice without a window. What the run reached is in
[Verified](#verified), and what it did not reach follows in a section of its own, rather than sitting
there looking tested. The run also found and fixed three defects: a folder directly below a drive root
was refused as an installation target (`--dir C:\MyApp` failed with "install path cannot be a drive
root"), a silent install started from an administrator console failed on an invalid handle, and a
project driving its own install from a script could not install over its own previous version.

Installing writes files and registry entries, so a new project's setup is still worth trying in a
virtual machine you throw away afterwards, and that goes for any installer.

## Verified

- The minimum version has been through a real run: build 7601 Ultimate, in a virtual machine thrown
  away afterwards, with a standard VGA adapter and a USB pointer; a setup the current builder produced
  installs and uninstalls there. Of the nine items in
  [Windows compatibility](WINDOWS_COMPATIBILITY.md#running-the-acceptance), all but the screen reader,
  the window dragging and the service were reached: the window has no title bar and rounded corners,
  and it draws its background bitmap, its icons and Chinese text from the page layouts. With
  `install.require_admin` on, Windows asks for administrator rights and shows "unknown" as the
  publisher, because nothing is signed; until the agreement is ticked, the install button is drawn in
  grey from `disabled-image`, and once it is ticked the button switches to its enabled state, changing
  8,994 of the 9,600 pixels in the same 240 by 40 area. The application files land with their nested
  directories, a desktop shortcut and a Start menu shortcut are created, one for the product and one
  for the uninstall, and the uninstall entry carries every field Windows' installed-programs list wants.
  The project's own `install.rhai` created its `logs` directory, wrote its registry values, and closed
  the previous version that was still running; `launch_app` on the finish page started the program that
  had just been installed, and the program wrote `launched.txt` into its own directory. The uninstall
  closed the running product and removed the application files, both shortcuts and both registry keys.
  `--silent` installs and uninstalls both exited 0 and left a log where `--log` asked for it, naming the
  system version and bitness, the interface language, whether the run was as administrator, the product
  and its path, and what each step did. An upgrade was walked on the same machine: a second version
  installed straight over the first, files the new version no longer ships were removed, files only it
  has landed, shared files were replaced, the version in the uninstall entry changed, and an uninstall
  then took away everything the upgrade had put down. The run also found and fixed three defects (see
  the [changelog](../../CHANGELOG.md)): a folder directly below a drive root being refused, a silent
  install from an administrator console failing on an invalid handle, and a script-driven project being
  unable to install over its own previous version. That last one came out of this upgrade walk, and the
  walk was repeated here after the fix.
- The builder and the runtime are two separate programs, so a setup never contains the builder.
- ZIP and 7z application files are recognised from the file signature, and then handed to the matching runtime.
- Both runtimes unpack real archives, and the result matches the expected SHA-256.
- The uninstaller has its own runtime and page list, and links no archive backend.
- The CLI and the Windows 10+ GUI drive the same build API, while GUI dependencies stay out of the runtime.
- A setup carries the application files, the project resources, and a self-contained uninstaller with its own icon and version information.
- Installing unpacks the application files, puts the files and the uninstaller into a new directory, and writes the manifest file and the uninstall entry; uninstalling removes only what the manifest file records.
- When the target directory already holds the same project, the install upgrades in place: files the new application files no longer carry are removed, and a failure in any step restores the version from before the upgrade.
- A release can ship only the files whose bytes changed: at build time `--delta-from` names the application-file archive of the release it replaces, the builder expands both archives and compares them file by file, and what did not change is left as one statement in the setup.
  An install checks those files on the machine first: are they there, their byte counts, their digests. A mismatch refuses the run and sends you for the full setup, while only a pass lets it write. Files it kept stay in the manifest file, and the uninstall takes them back.
- The first page draws background bitmaps and Unicode text natively; the window has no border and rounded corners, you can drag it, minimize it and close it, and a hand cursor appears over anything clickable.
- Every artifact has its PE imports checked for compatibility with the minimum supported version.
- Progress pages report live status, and the finish page can start the program that was installed.
- Shortcuts and autostart entries are created during install and restored on uninstall, and rollback does the same.
- Uninstall ends a running product and removes shortcuts, but it deletes declared user data only when you clear the keep-data option.
- The uninstall entry carries the fields a Windows installed-programs list wants: product name, version, publisher, install location, uninstall command and icon; the command a script or an administrator uses for a silent uninstall is the deployed uninstaller plus `--silent`.
  The size is counted from the bytes on disk and stored as kilobytes in a `REG_DWORD`, with the uninstaller itself included, so a product under 1 KiB reports 1 rather than 0, which would read as an unknown size.
  `NoModify` and `NoRepair` are both `REG_DWORD` 1; this installer has no separate modify or repair step, so those two marks keep Windows from putting up a button that leads nowhere, which is worse than no button.
- An embedded Rhai engine runs `scripts/install.rhai` and `scripts/uninstall.rhai`, and the primitives a script uses reuse the built-in deployment, rollback and manifest file code. A script that fails rolls back, and when a script skips manifest file cleanup, the library does it.
- Flow layout covers nested `VBox`, `HBox` and `Content` containers, with padding, margins, percentage sizing, `justify-content` and `align-items`.
- A Markdown link in a label opens its configured URL when clicked, the confirmation `close_confirm` puts up carries the product's own skin, and the folder picker writes the chosen path back into the layout's TextInput.
- A prompt, an error and a question from a project script are drawn in the wizard window too, the way the product's own dialogs look: `show_message`, `show_error`, `ask_yes_no`, and the button you click on the card is the answer the script carries on with. A run with nothing to draw in still uses the system dialog.
- Controls are painted from the `background`, `border-color` and `border-radius` a layout declares; once the language menu is open, the Up/Down keys, Enter and Escape all work.
- A text field can be edited directly. A click drops a blinking caret, typing inserts, Backspace and Delete remove characters, and the arrow keys, Home and End move the caret. Dragging with the left button held or double-clicking selects text with a highlight. Ctrl+A selects all, Ctrl+C/X/V copy, cut and paste, Ctrl+Z/Y undo and redo, Ctrl+Backspace/Delete and Ctrl+Left/Right work a word at a time, and a run of typing collapses into one step. A field declared `readonly` stays display-only.
- A `TextInput` can carry the rules a project puts on its value in the layout: `required`, `min-length` and `max-length` counted in characters, and `pattern`, which is a mask: `*` is any run of characters, which may be empty, `?` is exactly one, and the whole value has to match.
  A field with no rules always passes, and a field that may be left empty passes when it is empty. `enabled-when` names such a field with `valid` or `invalid`, with conditions separated by commas, all of which have to hold.
  A `Label` whose `value-source` is `field-error:<field id>` draws the message the page wrote for the first rule the value breaks, looked up in the project's own language, and it draws nothing while the value passes.
  An empty field still takes the caret, so a page can ask for a directory and let the install start only once you fill it in.
- A `Select` is a choice the page offers, not only the language control: it lists its `<Option>` children, shows the words of the current option, and records the `value` of the row you click.
  A `RadioButton` belongs to a `group` and stands for one `value` in it, `checked="true"` marks the row the group starts on, and a click on any row replaces that value for the whole group.
  `enabled-when="<id>:<value>"` reads either of them, with the select's own name or the radio group's name; once the menu is open, Up/Down, Enter and Escape work as they do in the language list.
- Flow layout takes `flex-basis` as the starting point and shares out the space that is left, and every element can set `align-self`. A nested container is measured by what is inside it, while an absolutely placed element can be positioned from the other side with `right`/`bottom` or the `inset` shorthand.
- A project can bundle the helper programs its script runs: the directory `resources.tools_dir` names goes into the setup as it stands, subdirectories included, and a script calls `get_tools_dir()` to release it to disk and get the path.
  A project that bundles no tools, and a script asking a setup built without them, get an empty string and a log warning, and the install carries on.
- A project can ship plugins of its own: a plugin is a 64-bit DLL built against `include/nano_plugin.h` and placed in `resources.plugins_dir`, and a script calls it with `plugin_call("dll::function", [...])` and reads back the text values it pushes.
  What a plugin writes through the host's `write_file` and `write_registry` travels with the manifest file, and the uninstall takes it back, while a write outside the installation is refused there and then.
  A DLL this runtime could never load is refused at build time: 32-bit, ARM64, not a DLL, not an image. The build names which one, so finding that out half way through an install is too late.
- A project script can write one of the current user's environment variables, with `set_env` and `remove_env`, and it can claim a file type with `register_file_association` and `unregister_file_association`. Both go through the manifest file: the install records the values it wrote and the keys it created, and the uninstall takes back only those.
  `Environment` is recorded as a value rather than a key, so PATH is not deleted with it.
- A script reads and writes every type the registry keeps: text, an expandable string, a multi-string, a DWORD, a QWORD and binary data, and it can also just ask whether a value is there, and what type the machine stored it as.
  A key name may carry `32` or `64` after the hive, as in `HKLM32\...` or `HKCU64\...`, naming the copy a 32-bit or a 64-bit program reads; the view is recorded with that key in the manifest file, so the uninstall takes back exactly the copy the script wrote.
- What a program a script ran wrote can be read back: `run_command_output` gives the exit code, standard output and standard error, and the bytes are decoded as UTF-8 first, in the machine's ANSI code page when that fails, so `ipconfig` on a Chinese Windows reads as Chinese instead of replacement characters.
  `run_command` waits the same way, and when you stop the task, both primitives end the program along with what it has written.
- A project script can install a service of its own: `service_install` installs it, pointed at a program inside the installation, while `service_exists` and `service_running` ask the machine.
  `service_start`, `service_stop`, `service_set_start_type` and `service_delete` control it, with `auto`, `delayed`, `manual` and `disabled` as the start types.
  What a script installs travels with the manifest file: the uninstall stops it first and then deletes it, before the files go, because the program it runs is inside the installation; a service of the same name running another program is refused, and the uninstall will not delete someone else's service.
  All of this needs administrator rights, and without them the call returns `false` and writes Windows' own words into the log. `service_install` only installs, so the script decides when the service runs.
- A container that declares `flex-wrap` moves onto the next line when a row is full, and by default a row still compresses its shrinkable items first.
- A flow container that declares `scrollable="true"` and gives an `id` keeps the size it was given, lays its children out at the sizes they declare, and cuts off whatever does not fit as a whole; a row the list has scrolled past is neither drawn nor registered for clicks, so a button below the list answers the click it would otherwise have taken.
  The wheel moves it 48 pixels a notch, the scrollbar along the trailing edge shows which part of the list is in view, and a click on either side of the thumb pages by one screen.
- A setup writes an application manifest file: with `install.require_admin` on, Windows raises the administrator prompt (UAC) before the process starts, while `ui.dpi_aware` tells the system whether the window scales itself, and a project that sets neither runs like an ordinary program.
- The builder compares every locale file against the default locale and the keys the pages actually ask for, reporting a locale that is missing text and one `supported_locales` lists without a file.
- Once an uninstall is done, the cleaner copy in the temporary directory immediately deletes the uninstaller and the emptied installation directory, but a directory that still holds files of yours is kept. The cleaner also exits and removes itself when it is done.
- A project that declares `advanced.silent_mode_support` installs with `--silent`, and one that declares `advanced.uninstall_mode_support` uninstalls the same way. A silent run opens no interface at all, takes only `--dir` and `--log`, refuses any other argument, and reports through the exit code and standard error; a project that did not declare it is refused outright, rather than installed or removed quietly.
- Every install and uninstall leaves a log of the run on disk, by default in the `nano-installer` directory of the temporary directory, and the file name carries the setup's name, the moment, and which of the two tasks this was; a silent run can name where with `--log`.
  The log opens with the machine: whether it ran as administrator, the Windows version and build, the architecture, and the interface language; then the product name and version and the directory it went to; then every step the run took and every line a script wrote, up to 1 MiB, where it stops and says it was truncated.
  A failed run leaves the file where it is: the wizard writes its full path under the error, and a silent run writes it to standard error, so you or your operations team can just hand it over. The file is not inside the installation directory, so it survives the directory a failed install takes back.
- A project can cut what it installs into components: `resources.payload_file` is the base application files every run installs, and each entry of `components.items` carries a ZIP or 7z archive of its own.
  A `Checkbox` of the same name on a page decides whether this run installs it, and a script asks the same question with `is_component_selected` and `selected_components`.
  A component marked `required` always installs, and when the page has no such checkbox, a silent run included, `default` decides. One runtime unpacks every archive, so a format that does not match is refused at build time; two archives carrying one relative path fail at install time, rather than overwriting each other in declaration order.
- A project can declare what the machine has to have first: each entry of `dependencies.items` says how to recognise it, how to install it, and whether missing it is fatal.
  How to recognise it: a file, or a registry value that has to exist, equal some value, or be at least some version; how to install it: an `.exe` the setup carries, or one fetched from a URL.
  The install asks once before it unpacks the application files: what is there is left alone and what is missing is installed. A required one that cannot be installed stops the run and reports the dependency name and the installer's exit code, while an optional one leaves a warning. A downloaded program has to match the SHA-256 the project recorded before it may run, and when it does not, it is deleted there and then.
  What lands on the machine is the dependency itself, and it is not in the uninstall manifest file, so after the product is removed, what the machine was missing is still there. When a script wants to decide the timing itself, `dependency_installed`, `install_dependency`, `download_file`, `download_file_with_hash` and `sha256_of_file` ask the same declaration and take the same fetch path.
- A project script can decide which page comes next: with `scripts/pages.rhai` in the project defining `next_page(from)`, the runtime hands it the id of the current page every time you click Next, and it answers where to go.
  Naming a page takes the wizard there, and a page the project declared in between is skipped; an empty answer, a page the project gave no `id`, or no such function at all still follows the declared page order.
  A hook can look but not touch: it registers only the queries of `system`, `ui`, `registry` and `file`, plus `get_mode()` and `log_*`, and installing files, writing the registry and putting up cards are not among them. It also has an operation ceiling of its own: a million operations, against an install script's hundred million, because this click is handled on the thread that draws the window.
  A hook that fails, or names a page that does not exist, does not trap anyone: the reason is drawn on the installer's own card, and the wizard goes on in the declared page order.
  `back` walks the way you came, and a page that was skipped does not appear just because Back was pressed, while starting a task, a task failing on return, or a task ending clears that trail. A page id has to be unique in one page list, and a project that gives two pages the same id is refused at build time.
- The setup-level cases in `crates/nano-installer-core/tests/e2e_setup.rs` write their own project, build a setup and really install it: files land on disk byte for byte, the manifest file and the uninstall entry are written, an upgrade removes old files and keeps files that are not its own, and an uninstall clears the product, the registration and the directory. A dependency the machine is missing is really installed, and a downloaded one passes a hash check before it runs. What the setup itself carries is checked entry by entry too, and a setup that was changed does not even create the target directory.
- A build can wrap the finished setup in the installer package an enterprise deploys: `--msi` writes a `.msi` that carries exactly the setup this build produced, so installing with it installs that setup silently, and an uninstall runs the uninstaller it deployed to take the product away.
  The package was driven on this build machine with the installer Windows itself ships: `msiexec /i <package> /qn INSTALLDIR=<directory>` exited 0, and the product exe, the uninstaller and the manifest file all landed in that directory, with the product's own uninstall entry naming it; `msiexec /x <package> /qn` exited 0, and neither the directory nor the entry was left.
  A second package built from the same project at a higher version upgraded what the first one installed: the uninstall entry reported the new version, the older package had nothing left to remove, and the newer one cleaned everything up. A project that never declared a silent run is refused a package, by name of the setting it is missing.
  Twenty-five of its seventy-four cases open the wizard window and drive it: one measures the client area it drew, one walks the page actions a project declares, one stops a running task from a cancel button, and one types a directory into the field a page asks for and starts the install with it.
  One clicks the row a radio group's install button waits for, one rolls the wheel one notch over a list and clicks the row it brings into view, one clicks the card a project script puts up and reads what the script wrote back after each answer, and one reads the words typed into the page and the selected row into the script.
  One ticks one component in the window, clears another that was ticked, and looks at which of them landed; one moves the pointer onto a button and holds it down, then reads the three pictures the button declares back out of the window; and one reads which of the three standard pointers the window answers with over a button, a field and the empty page.
  One walks the language menu with the arrow keys, Enter and Escape. One walks the page's controls with Tab in layout order, acts on the control the focus ring is on with Space or Enter, and comes round to the first from the last with one more Tab.
  One runs the high-contrast switch each way and checks which set of colours the page, the card, the card's edge and the focus ring use, while the picture a checkbox carries stays the same either way. One more opens the browse button and closes the shell's folder dialog again.
  Another lets `scripts/pages.rhai` send the wizard past the licence page in the middle, and Back brings you back to the welcome page you came from; one lets a hook fail once and watches the wizard write the reason on the product's own card and then walk on in the declared order.
  Eight more hand the page to a client in another process. The client asks for the list of controls with their roles, names, values, states and places. One case works a control through its default action, and another has the keyboard handed to a different control on request. As Tab walks on, the focus events the window announces are handed over too. The value being typed into a field is handed over as well, reported once per character and not for a key that only moves the caret. The words a task reports and the progress bar, the message a field that breaks a rule puts up, and the card a question is drawn on all go the same way. And the same page asked for by member name over `IDispatch` gives the same answers as the interface cases. One of them keeps reading the page to see whether the wizard falls over because someone is asking.
  Opening a window needs an interactive desktop session; on a machine with no desktop they skip, and `NANO_INSTALLER_E2E_REQUIRE_DESKTOP=1` turns the skip into a failure. The cursor case also asks that the session be showing a pointer; a hosted runner does not, and it prints its skip there too.
  The other forty-nine open no window. Three of them read their answer back out of the machine rather than out of the primitive that wrote it: one checks every registry type a script named, and which copy of a key a view name selects; one checks the exit code and both output streams a command a script ran left behind; one checks that the service a script installed really is on the machine and is gone again with the uninstall. The rest pass on Windows 11 and in CI.
- A signed setup is still a setup: the certificate table is appended after the bundle data, which is what Authenticode writes when a pipeline signs, and it no longer hides the footer the runtime reads its own resources from.
- A project's own command can run on the finished files, which is NSIS's `!finalize` and `!uninstfinalize`. While the uninstaller is still a file of its own and not yet embedded in the setup, the builder calls `finalize.uninstaller`; once the setup is completely written and the file is closed, it calls `finalize.installer`. `%1` in the command becomes that file's path, every line it writes goes into the build log, and a non-zero return stops the build, so the refused setup does not stay on disk either. Signing is what it is usually for, and the builder signs nothing itself.
- What the setup carries is checked entry by entry. Every entry in the bundle data carries the SHA-256 computed at build time, and a read is checked against it: a truncated download, a bad sector, or a hand that changed something stops the run before anything is unpacked, reports the recorded value and the actual digest by entry name, and sends you for a fresh setup rather than installing bytes that do not match. A payload streamed out of the setup is hashed as it is read. On an error part way through, or when the digest does not match, that copy is deleted, so the step that would have unpacked it has nothing to run against.
- Large application files cost no memory. A setup is made by appending the application files to itself in 1 MiB blocks, and each block goes both into the setup and into the SHA-256 recorded in the bundle data. So the peak working set does not grow with the application files. Measured on this machine, application files from 8 MiB to 1024 MiB took from 1.5 s to 4.7 s to build, and the peak working set was 2.5 MiB in all four sizes. `scripts/measure_build.ps1` reproduces those numbers; see [Build and release](BUILD_AND_RELEASE.md#build-time-and-memory).
- The builder refuses a configuration key it does not read, so a setting that once parsed and then did nothing cannot ship as if it were in effect. The message names the key and says which setting to use instead.
- A task that is running can be stopped: the `cancel` button, or answering Yes to the close question, makes the task give up at the checkpoint after the step it is on and take back what it wrote, rather than leave a half-installed product on the machine. A project script sees the same request through `is_cancelled`, which used to answer `false` for ever.
- `scripts/capture_setup_snapshots.ps1` builds the example setup and photographs every page the project declares: the wizard's first page in Chinese, English and Russian, one more at each higher scaling, 150% and 200% by default, and one of each other page.
  Every snapshot is then checked against the project's own layout: the client area, whether the corners are cut on a page that declares rounded ones, whether every image and label placed by absolute coordinates drew something, and the number of colours on the page. It also checks that no two pages came out the same, and on the first page whether the button and the version line follow the language.
  It runs on a machine with a desktop session. For each page it puts that page at `wizard.pages[0]`, builds, and writes the project back byte for byte afterwards, so a clone that never unpacked the example application files builds against an empty archive of the same format; a picture shows what the layout draws, not a flow that walked to that step.
- Moving off NSIS is written down and checked. `docs/{zh-CN,en}/MIGRATION_FROM_NSIS.md` sorts every NSIS command, directive and `${...}` macro into five answers: a configuration item or a page element matches it directly, it has to become a script primitive, only a person can handle it, there is nothing to migrate, or the table has no row for it yet. It says what each one becomes.
  `examples/nsis-migration/` is an example written to that guide, with a configuration, seven pages of layout, two languages, and both install and uninstall scripts. The real builder turns it into a setup that installs.
  `scripts/check_nsi_migration.ps1` reads both guides' tables on CI and holds every construct of two fixtures to a row: the example's own script, plus `instructions.nsi`, which carries one line for every instruction, attribute and header macro NSIS documents. So the table is held to the language itself, and not only to the script beside it. The two languages agree line for line, and the example configuration goes through the builder's configuration audit as well.

## Signing

The builder does not sign anything, and it should not. A setup and its uninstaller are signed by the
release pipeline that publishes them, exactly as an NSIS installer is signed by the product that
builds it. NSIS only offers `!finalize` and `!uninstfinalize`, two hooks that hand the generated file
to a command, and it signs nothing itself.

Those two hooks have their counterparts here: a project writes its commands into `finalize.installer`
and `finalize.uninstaller`, and the builder calls them on the uninstaller while it is still a file of
its own, and on the setup once it is completely written. Every line they print goes into the build log,
and a non-zero exit code stops the build and deletes the setup it refused.

`scripts/sign.ps1` is how that command is usually written: it signs with the certificate
`NANO_INSTALLER_CERT_THUMBPRINT` names, and checks the result. The division of labour has not moved:
the certificate and its private key stay in the pipeline.

What this project owes that pipeline is "still usable after signing". Authenticode appends its
certificate table behind everything the build wrote, so the bundle footer is no longer the end of the
file. The runtime now searches the tail of the file for that footer, instead of reading the last few
bytes. `a_setup_with_a_signature_appended_still_installs` pins that behaviour down: a setup signed with
signtool and a locally issued certificate installs on Windows 11.

The path through the real hooks was walked end to end too. A probe project pointed both `finalize`
settings at `scripts/sign.ps1`. The setup it produced and the uninstaller extracted from that setup
both pass `signtool verify /pa`, with a DigiCert timestamp. `scripts/audit_embedded_uninstaller.ps1`
still finds the footer, extracts the uninstaller and matches its digest on the signed setup. A signed
setup installs, registers its uninstall entry, and removes itself cleanly.

Once it is signed, `scripts/verify_signing.ps1` checks it: it reads the setup and the uninstaller inside it,
reports who signed each one and whether it carries a timestamp, and fails on an untrusted chain or a missing
timestamp. **A timestamp is not optional** -- it is what keeps a short-lived certificate's signature valid
after the certificate expires, and a test certificate needs `-AllowUntrustedRoot` for the check to get that
far. This script belongs to the pipeline that ships a product; this repository's own artifacts are unsigned.

What signing changes was measured once. With the example setup signed by a self-signed
certificate and that certificate's root trusted on the test machine, the Windows prompt went from
"an unknown publisher" to **"Verified publisher: nano-installer signing test"** (the certificate's
subject). Only a chain that ends in a trusted root counts: the Trusted Publishers store plays no part,
and putting a root into the trusted root store **takes a human confirmation**, so a test image is where
that happens once. For a real certificate, SignPath Foundation signs open-source projects for free,
without personal identity validation and with the private key held in their HSM.

## Publishing

A release is built and published from a tag, and the tag is pushed by hand. Two things the pipeline
**checks** rather than assumes:

- **The commit the tag names has a green suite behind it.** A run in this repository is often replaced
  by the next push, so a commit's last word is often cancelled rather than a verdict. The release job
  asks GitHub for the runs on that commit. It refuses to build unless the newest **whole-suite** run
  succeeded. That is `scripts/audit_release_evidence.ps1`, and it runs before anything expensive.
  `ci.yml` does not run tests for tag pushes. So that run is `main`'s own push run, and a run the tag
  itself started cannot shut the gate. A branch's suite is the one its pull request ran, and the
  required checks on `main` read exactly that one.
  A push run is always the whole suite. A dispatched run counts as whole only when it was not given
  targets of its own, which the name of the job that runs them records: `ci.yml` appends `(narrowed)`
  to that job's name when `suite_command` is set. When the workflow file on the commit predates that
  marker, no dispatched run of it counts at all.
  So a run that stops answering leaves no green evidence. The way back is to dispatch the whole suite
  on the tag itself: `gh workflow run ci.yml --ref <tag>`. It needs no extra commit, and the gate opens
  once that run is green. What is known about that, and what the suite does about it, is in the
  changelog.
- **The five assets are published with their digests.** `scripts/release_manifest.ps1` writes
  `SHA256SUMS.txt`, one line per file, in the form `sha256sum -c` reads. The workflow attaches it to
  the release and to the artifact.
  Downloading is the one step of an install nobody can check afterwards, so a product that fetches a
  dependency already requires the digest to match.

The setup a project builds is not one of those assets: whoever ships the product builds and publishes
it, and the two hooks described under [Signing](#signing) are where that pipeline signs it.

## Blocking a release

1. Neither the setup nor the uninstaller has a code signature, so Windows warns about an unknown
   publisher. The builder does not sign, and should not. The pipeline that publishes them signs,
   through the `finalize.installer` and `finalize.uninstaller` hooks, see [Signing](#signing). Once
   that is in place, a product can plug in the way `examples/TapTap` does.

The minimum supported version has been through a real run (see [Verified](#verified)). What that run
did not cover is in the next section.

## Not covered

The gaps this run left, written down rather than left looking tested:

- The screen reader (item 8 of [the real-machine checklist](WINDOWS_COMPATIBILITY.md#how-to-run-the-acceptance)):
  Narrator and NVDA were never actually running and listened to; an MSAA client read the wizard instead,
  with Notepad used first to check that this client can read an ordinary window. The minimum
  supported version and Windows 11 read the accessibility tree byte for byte identically. What it
  read: the window is named after the product, the language combobox carries "简体中文" as both its
  name and its value, the agreement checkbox carries its sentence, and image buttons report the name
  `accessible-name` gives them. That reading closed two gaps, and a third was closed later:

  - A button whose label is artwork has no words to read, so a screen reader user hears only "button";
    a layout now names such a control with `accessible-name`, and the example's minimize, close and
    custom-options buttons already use it.
  - The checkbox's name carried its inline link markup, `[《服务协议》](agreement)`, and it now reads as
    the sentence without the markup.
  - **A disabled control is in the tree now.** The install button is unusable until the agreement is
    ticked: it used to not appear as an element at all, so a screen reader user could not hear that the
    page has one, still less what it is waiting for. It now reports its name and role with the
    unavailable state, in the place the layout puts it, and becomes usable once the box is ticked. The
    Tab ring still skips it, and a click or a hover still does not land on it. The tree is built from
    the Tab order, so this change is what separates "the page took it away" from "the page draws it and
    it cannot be used yet".

  Narrator itself was never heard, so this counts as "the accessibility tree was read on the minimum
  supported version", not as "a screen reader was run against it".
- Dragging, minimising and restoring the window (item 2), and dragging it to a display with a different
  scaling: the virtual machine did neither.
- Cursor shapes: `screendump` does not capture the hardware cursor, so the machine itself was asked
  instead. With the wizard in front, `scripts/cursor_probe.ps1` walked the screen on a grid of 60 pixels
  and called `GetCursorInfo` at every point: at 1920x1080 six points reported the hand cursor, in two runs
  of three neighbouring points, which is where the two buttons are, and every other point was the arrow. No
  third cursor shape appeared.
- Services (item 9): this real-machine run was not elevated and the example project installs none, so
  there was nothing to observe. The automated suite already installs a real service and deletes it
  where it has the rights, and covers the round trip through the manifest file; another real-machine
  look is not planned, because most setups involve no service.
- Dependency downloads: the virtual machine had no KB3140245, so the TLS 1.2 path was not reached.
  KB3033929 was not installed either.
- 32-bit versions and ARM64 are outside what is supported, and a machine without SP1 is outside what is
  claimed.

## Known limitations

- The MSI package is not signed either, so SmartScreen still warns about an unknown publisher.
  Installing it for the whole machine needs a session with administrator rights, and a silent install
  makes Windows Installer refuse outright under `/qn` rather than raise the administrator prompt (UAC).

- The runtime embeds the Rhai engine, and the engine is where the size of each runtime goes. Without it
  about 0.57-0.66 MB, and 1.47-1.54 MB with it, measured on this machine: the uninstaller 1.47 MB, the
  ZIP one 1.52 MB, the LZMA one 1.54 MB. It used to be 2.02-2.11 MB. That slimdown was chosen by
  measuring one lever at a time: `opt-level = "z"` about −350 KB. The linker's `/OPT:ICF=3` about
  −170 KB, which the MSVC linker does not fold by default. `panic = "immediate-abort"` about −170 KB.
  And Rhai's `no_optimize` and `no_time` about −45 KB together, neither of which the script API can
  reach. The same flags took the builder from 702 KB to 583 KB. The visual builder has a `gui` profile
  of its own, because `z` makes it 400 KB larger instead.
- An elevated setup and its uninstaller both run at high integrity, so a product that writes only to
  `%LOCALAPPDATA%` should turn `install.require_admin` off.
- Scaling on a multi-monitor desktop: a setup declares both `dpiAware`, which the minimum supported
  version and Windows 8.1 read, and `dpiAwareness`, which versions from Windows 10 1607 read, set to
  `PerMonitorV2, PerMonitor`. A window dragged to a display with a different scaling is laid out again
  for that display, so text and pictures stay sharp. On the minimum supported version the system still
  scales for the primary display.

- A dependency is downloaded through the WinHTTP Windows ships, with TLS 1.1/1.2 turned on for https.
  On the minimum supported version that also needs the machine's own schannel to support TLS 1.2, which
  means KB3140245 or a later update. Without it the download fails and says why rather than falling
  back to plain text. An http address is unaffected.

The real-machine run on the minimum supported version is done. Once signing is in place, a product can
plug in the way `examples/TapTap` does.


