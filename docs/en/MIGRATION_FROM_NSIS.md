# Migrating from NSIS

An NSIS installer is one script that is also the program: `Section` blocks run,
`File` copies, `WriteRegStr` writes, and everything the user sees is a macro the
NSIS runtime draws for you. A Nano Installer project separates the three things
that script mixes together — what the product is and where it goes, what the
pages look like, and what the run does — and gives the last of those a small
script of its own. Most of a migration is therefore not a rewrite: it is moving
each line to the place that now owns it.

This page is the reference for that move. Its tables are not a description of
[`scripts/check_nsi_migration.ps1`](../../scripts/check_nsi_migration.ps1);
they are the table that script reads, so a construct this page has no row for is
reported as `unknown` rather than guessed at, and the two language versions have
to agree or the check fails.

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
| A plugin that does something NSIS cannot | A primitive in the script, a declared dependency, or `run_command` — there is no plugin ABI |

Two things do not survive the move, and both are worth deciding early. An
`Uninstall` section that deletes by hand is not needed: the manifest knows what
the install wrote, and an uninstaller that recomputes it from a hand-written
list is how products come to leave files behind. And an NSIS `SetShellVarContext all`
has no counterpart — shortcuts are created for the account that runs the setup
(see the table below).

## Run the checker over the script

```powershell
.\scripts\check_nsi_migration.ps1 -Script .\legacy.nsi
```

It reads the script line by line, follows `!include`s that sit next to it, and
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
is a reading aid, not a gate. `-FailOnUnknown` makes it fail instead, which is
what this repository's own CI runs on `examples/nsis-migration/legacy.nsi`: a
fixture that carries one of every construct is only useful while the table
covers all of them.

## The table

Each row says what a command becomes. Where several spellings of the same
command exist they share a row. `*` at the end of a name matches anything after
it, so `MUI_PAGE_*` catches the MUI pages that have no row of their own.

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
| `VIProductVersion` | direct | `project.file_version` |
| `VIAddVersionKey` | direct | `project.publisher`, `project.copyright`, `project.description` |
| `ManifestDPIAware` | direct | `ui.dpi_aware`; the builder writes the application manifest |
| `ManifestSupportedOS` | none | the setup already claims Windows 7 SP1 x64 and later |
| `SetCompressor`, `SetCompressorFinal` | none | the builder packs the payload itself |
| `SetDatablockOptimize` | none | nothing to migrate |
| `CRCCheck` | none | the payload carries its own digests; an unsigned setup cannot checksum itself at run time |
| `Unicode` | none | every build is Unicode |
| `Target` | none | the setup is x64 |
| `XPStyle` | none | the pages draw themselves |
| `ShowInstDetails`, `ShowUninstDetails` | none | the task reports through the page carrying `role: "progress"` |
| `SetOverwrite` | none | files land where the payload puts them; an upgrade replaces the previous version |
| `BrandingText`, `SetBrandingImage` | none | the pages carry the words and the artwork |
| `InstallColors`, `SetFont`, `SetCtlColors` | none | the pages carry their own colours and fonts |
| `AutoCloseWindow`, `SetAutoClose` | none | the finish page closes when the user says so |
| `LoadLanguageFile`, `LangFile` | none | the runtime reads `locales/<locale>.json` |
| `LangString` | direct | a field in `locales/<locale>.json`, asked for with `text="@key"` |
| `BringToFront` | none | the wizard owns its own window |

### Building, and the pages

| Command | Verdict | What it becomes |
| --- | --- | --- |
| `!define`, `!undef`, `!searchparse`, `!searchreplace`, `!addincludedir` | none | build-time text, expanded by NSIS before anything runs |
| `!macro`, `!macroend` | none | the statements inside are reported where they are written |
| `!insertmacro` | none | a macro the script defines itself; its statements are reported where they are written |
| `!addplugindir` | manual | there is no plugin ABI; what a plugin did has to be a primitive or a command the script runs |
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
| `WriteINIStr`, `ReadINIStr`, `DeleteINISec` | manual | no INI primitive: `read_text_file` and `write_file`, or a command through `run_command` |
| `RegDLL`, `UnRegDLL` | manual | run `regsvr32` through `run_command` |
| `SetShellVarContext` | manual | shortcuts go to the folders of the account that runs the setup; there is no all-users switch |
| `DetailPrint` | script | `set_status` for written words, `set_status_key` for a locale key |
| `MessageBox` | script | `show_message`, `show_error` or `ask_yes_no`, drawn inside the window from `ui.dialog_layout` |
| `Sleep` | script | `sleep_ms` |
| `GetTempFileName` | script | `get_temp_path`, with `path_join` to name the file |
| `GetSize` | script | `get_file_size` |
| `GetFileTime` | manual | no primitive reads a file time |
| `SetFileAttributes` | manual | no primitive sets file attributes |
| `SearchPath` | manual | `get_env` covers a named variable; nothing searches the path list |
| `GetFullPathName` | manual | the primitives that take a path require it to be absolute |
| `GetParent` | script | `path_parent` |
| `ReadEnvStr` | script | `get_env` |
| `ExpandEnvStrings` | none | most primitives expand `%NAME%` themselves, and `reg_read_expand_string` does it for a registry value |
| `FileRead` | script | `read_text_file` |
| `FileWrite` | script | `write_file` |
| `FileOpen`, `FileClose` | manual | no primitive appends to a file or keeps a handle open |
| `GetDLLVersion`, `GetDLLVersionLocal` | manual | no primitive reads a file version; `run_command_output` can run a program that does |
| `ExecWait` | script | `run_command`, or `run_command_output` when the exit code and the output matter |
| `Exec` | script | `run_detached` |
| `ExecShell` | manual | no primitive opens a URL or a document; a page element with `action="open_url:<links key>"` does |
| `Reboot`, `IfRebootFlag` | manual | a run never restarts the machine; a dependency answering 3010 or 1641 counts as installed and the run continues |

