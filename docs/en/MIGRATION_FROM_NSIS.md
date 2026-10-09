# Migrating from NSIS

An NSIS installer is one script that is also the program. `Section` blocks run,
`File` copies, `WriteRegStr` writes, and every screen the user sees is a macro the
NSIS runtime draws for you — all of that mixed together in one place. A Nano
Installer project splits the mix into three parts: what the product is and where it
goes, what the pages look like, and what a run does. The last part gets a small
script of your own, in Rhai, so most of a migration is not a rewrite at all: it is
moving each line to the place that now owns it, and the tables below say where each
line goes.

This page is the reference for that move, and its tables are more than a
description of [`scripts/check_nsi_migration.ps1`](../../scripts/check_nsi_migration.ps1):
they are the table that script reads. A construct this page has no row for is
reported as `unknown`, never guessed at, and the two language versions must agree
line for line or the check fails.

## The shape of a project

| NSIS | Nano Installer |
| --- | --- |
| `setup.nsi` and the `!include`s beside it | `installer_config.json`, one XML file per page under `layouts/`, one JSON file per language under `locales/`, and `scripts/install.rhai` / `scripts/uninstall.rhai` when the built-in steps are not enough |
| The files you install, written out as a list of `File` statements | One ZIP or 7z archive the project names (`resources.payload_file`), plus one archive per optional component |
| `Section` blocks and `SectionIn` | `components.items`, with a `Checkbox` on a page per component |
| `MUI_PAGE_*` macros | Pages of your own, listed in `wizard.pages` and `wizard.uninstall_pages` |
| `LangString` and `LoadLanguageFile` | `locales/<locale>.json` and `localization.supported_locales` |
| `MUI_LANGUAGE` | the same |
| `Uninstall` section deleting what the installer wrote | The machine's manifest, which records every file, shortcut and registry value the install wrote; the uninstaller removes what it records |
| `!finalize` and `!uninstfinalize` | `finalize.installer` and `finalize.uninstaller` — the same two hooks, with the same job |
| A plugin that does something NSIS cannot | A primitive in the script, a declared dependency, `run_command` — or a plugin of your own, written against the [plugin ABI](PLUGIN_API.md) and called from the script |

Two things do not survive the move, and both are worth deciding early. You no
longer need an `Uninstall` section that deletes by hand, because the manifest knows
what the install wrote; an uninstaller that works from a hand-written list is how
products come to leave files behind. And NSIS's `SetShellVarContext all` has no
counterpart, so shortcuts are created for the account that runs the setup, as the
table below says.

## Run the checker over the script

```powershell
.\scripts\check_nsi_migration.ps1 -Script .\legacy.nsi
```

It reads the script line by line, and the files `!include`d beside it. Then it
prints what each statement becomes:

```
NSIS migration report for D:\src\legacy.nsi

  direct   a configuration setting or a page element does this
  script   a primitive in scripts/install.rhai or scripts/uninstall.rhai does this
  manual   no equivalent: the guide says what to do instead
  none     build-time or cosmetic: nothing to migrate
  unknown  the table on this page has no row for it yet

     16  Name "${APP_NAME}"                              direct   project.name
     20  RequestExecutionLevel admin                     direct   install.require_admin
     38  !insertmacro MUI_PAGE_INSTFILES                 direct   the page the configuration gives role "progress"
     46  SetShellVarContext all                          manual   shortcuts go to the folders of the account that runs the setup
     63  WriteRegExpandStr HKLM "Software\App" "Data"     script   reg_write_expand_string
     72  Pop $0                                          manual   Rhai has variables; the NSIS stack has no counterpart

92 statement(s) read from 1 file(s)
  direct     31
  script     43
  manual      5
  none       13
  unknown     0

5 line(s) the guide has to answer for:
  legacy.nsi:19  InstallDirRegKey HKLM "${UNINST_KEY}" "InstallLocation"
  legacy.nsi:46  SetShellVarContext all
  ...
```

The exit code is 0 whenever a report was produced, whatever it says — the check
is a reading aid, not a gate. Add `-FailOnUnknown` to make it a gate, which is
how this repository's own CI runs it over two fixtures. Of those two, `legacy.nsi`
is one product's installer, while `instructions.nsi` carries one line for every
instruction, attribute and header macro NSIS documents. The first holds the table
to a real script, the second holds it to the language — a construct NSIS has and
the table does not fails the check instead of printing `unknown` at somebody who
then has to guess.

