# Custom install and uninstall steps

If your project ships `scripts/install.rhai`, that script decides the install steps;
`scripts/uninstall.rhai` does the same for uninstall. Without them the built-in flow runs. Both
scripts share one set of primitives and differ only in their entry point.

The scripts are project data, not a plugin system: primitives are fixed and the engine enforces an
operation ceiling, so a runaway loop cannot hang an installation.

## Contract

- The install script must deploy the executable named by `install.exe_name`, otherwise
  `finish_install` fails and the installation rolls back.
- The uninstall script should call `run_tracked_uninstall(start, end)`, which removes shortcuts,
  registry values, and installed files recorded in the manifest. If your script skips it, the driver
  runs the library fallback after the script ends, so the product is still cleaned up; calling it
  lets the script choose the timing and progress range.
- A failing script (`throw`, or a primitive returning failure that the script returns on) rolls back
  the files, shortcuts, and registry values written by this installation, and removes an install
  directory that was created from scratch.

## Encoding

`scripts/install.rhai` and `scripts/uninstall.rhai` must be UTF-8.

## Progress and status

| Primitive | Description |
| --- | --- |
| `set_progress(percent)` | Sets progress, 0-100; out-of-range values are clamped |
| `set_status(text)` | Shows literal text, not translated |
| `set_status_key(key)` | Shows the localized text for a locale key, follows language switches |
| `is_cancelled()` | `true` once the user asked the running task to stop |
| `get_install_path()` | Current install directory |
| `get_checkbox_value(id)` | Reads a checkbox; use the layout id for install (`chkShotcut`) and `keep_data` for uninstall |
| `get_text_value(id)` | Reads what a field holds; an empty string when the page has no such control |
| `get_choice_value(id)` | Reads the value a select or radio group stands on; a select by its control id, a radio group by the group's name; an empty string when the page has neither |
| `is_component_selected(id)` | Whether this run installs that component |
| `selected_components()` | The component names this run installs, in the order the project declares them |
| `get_mode()` | `"install"` or `"uninstall"` |
| `log_info(text)`, `log_warn(text)`, `log_error(text)` | Write to the script log |