### Plugins, flow and the NSIS runtime

| Command | Verdict | What it becomes |
| --- | --- | --- |
| `*::*` | manual | a plugin call: there is no plugin ABI, so what it did has to be a primitive or a command the script runs |
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
| `SetErrors`, `ClearErrors` | none | the primitives return their own result |
| `SetErrorLevel` | manual | a windowless run answers with the runtime's own exit code |
| `IfErrors` | script | the primitives return their own result |
| `IfFileExists` | script | `file_exists` and `is_dir` |
| `IfSilent` | script | `get_mode()` and the arguments the run was given |
| `StrCmp`, `StrCmpS`, `StrICmp` | script | `==` in Rhai |
| `StrCpy` | script | `let` in Rhai |
| `StrLen` | script | `.len` in Rhai |
| `StrReplace`, `StrStr`, `StrTok`, `StrTrim` | script | Rhai's string methods |
| `IntOp` | script | arithmetic in Rhai |
| `IntCmp` | script | a comparison in Rhai |
| `IntFmt` | manual | nothing pads a number into a string |
| `Push`, `Pop`, `Exch` | manual | Rhai has variables; the NSIS stack has no counterpart |
| `GetLabelAddress` | manual | labels have no counterpart |
| `SetSilent`, `SilentInstall`, `SilentUninstall` | direct | `advanced.silent_mode_support` and `advanced.uninstall_mode_support`; a windowless run is `--silent` |
| `${If}`, `${Unless}`, `${IfNot}`, `${AndIf}`, `${OrIf}` | script | `if` in Rhai |
| `${ElseIf}`, `${Else}` | script | `else if` and `else` in Rhai |
| `${EndIf}`, `${While}`, `${EndWhile}`, `${ForEach}`, `${Next}`, `${Do}`, `${Loop}`, `${Break}`, `${Continue}` | script | Rhai's own control flow |

## What has no counterpart, and what to do instead

These are the rows that cost real time, so they are worth reading before the
rest of the table.

- **Plugins.** `nsDialogs`, `nsisXML`, `InetLoad`, `nsis7z` and the rest have no
  counterpart, because there is no plugin ABI here. A page built with `nsDialogs`
  becomes a page in `layouts/` and, when its behaviour depends on what the user
  did, a `next_page(from)` hook in `scripts/pages.rhai`. A download becomes a
  `dependencies.items` entry with a `sha256`, or `download_file_with_hash` from
  the script. An archive becomes the payload or a component.
- **`SetShellVarContext all`.** Shortcuts are created for the account running
  the setup. An installer that wrote an all-users shortcut has to place that
  shortcut another way — a per-machine install started with `require_admin` still
  writes the shortcut of the account that ran it.
- **Restarting the machine.** Nothing here restarts a machine. A dependency
  that answers 3010 or 1641 is reported as installed and the run continues; a
  product that cannot work before a restart has to say so on its finish page.
- **The NSIS stack.** `Push`, `Pop` and `Exch` exist because NSIS has one stack
  and no variables scoped to a function. Rhai has neither problem.
- **`MessageBox` outside the window.** Here a prompt is a card inside the
  wizard, drawn from `ui.dialog_layout`, and a script waits for the click on a
  worker thread. A project that ships no such layout falls back to the system
  dialog, and one that runs windowless never prompts at all.
- **`WriteINIStr`, registry enumeration, file times, DLL registration.** Small,
  specific gaps; each row above says which primitive or command stands in.

## A worked example

`examples/nsis-migration/` holds a representative NSIS script and the project it
becomes:

```powershell
.\scripts\check_nsi_migration.ps1 -Script .\examples\nsis-migration\legacy.nsi -FailOnUnknown
```

`legacy.nsi` is 111 lines and the check reports 92 statements: 31 settings the
project declares, 43 primitives the script calls, 13 build-time or cosmetic
lines, and 5 that the table answers with "there is no equivalent". Its migration
is `examples/nsis-migration/migrated/`:

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
wrote. That is the part of a migration worth doing first — a hand-written
uninstall list is what leaves files behind when a future version installs one
more file than the list knows about.

## What the migrated project does without being asked

The installer this repository builds already does what an NSIS script usually
hand-writes, so those lines do not have to be migrated at all:

- The uninstall registration — display name, version, publisher, icon, the quiet
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