The five verdicts the table below uses are the five printed above: `direct` is what a
configuration setting or a page element does on its own, and `script` is a primitive in
`scripts/install.rhai` or `scripts/uninstall.rhai`. `manual` is no equivalent, so this page
says what to do instead; `none` is build-time or cosmetic, with nothing to migrate; and
`unknown` is what this page carries no row for yet.

## The table

Each row says what a command becomes, and several spellings of one command share a
row. A `*` at the end of a name matches anything after it, so `MUI_PAGE_*`
catches the MUI pages that have no row of their own. A macro from one of NSIS's
own header libraries is listed under the name a script calls it by, so
`${GetSize}` and `${StrStr}` are the `GetSize` and `StrStr` rows — the `${...}`
is not part of the name. A label (`done:`) is a place, not a statement, and Rhai's
own control flow replaces the jumps that reach it, so the checker reports one as
`none`.

### The product and the file it builds

| Command | Verdict | What it becomes |
| --- | --- | --- |
| `Name` | direct | `project.name` |
| `OutFile` | direct | `output.installer_name`, or the `--output` the build is given |
| `InstallDir` | direct | `install.default_path`; a `TextInput` with `value-source="config:install.default_path"` lets the user change it |
| `InstallDirRegKey` | manual | nothing reads the previous folder back; upgrading an install in the same folder already works, and `reg_read_expand_string` can read that key |
| `RequestExecutionLevel` | direct | `install.require_admin` |
| `Icon` | direct | `output.installer_icon` |
| `UninstallIcon` | direct | `output.uninstaller_icon` |
| `VIProductVersion`, `VIFileVersion` | direct | `project.file_version` |
| `VIAddVersionKey` | direct | `project.publisher`, `project.copyright`, `project.description` |
| `ManifestDPIAware` | direct | `ui.dpi_aware`; the builder writes the application manifest |
| `ManifestSupportedOS`, `ManifestLongPathAware` | none | the setup already claims Windows 7 SP1 x64 and later, and the builder writes the manifest |
| `PEAddResource`, `PERemoveResource` | manual | the builder injects the project's icon, version information and manifest; nothing adds a resource of its own |
| `SetCompressor`, `SetCompressorFinal`, `SetCompressorDictSize`, `SetCompress`, `FileBufSize` | none | the builder packs the payload itself, in blocks of its own size |
| `SetDatablockOptimize` | none | nothing to migrate |
| `CRCCheck` | none | the payload carries its own digests; an unsigned setup cannot checksum itself at run time |
| `Unicode` | none | every build is Unicode |
| `Target` | none | the setup is x64 |
| `XPStyle` | none | the pages draw themselves |
| `ShowInstDetails`, `ShowUninstDetails`, `SetDetailsPrint`, `SetDetailsView` | none | the task reports through the page carrying `role: "progress"` |
| `SetOverwrite`, `SetDateSave`, `AllowSkipFiles` | none | files land where the payload puts them; an upgrade replaces the previous version, and a file that cannot be written fails the step and rolls it back |
| `BrandingText`, `SetBrandingImage`, `AddBrandingImage`, `BGFont`, `BGGradient`, `CheckBitmap`, `ChangeUI`, `WindowIcon`, `InstProgressFlags` | none | the pages carry the words, the artwork and the progress bar |
| `InstallColors`, `SetFont`, `SetCtlColors`, `CreateFont`, `LicenseBkColor` | none | the pages carry their own colours and fonts |
| `AutoCloseWindow`, `SetAutoClose` | none | the finish page closes when the user says so |
| `LoadLanguageFile`, `LangFile` | none | the runtime reads `locales/<locale>.json` |
| `LangString` | direct | a field in `locales/<locale>.json`, asked for with `text="@key"` |
| `Caption` | direct | the `title` of that page in `wizard.pages` |
| `SubCaption`, `CompletedText`, `ComponentText`, `DirText`, `SpaceTexts`, `FileErrorText`, `DetailsButtonText`, `InstallButtonText`, `MiscButtonText`, `UninstallButtonText`, `UninstallCaption`, `UninstallSubCaption`, `UninstallText` | direct | the page's own words, in the page's layout and `locales/<locale>.json` |
| `LicenseText`, `LicenseData` | direct | the words on the licence page, or a link to them |
| `LicenseForceSelection` | direct | a `Checkbox` on that page, and `enabled-when="<its id>:checked"` on the button that goes on |
| `DirVar`, `DirVerify` | direct | the directory page's `TextInput` holds the folder, and its own `required`, `min-length` and `pattern` rules say what may go in it |
| `AllowRootDirInstall` | manual | the setup refuses the drive root itself; name a folder below it |
| `BringToFront`, `LockWindow` | none | the wizard owns its own window and draws itself |