A `cancel` button, or the close question answered with Yes, asks the running task to stop. The
runtime then gives up at the checkpoint after the step that is running and undoes what it wrote, so
a script that would rather end a long step of its own early asks `is_cancelled()` in its loop and
returns by itself. See [actions](XML_LAYOUT_GUIDE.md#actions).

`get_text_value` and `get_choice_value` read the page as it stood when the user started the
install; a silent run has no page, so both answer an empty string.

The component selection comes from the same rule: beside the base payload, which of the components
the project cut out with `components.items` this run installs. `is_component_selected(id)` answers for
one by name, and `selected_components()` gives them in the order the project declares them, as the
[configuration reference](CONFIG_REFERENCE.md#components) describes. An uninstall has no page and no
component selection, so both primitives answer `false` and an empty array. The checkbox itself is
readable as a control through `get_checkbox_value(<component id>)`, which is what the page holds; ask
these two whether the run installs it.


Scripts have no console. On failure the runtime appends the last 32 log lines to the error the
wizard shows.

## Files and payload

| Primitive | Description |
| --- | --- |
| `extract_payload()` | Extracts the payload into the install directory, progress 0-100 |
| `extract_payload_with_progress(start, end)` | Same, with an explicit progress range |
| `copy_uninstaller()` | Writes the bundled uninstaller into the install directory |
| `write_file(path, contents)` | Writes a text file |
| `create_dir(path)` | Creates a directory recursively |
| `delete_dir(path)` | Deletes a directory recursively; the path must be absolute on a drive |
| `delete_file(path)` | Deletes a file |
| `copy_file(source, target)` | Copies a file |
| `file_exists(path)`, `is_dir(path)` | Existence checks |
| `get_file_size(path)` | Size in bytes, `-1` on failure |
| `read_text_file(path)` | Reads text, empty string on failure |
| `list_dir(path)` | Array of entry names |
| `path_join(base, child)`, `path_parent(path)`, `path_filename(path)` | Path manipulation |
| `get_temp_path()` | Temporary directory |
| `get_tools_dir()` | Unpacks the directory `resources.tools_dir` bundled, returns its path |
| `sleep_ms(milliseconds)` | Waits |

`extract_payload*` reuses the built-in extraction: it stages the payload to disk, unpacks it with
the matching runtime, and verifies it holds no uninstaller, no manifest, and no symbolic links. It
unpacks the base payload and the components this run installs, in the order the project declares
them, and two archives that carry one relative path fail there and then.

`get_tools_dir()` unpacks the directory `resources.tools_dir` names into the setup's own scratch
directory, keeping the relative paths, and returns that directory, which a script hands to
`run_command`. A run unpacks the tools once: a second call answers with the same path. A project
that bundles no tools, and a script that asks a setup built without them, both get an empty string
and a warning in the log, so the install carries on and the script decides what to do without the
program it hoped for.

## Registry

Keys use the `HKCU\...` or `HKLM\...` form. The manifest records the values and keys your script
writes, and uninstall removes them.

| Primitive | Description |
| --- | --- |
| `reg_write_string(key, name, value)` | Writes `REG_SZ` |
| `reg_write_expand_string(key, name, value)` | Writes `REG_EXPAND_SZ`, its references kept as written |
| `reg_write_multi_string(key, name, values)` | Writes `REG_MULTI_SZ` from an array of strings |
| `reg_write_dword(key, name, value)` | Writes `REG_DWORD` |
| `reg_write_qword(key, name, value)` | Writes `REG_QWORD` |
| `reg_write_binary(key, name, bytes)` | Writes `REG_BINARY` from an array of numbers 0-255 |
| `reg_read(key, name)` | Reads `REG_SZ`, empty string when absent |
| `reg_read_expand_string(key, name)` | Reads `REG_EXPAND_SZ`, its references expanded |
| `reg_read_multi_string(key, name)` | Reads `REG_MULTI_SZ` as an array of strings |
| `reg_read_dword(key, name)`, `reg_read_qword(key, name)` | Read a 32- or 64-bit number, `-1` when absent |
| `reg_read_binary(key, name)` | Reads `REG_BINARY` as an array of numbers |
| `reg_read_type(key, name)` | The type the machine stored, such as `REG_MULTI_SZ`; empty when absent |
| `reg_value_exists(key, name)` | Whether the value is there, whatever its type |
| `reg_key_exists(key)` | Whether the key exists |
| `reg_delete_value(key, name)` | Deletes a single value |
| `reg_delete_key(key)` | Deletes a key and its subkeys, and drops it from the tracked list |

A read answers for one type: `reg_read` on a `REG_DWORD` is an empty string and `reg_read_dword` on a
`REG_SZ` is `-1`, so a script that does not know what the machine holds asks `reg_read_type` first.
An empty string inside an array handed to `reg_write_multi_string` is dropped rather than written,
because it would end the list where it stands. A number outside 0-255 in `reg_write_binary`, and a
value outside the 32 bits of a `REG_DWORD`, are refused instead of written in part.
`reg_read_expand_string` expands what a value refers to, so a path the project wrote as
`%ProgramFiles%\MyApp` comes back as the directory this machine keeps it in. `reg_read_qword` hands
back a signed 64-bit number, so a value with its top bit set reads as a negative one.

A key may name the view it is read and written in, which 64-bit Windows keeps two of. Put `32` or
`64` after the hive: `HKLM64\SOFTWARE\MyApp` is the copy a 64-bit program reads,
`HKLM32\SOFTWARE\MyApp` the one a 32-bit program sees, and a bare `HKLM` is whichever this setup is
subject to. The long names work the same way, as in `HKEY_CURRENT_USER32`. The view is part of the
recorded key, so an uninstall takes back exactly the copy the script wrote to. The uninstall key and
the autostart entry are fixed to the native view, because their paths travel to the standalone
uninstaller as plain strings: `registry.uninstall_key` and `autostart.registry_key` take no view.

When you write into a container key shared with Windows or another product (such as
`...\CurrentVersion\Run`), the manifest records only that value, and uninstall deletes only that
value rather than the whole key. `reg_delete_key` refuses a target that names a hive root itself, so
a script cannot ask for `HKCU\Software` and take the machine's software tree with it.

## Shortcuts

| Primitive | Description |
| --- | --- |
| `create_desktop_shortcut(name, target)` | Desktop shortcut |
| `create_start_menu_shortcut(name, target, folder)` | Start menu shortcut |
| `create_uninstall_shortcut(name, target, folder)` | Uninstall shortcut, same arguments |
| `delete_desktop_shortcut(name)` | Removes a desktop shortcut |
| `delete_start_menu_folder(folder)` | Removes a Start menu folder |

## File associations

| Primitive | Description |
| --- | --- |
| `register_file_association(extension, prog_id, description, command, icon)` | Claims a file type for the current user |
| `unregister_file_association(extension, prog_id)` | Gives it back |

An association is written under `HKCU\Software\Classes`: the extension names the program id, and the
program id carries the words Explorer shows, the icon, and the command that opens the file. The
extension is accepted with or without its leading dot, and every text argument after `prog_id` may
be empty, in which case that part is not written. `command` is the one argument a file type cannot
do without: it is the command line Windows runs, so quote the file placeholder, as in
`"C:\Program Files\MyApp\App.exe" "%1"`. Every key and value written is recorded in the manifest,
so an uninstall removes the file type again. A name carrying a path separator is refused before
anything is written, because it would write outside the classes tree a user's file types belong in.

## Processes

| Primitive | Description |
| --- | --- |
| `is_process_running(name)` | Matches by executable file name |
| `kill_process(name)` | Ends that process |
| `run_detached(command)` | Starts in the background |
| `run_command(command, args)` | Waits for exit, returns the exit code or `-1` |
| `run_command_output(command, args)` | Waits for exit, returns `#{code, stdout, stderr}` |

Every command runs without a console window of its own, and `args` is an array of strings.
`run_command` and `run_command_output` wait for the program, and end it with what it has written so
far when the user stops the task, handing `-1` back to the script; `is_cancelled()` is what tells
that apart from a program that could not be started at all. What `run_command_output` collects is
decoded as UTF-8 where those bytes are valid and in the machine's own code page otherwise, so
`ipconfig` on a Chinese Windows reads as Chinese rather than as replacement characters on both
streams.

## Dependencies and downloads

| Primitive | Description |
| --- | --- |
| `dependency_installed(id)` | Asks the machine whether it has this dependency, by the project's own rule |
| `install_dependency(id)` | Installs it when it is missing, and reports whether the rule says it is there afterwards; install only |
| `download_file(url, path)` | Fetches a URL into `path` |
| `download_file_with_hash(url, path, sha256)` | The same, keeping the file only when its digest matches |
| `sha256_of_file(path)` | The file's SHA-256 in lower-case hexadecimal, empty when it cannot be read |

Both dependency primitives read the project's `dependencies.items`, described in the
[configuration reference](CONFIG_REFERENCE.md#dependencies): the script decides when to check and
whether to install, and the project says what the dependency is and how it is recognized.
`install_dependency` always answers `false` in an uninstall script: removing a product does not
remove a dependency, and does not install anything for another one.

A download goes through the machine's own HTTP stack, so its proxy, certificate store and TLS
settings are the ones a browser on that machine uses, and `https` is offered TLS 1.2. What arrives
is compared with `sha256` before anything else happens: a file that does not match is deleted and
the call answers `false`, so no half of it is left for the next step. Neither download reports
progress; a script that wants the progress bar to say something calls `set_progress()` around it.

## System

| Primitive | Description |
| --- | --- |
| `get_env(name)` | Environment variable, empty string when absent |
| `set_env(name, value)` | Writes one of the user's variables, for the processes that start later |
| `remove_env(name)` | Removes that variable |
| `get_drives()` | Fixed disk roots, for example `["C:\\", "D:\\"]` |
| `get_drive_space(drive)` | `[free, total]`, empty array on failure |
| `shell_notify()` | Notifies the shell to refresh its icon cache |
| `get_config_value(path)` | Reads `installer_config.json`, for example `project.name` |
| `get_current_exe()`, `get_exe_dir()` | Path and directory of the running setup or uninstaller |
| `is_elevated()` | Whether the process is elevated |
| `show_message(title, message)` | Information dialog, drawn in the installer window |
| `show_error(title, message)` | Error dialog, drawn in the installer window |
| `ask_yes_no(title, message)` | Question dialog, returns a bool |
| `run_tracked_uninstall(start, end)` | Removes the product from the manifest; uninstall only |

`set_env` writes `HKCU\Environment`, which is where Windows reads a new process's variables
from: a variable the setup exports into its own environment would die with the setup, and one
written here is still there for the program it installs. The key belongs to Windows and to every
other product on the machine, so the manifest records the value and the uninstall takes that
value back rather than the key, exactly as it does for `...\CurrentVersion\Run`.

`show_message`, `show_error` and `ask_yes_no` draw the product's own card inside the installer
window rather than opening a system message box, from the layout `ui.dialog_layout` names. The card
shows the title the script wrote above its message, and a question carries two answers whose labels
come from the `yes` and `no` keys of the locale file; `ask_yes_no` returns the one that was clicked.
The script waits for that click on the worker thread, so the window keeps painting and keeps taking
clicks while the card is up.

A run with nothing to draw in keeps the system box it used before: a silent install, a script that
starts before the window opens, and a project that ships no dialog layout.

## Example

A minimal install script:

```rhai
let install_path = get_install_path();
set_status_key("status.extracting");
if !extract_payload_with_progress(5.0, 50.0) {
    show_error("Installation Error", "Failed to extract files");
    return;
}
copy_uninstaller();
create_start_menu_shortcut(get_config_value("project.name"),
                           path_join(install_path, get_config_value("install.exe_name")),
                           get_config_value("project.name"));
set_status_key("status.install_complete");
set_progress(100.0);
```

You can find a complete pair in `examples/TapTap/scripts/`.