### Building, and the pages

| Command | Verdict | What it becomes |
| --- | --- | --- |
| `!define`, `!undef`, `!searchparse`, `!searchreplace`, `!addincludedir` | none | build-time text, expanded by NSIS before anything runs |
| `!if`, `!ifdef`, `!ifndef`, `!else`, `!endif`, `!error`, `!warning`, `!verbose`, `!echo`, `!pragma`, `!cd`, `!tempfile`, `!delfile`, `!appendfile`, `!getdllversion` | none | build-time tests and build-time text, decided before anything is packaged |
| `!macro`, `!macroend` | none | the statements inside are reported where they are written |
| `!insertmacro` | none | a macro the script defines itself; its statements are reported where they are written |
| `!addplugindir` | manual | the compiler has no plugin search path: a plugin of your own goes in `resources.plugins_dir` and is embedded in the setup |
| `!system`, `!execute` | manual | the builder runs no command of its own while building; `finalize.installer` runs one on the finished setup |
| `!finalize` | direct | `finalize.installer` |
| `!uninstfinalize` | direct | `finalize.uninstaller` |
| `!packhdr` | manual | nothing rewrites the setup between building it and signing it |
| `Page` | direct | an entry in `wizard.pages`, laid out in XML |
| `UninstPage` | direct | an entry in `wizard.uninstall_pages` |
| `PageCustom` | direct | a page of your own XML, plus `scripts/pages.rhai` when the order is conditional |
| `PageComponents` | direct | a page with one `Checkbox` per component |
| `PageDirectory` | direct | a page with a `TextInput` and `action="pick_directory"` |
| `PageLicense` | direct | a page with the licence text, or a link to it |
| `PageInstFiles` | direct | the page the configuration gives `role: "progress"` |
| `PageEx` | direct | an entry in `wizard.pages` |
| `PageExEnd` | none | nothing to migrate |
| `MUI_PAGE_*` | manual | an MUI page with no row of its own: build it from the page elements |
| `MUI_UNPAGE_*` | manual | an MUI page with no row of its own: build it from the uninstall page elements |
| `MUI_PAGE_WELCOME` | direct | a page of its own in `wizard.pages` |
| `MUI_PAGE_LICENSE` | direct | a page with the licence text, or a link to it |
| `MUI_PAGE_COMPONENTS` | direct | a page with one `Checkbox` per component |
| `MUI_PAGE_DIRECTORY` | direct | a page with a `TextInput` and `action="pick_directory"` |
| `MUI_PAGE_INSTFILES` | direct | the page the configuration gives `role: "progress"` |
| `MUI_PAGE_FINISH` | direct | the page the configuration gives `role: "finish"`; `action="launch_app"` runs the product |
| `MUI_PAGE_STARTMENU` | direct | a page with one `Checkbox` per start-menu choice, or the shortcuts settings |
| `MUI_PAGE_UNINSTCONFIRM` | direct | a page with the keep-data checkbox (`chkReserveData`) |
| `MUI_UNPAGE_CONFIRM` | direct | the uninstall page with the keep-data checkbox (`chkReserveData`) |
| `MUI_UNPAGE_INSTFILES` | direct | the uninstall page the configuration gives `role: "progress"` |
| `MUI_UNPAGE_LICENSE` | direct | an uninstall page with the licence text |
| `MUI_UNPAGE_COMPONENTS` | direct | an uninstall page with one `Checkbox` per component |
| `MUI_UNPAGE_DIRECTORY` | direct | an uninstall page with a `TextInput` and `action="pick_directory"` |
| `MUI_LANGUAGE` | direct | `localization.supported_locales`, with `locales/<locale>.json` beside it |

### Sections, components and what a run does

| Command | Verdict | What it becomes |
| --- | --- | --- |
| `Section` | direct | a component in `components.items`; `SectionIn RO`, or `!` in the name, means `required: true` |
| `SectionEnd` | script | the end of that part of `install.rhai` |
| `SectionGroup` | direct | a page that carries the group's components |
| `SectionGroupEnd` | none | nothing to migrate |
| `SectionIn` | direct | `required: true` for `RO`; otherwise the checkbox on the page decides |
| `SectionInGroup` | direct | the components the page shows |
| `InstType`, `InstTypeSetText`, `InstTypeGetText`, `SectionInstType` | manual | there are no named installation types: the page's checkboxes pick the components one by one, and a script cannot tick one |
| `SectionSetFlags`, `SectionGetFlags`, `SectionSetText`, `SectionGetText`, `SectionSetSize`, `SectionGetSize`, `SectionSetInstTypes`, `SectionGetInstTypes`, `SetCurInstType`, `GetCurInstType` | manual | the page's checkboxes decide the components; a script asks with `is_component_selected` and `selected_components` and cannot change them |
| `AddSize` | none | `install.required_space_mb` is the one space check that runs; nothing adds up a section's size |
| `Nop` | none | nothing to migrate |
| `SetPluginUnload` | none | a plugin lives as long as the setup that loaded it |
| `SetOutPath` | none | the payload archive carries the paths |
| `File` | none | put the files in the payload archive, or in a component's |
| `CreateDirectory` | script | `create_dir` |
| `RMDir` | script | `delete_dir` |
| `Delete` | script | `delete_file` |
| `CopyFiles` | script | `copy_file` |
| `Rename` | manual | no rename primitive: `copy_file` then `delete_file` |
| `WriteUninstaller` | direct | the uninstaller is the project's own (`output.uninstaller_name`); `install.rhai` calls `copy_uninstaller()` |
| `CreateShortCut` | script | `create_desktop_shortcut`, `create_start_menu_shortcut` or `create_uninstall_shortcut` |
| `WriteRegStr` | script | `reg_write_string` |
| `WriteRegExpandStr` | script | `reg_write_expand_string` |
| `WriteRegDWORD` | script | `reg_write_dword` |
| `WriteRegBin` | script | `reg_write_binary` |
| `WriteRegNone` | manual | no primitive writes a value with no type |
| `ReadRegStr` | script | `reg_read`, or `reg_read_expand_string` for a value that carries references |
| `ReadRegDWORD` | script | `reg_read_dword` |
| `DeleteRegKey` | script | `reg_delete_key` |
| `DeleteRegValue` | script | `reg_delete_value` |
| `EnumRegKey`, `EnumRegValue` | manual | no primitive lists the subkeys or the values of a key |
| `SetRegView` | direct | the `HKLM64` / `HKLM32` prefix on the key |
| `WriteINIStr`, `ReadINIStr`, `DeleteINISec`, `DeleteINIStr`, `FlushINI` | manual | no INI primitive: `read_text_file` and `write_file`, or a command through `run_command` |
| `RegDLL`, `UnRegDLL` | manual | run `regsvr32` through `run_command` |
| `SetShellVarContext` | manual | shortcuts go to the folders of the account that runs the setup; there is no all-users switch |
| `GetDlgItem`, `SendMessage`, `ShowWindow`, `EnableWindow` | manual | there are no control handles to poke: the page declares its controls and the layout says what they look like |
| `HideWindow` | manual | nothing hides the wizard while a step runs; the progress page names the step instead |
| `DetailPrint` | script | `set_status` for written words, `set_status_key` for a locale key |
| `LogText` | script | `log_info` |
| `LogSet` | none | every run keeps its own log on disk, and the log is not something a script turns on |
| `InitPluginsDir` | script | `get_temp_path` for a folder of its own, `extract_payload` for what the setup carries |
| `GetInstDirError` | manual | there is no error code to read: the directory page's own rules refuse a value before the install starts |
| `MessageBox` | script | `show_message`, `show_error` or `ask_yes_no`, drawn inside the window from `ui.dialog_layout` |
| `Sleep` | script | `sleep_ms` |
| `GetTempFileName` | script | `get_temp_path`, with `path_join` to name the file |
| `GetSize` | script | `get_file_size` |
| `GetDrives` | script | `get_drives` |
| `DriveSpace` | script | `get_drive_space` |
| `GetFileName`, `GetBaseName` | script | `path_filename`, which keeps the extension |
| `FindFirst`, `FindNext`, `FindClose` | script | `list_dir` |
| `GetFileTime`, `GetFileTimeLocal`, `GetTime` | manual | no primitive reads a file time or the clock |
| `SetFileAttributes`, `GetFileAttributes` | manual | no primitive reads or sets file attributes |
| `SearchPath` | manual | `get_env` covers a named variable; nothing searches the path list |
| `GetFullPathName` | manual | the primitives that take a path require it to be absolute |
| `GetRoot` | manual | no primitive returns a drive root; `get_drives` lists what there is |
| `GetParent` | script | `path_parent` |
| `ReadEnvStr` | script | `get_env` |
| `ExpandEnvStrings` | none | the install folder, a dependency's own paths and `reg_read_expand_string` expand `%NAME%`; every other primitive takes the path as written |
| `FileRead` | script | `read_text_file` |
| `FileWrite` | script | `write_file` |
| `FileOpen`, `FileClose`, `FileWriteByte`, `FileReadByte`, `FileSeek` | manual | no primitive appends to a file, moves inside one or writes raw bytes, and nothing holds a handle open for you |
| `GetDLLVersion`, `GetDLLVersionLocal`, `GetFileVersion` | manual | no primitive reads a file version; `run_command_output` can run a program that does |
| `ExecWait` | script | `run_command`, or `run_command_output` when the exit code and the output matter |
| `Exec` | script | `run_detached` |
| `ExecShell`, `ExecShellWait` | manual | no primitive opens a URL or a document; a page element with `action="open_url:<links key>"` does |
| `Reboot`, `IfRebootFlag`, `SetRebootFlag` | manual | a run never restarts the machine; a dependency answering 3010 or 1641 counts as installed and the run continues |

### Plugins, flow and the NSIS runtime

| Command | Verdict | What it becomes |
| --- | --- | --- |
| `*::*` | manual | a plugin call becomes `plugin_call("dll::function", [...])`; the DLL itself is rebuilt against [`include/nano_plugin.h`](../../include/nano_plugin.h), because an NSIS plugin is 32-bit and calls back into the NSIS runtime |
| `nsProcess::_FindProcess` | script | `is_process_running` |
| `nsProcess::_KillProcess` | script | `kill_process` |
| `nsExec::Exec` | script | `run_command` |
| `nsExec::ExecToLog` | script | `run_command_output` — the exit code and both streams come back |
| `nsExec::ExecShellEx` | script | `run_command` |
| `EnvVar::set` | script | `set_env` |
| `EnvVar::unset` | script | `remove_env` |
| `Var` | script | a `let` in Rhai; a script's variables need no declaration |
| `Function`, `FunctionEnd` | script | a function in `install.rhai` or `uninstall.rhai` |
| `Call` | script | a function call in Rhai |
| `Return` | script | `return` |
| `Goto` | manual | Rhai has `if`/`else` and loops; labels have no counterpart |
| `Abort` | script | `return` from the script, after saying why through `show_error` |
| `Quit` | script | `return` |
| `SetErrors`, `ClearErrors`, `GetErrorLevel` | none | the primitives return their own result |
| `SetErrorLevel` | manual | a silent run answers with the runtime's own exit code |
| `IfErrors` | script | the primitives return their own result |
| `IfFileExists` | script | `file_exists` and `is_dir` |
| `IfAbort` | manual | nothing sets an abort flag; a script stops itself with `return` |
| `IfSilent` | manual | no primitive tells a script whether this run is windowless; `show_message`, `show_error` and `ask_yes_no` already answer that question themselves, and no page is drawn |
| `StrCmp`, `StrCmpS`, `StrICmp` | script | `==` in Rhai |
| `StrCpy` | script | `let` in Rhai |
| `StrLen` | script | `.len` in Rhai |
| `StrReplace`, `StrStr`, `StrTok`, `StrTrim` | script | Rhai's string methods |
| `StrRep`, `StrLoc`, `StrSort`, `StrTrimNewLines`, `WordFind`, `WordReplace`, `WordInsert`, `WordDelete`, `TextCompare` | script | Rhai's string methods |
| `LineFind`, `LineRead`, `LineSum`, `FileReadFromEnd`, `FileJoin`, `TrimNewLines` | script | `read_text_file`, then Rhai's own string handling |
| `VersionCompare`, `VersionConvert` | manual | no primitive compares versions; a dependency's `detect.at_least` is where the machine is asked about one |
| `GetParameters`, `GetOptions` | manual | a silent run takes `--dir` and `--log` and refuses anything else, so there is no command line for a script to parse |
| `DisableX64FSRedirection`, `EnableX64FSRedirection`, `RunningX64` | none | the setup is x64 |
| `AtLeastWin*`, `IsWin*` | manual | no primitive hands a script the version of Windows; the run's own log records it in its first lines |
| `IntOp` | script | arithmetic in Rhai |
| `IntCmp`, `IntCmpU` | script | a comparison in Rhai |
| `IntFmt` | manual | nothing pads a number into a string |
| `Push`, `Pop`, `Exch` | manual | Rhai has variables; the NSIS stack has no counterpart |
| `GetLabelAddress`, `GetFunctionAddress`, `GetCurrentAddress` | manual | labels and addresses have no counterpart |
| `SetSilent`, `SilentInstall`, `SilentUninstall` | direct | `advanced.silent_mode_support` and `advanced.uninstall_mode_support`; a silent run is `--silent` |
| `${If}`, `${Unless}`, `${IfNot}`, `${AndIf}`, `${OrIf}`, `${AndUnless}`, `${OrUnless}` | script | `if` in Rhai |
| `${ElseIf}`, `${Else}` | script | `else if` and `else` in Rhai |
| `${EndIf}`, `${EndUnless}`, `${While}`, `${EndWhile}`, `${For}`, `${ForEach}`, `${Next}`, `${Do}`, `${Loop}`, `${Until}`, `${DoWhile}`, `${Break}`, `${Continue}` | script | Rhai's own control flow |
| `${Select}`, `${Case}`, `${CaseElse}`, `${Default}`, `${EndSelect}`, `${Switch}`, `${EndSwitch}` | script | `switch` and `if`/`else if` in Rhai |
| `${SelectSection}`, `${UnselectSection}` | manual | the page's checkboxes decide the components; a script cannot tick one |

## What has no counterpart, and what to do instead

These are the rows that cost real time. Read them before the rest of the
table.

- **Plugins.** There is a plugin ABI now, and it is not NSIS's: a plugin is a
  64-bit DLL built against [`include/nano_plugin.h`](../../include/nano_plugin.h),
  embedded in the setup like a project resource, and called from a script as
  `plugin_call("dll::function", [...])`. An existing NSIS plugin cannot be loaded,
  because it is a 32-bit image that calls back into the NSIS runtime through
  `extra_parameters`, so `nsDialogs`, `nsisXML`, `InetLoad`, `nsis7z` and the
  rest have to be rewritten or replaced. Most of what they did has a counterpart
  already: a page built with `nsDialogs` becomes a page in `layouts/`, plus a
  `next_page(from)` hook in `scripts/pages.rhai` when its behaviour depends on
  what the user did. A download becomes a `dependencies.items` entry with a
  `sha256`, or `download_file_with_hash` from the script, and an archive becomes
  the payload or a component. What is left over is what a plugin of your own is
  for.
- **`SetShellVarContext all`.** Shortcuts are created for the account running
  the setup, so an installer that wrote an all-users shortcut has to place it
  another way: a per-machine install started with `require_admin` still writes the
  shortcut of the account that ran it.
- **Installation types.** `InstType`, `SectionInstType`, `SectionSetFlags` and
  the rest of that family have no counterpart, and the checkboxes on the page
  decide what gets installed: a script can ask (`is_component_selected`,
  `selected_components`), but it cannot tick one. If a type used to set the
  sections, move that decision onto the page and let the user make it — a
  `RadioButton` group can gate a `Button` with `enabled-when="mode:full"`, but
  nothing changes a checkbox the user is looking at, script or page.
- **Control handles.** `GetDlgItem`, `SendMessage`, `ShowWindow` and
  `EnableWindow` reach into the window and drive a control directly, and there
  are no handles here: the page declares its controls, and the layout says what
  they look like and when they may be used.
- **Accepting the licence.** `LicenseForceSelection` is a page element, not a
  setting: a `Checkbox` on the licence page, and `enabled-when` on the button
  that goes on, is the same gate the MUI licence page enforced.
- **Restarting the machine.** Nothing here restarts a machine: a dependency
  that answers 3010 or 1641 is reported as installed and the run continues, while
  a product that cannot work before a restart has to say so on its finish page.
- **The NSIS stack.** `Push`, `Pop` and `Exch` exist because NSIS has one stack
  and no variables scoped to a function; Rhai has neither problem.
- **`MessageBox` outside the window.** Here a prompt is a card inside the
  wizard, drawn from `ui.dialog_layout`, and a script waits for the click on a
  worker thread; a project that ships no such layout falls back to the system
  dialog, and one that runs windowless never prompts at all.
- **`WriteINIStr`, registry enumeration, file times, DLL registration.** Small,
  specific gaps, and each row above names the primitive or command that stands in.

## A worked example

`examples/nsis-migration/` holds a representative NSIS script and the project it
becomes, and `instructions.nsi` sits beside them: it is the same table held to the
language, one line for every instruction, attribute and header macro NSIS
documents.

```powershell
.\scripts\check_nsi_migration.ps1 -Script .\examples\nsis-migration\legacy.nsi -FailOnUnknown
```

`legacy.nsi` is 111 lines, and the check reports 92 statements: 31 settings the
project declares, 43 primitives the script calls, 13 build-time or cosmetic
lines, and 5 the table answers with "there is no equivalent". The migration is in
`examples/nsis-migration/migrated/`:

- `installer_config.json` — the product, the two components, the dependency, the
  install registration and the pages. `Name`, `OutFile`, `InstallDir`,
  `RequestExecutionLevel`, the `VIAddVersionKey` group, `SectionIn RO` and
  `Section /o` all live here.
- `layouts/` — seven pages: welcome, options, progress, finish, the dialog, and
  the two uninstall pages. `MUI_PAGE_COMPONENTS` and `MUI_PAGE_DIRECTORY` are one
  page here, because a page is a layout and not a macro.
- `locales/zh-CN.json` and `locales/en-US.json` — what `LangString` wrote.
- `scripts/install.rhai` — the core section, in the order the NSIS script ran
  it: close the running copy, unpack the payload and the selected components,
  install the required dependency, run the bundled migration tool and stop when
  it fails, create the shortcuts the page asked for, write the one registry
  value the project keeps of its own.
- `scripts/uninstall.rhai` — `run_tracked_uninstall` for everything the manifest
  records, then the two paths the product itself writes into.

The uninstall section shrank the most: eight `Delete`, `RMDir` and
`DeleteRegKey` statements became one call, because the install recorded what it
wrote, so it is the part worth doing first — a hand-written uninstall list is what
leaves files behind when a future version installs one more file than the list
knows about.

## What the migrated project does without being asked

The installer this repository builds already does what an NSIS script usually
hand-writes. So these lines need no migration at all:

- The uninstall entry — display name, version, publisher, icon, the quiet
  uninstall command, the installed size, `NoModify`, `NoRepair` — is written from
  `registry.uninstall_key` and the project's metadata.
- Every file, shortcut and registry value the install writes is recorded, and
  the uninstaller replays that record.
- A failed step rolls the machine back to the previous state.
- A silent run needs `advanced.silent_mode_support`; it accepts `--dir` and
  `--log` and refuses anything else.
- Every run leaves a log on disk, named on failure.
- An upgrade in place is what re-running the setup does, and an update package
  can carry only the files whose bytes changed (`--delta-from`).
- Signing is the pipeline's step, called through `finalize.installer` and
  `finalize.uninstaller`.

## Before you ship the migrated installer

1. `check_nsi_migration.ps1` reports no `unknown` statements.
2. Every `manual` line has a decision recorded — implemented another way, or
   deliberately dropped.
3. The pages walk the way the NSIS pages did: `wizard.pages`, the `role` of the
   progress and finish pages, and `scripts/pages.rhai` where a page was
   conditional.
4. The components install what the sections installed, including the one that
   started unchecked (`default: false`).
5. Silently, in a disposable VM: `MyApp_Setup.exe --silent --dir <path> --log <file>`,
   then `uninst.exe --silent`, then read the log.
6. The uninstall leaves nothing behind that the old `Uninstall` section removed.
