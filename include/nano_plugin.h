/*
    nano_plugin.h -- the ABI a third-party plugin is written against.

    A plugin is a DLL that a project ships in the directory its configuration
    names as `resources.plugins_dir`. A project script calls it by name:

        plugin_call("sample::Add", ["1", "2"])   // dll::function

    The runtime loads `sample.dll` out of the bundle on first use, calls the
    export named `Add` with the signature below, and hands back the values the
    plugin pushed.

    Three rules make up the whole contract:

      * Arguments and answers are text. The script passes an array of strings,
        the plugin calls `push` for each value it wants back. Text is UTF-16 and
        the host copies it, so a plugin never has to care who frees what.
      * A plugin writes through the host, not around it. `write_file` and
        `write_registry` record what they write the same way the script
        primitives do, so the uninstaller takes it back. A plugin that writes
        with the Win32 API instead leaves the installation with files and keys
        nothing will remove.
      * Nothing here is allowed to be a guess. The host fills in `abi_version`
        and `struct_size`; a plugin that does not know the version must return
        non-zero rather than read fields that may not be there. New services are
        appended to `nano_plugin_host`, and the three parameters of a plugin
        function never change.

    This file is the specification; docs/{en,zh-CN}/PLUGIN_API.md is the same
    contract written out, with an example and the failure modes.

    Not in this ABI, deliberately: no plugin can add a script primitive, replace
    a page, or hook the wizard's own controls. A plugin is native code inside the
    installer process, so it can call the Win32 API directly for anything it only
    wants to read or check; what it cannot do is put a file on the machine
    without the installation knowing.
*/

#ifndef NANO_PLUGIN_H
#define NANO_PLUGIN_H

#include <stdint.h>
#include <wchar.h>

/* The ABI this header describes. The host exports the version it speaks before
   calling anything, and a plugin refuses a version it does not know. */
#define NANO_PLUGIN_ABI_VERSION 1u

/* Levels for `log`, matching the installer's own log. */
#define NANO_PLUGIN_LOG_ERROR 0
#define NANO_PLUGIN_LOG_WARN 1
#define NANO_PLUGIN_LOG_INFO 2

/* Service results. A service returns NANO_PLUGIN_OK or NANO_PLUGIN_FAILED; the
   reason for a failure is in `error_text` until the next service call. */
#define NANO_PLUGIN_OK 0
#define NANO_PLUGIN_FAILED 1

/* The calling convention of every export. On x64 Windows there is only one
   convention, so this matters only to a 32-bit compiler that never builds a
   plugin for this runtime. */
#if defined(_MSC_VER) || defined(__MINGW32__)
#define NANO_PLUGIN_CALL __cdecl
#else
#define NANO_PLUGIN_CALL
#endif

typedef struct nano_plugin_host {
    /* Filled in by the host before the plugin function is called. A plugin must
       check `abi_version` first: NANO_PLUGIN_ABI_VERSION is the version this
       header was written for, and a host that answers anything else is a host
       whose later fields cannot be trusted. `struct_size` says how much of this
       structure the host actually built, so a plugin built against a newer
       header can tell whether a field it wanted exists. */
    uint32_t struct_size;
    uint32_t abi_version;

    /* Opaque. Hand this back to every service below. */
    void *host;

    /* Adds one value to the answer the script reads. The host copies the string
       before this returns, so the plugin may free or reuse its buffer. Returns
       NANO_PLUGIN_OK, or NANO_PLUGIN_FAILED when the run has no room for it. */
    int32_t(NANO_PLUGIN_CALL *push)(void *host, const wchar_t *value);

    /* Writes one line into the run's log, which is the file a failed install
       leaves behind. Use NANO_PLUGIN_LOG_ERROR to explain a failure: the host
       puts the plugin's own words next to the code it returned. */
    void(NANO_PLUGIN_CALL *log)(void *host, int32_t level, const wchar_t *message);

    /* The wizard window, or 0 when this run has no window (a silent install, or
       one whose window is not up yet). A plugin may parent its own dialog to it.
       Whatever a plugin draws there is the plugin's own: the wizard's layout,
       its high-contrast palette and its screen-reader description do not reach
       into a window the plugin created. */
    void *(NANO_PLUGIN_CALL *window)(void *host);

    /* Whether the user has asked this run to stop. A plugin that loops, waits,
       or works through a list should ask between steps and return early. */
    int32_t(NANO_PLUGIN_CALL *cancelled)(void *host);

    /* The directory the product is installed into. Valid until this call
       returns; copy it if the plugin needs it later. */
    const wchar_t *(NANO_PLUGIN_CALL *install_dir)(void *host);

    /* Writes a text file into the installation and records it, so uninstalling
       removes it. `path` is absolute or relative to the installation directory,
       and a path outside that directory is refused: a plugin does not write
       where the uninstaller will not look. Text is UTF-16 and is written as
       UTF-8, which is what the runtime's own `write_file` primitive writes. */
    int32_t(NANO_PLUGIN_CALL *write_file)(void *host, const wchar_t *path,
                                          const wchar_t *text);

    /* Writes one registry value and records it the way the script primitives
       record theirs: the uninstaller takes back this value, or the whole key
       when the key is this product's own. `kind` is one of "string", "expand",
       "dword" or "qword"; `value` is the text to store, or the number for the
       two numeric kinds. Anything else is refused, and so is a key this product
       does not own. */
    int32_t(NANO_PLUGIN_CALL *write_registry)(void *host, const wchar_t *key,
                                              const wchar_t *name,
                                              const wchar_t *kind,
                                              const wchar_t *value);

    /* Why the last service call failed, as a sentence a log can carry. Valid
       until the next service call. Never null. */
    const wchar_t *(NANO_PLUGIN_CALL *error_text)(void *host);
} nano_plugin_host;

/*
    The signature of every exported function a script can call.

    `argc` is the number of arguments the script wrote and `argv` holds them in
    that order, each UTF-16 and NUL-terminated, owned by the host and valid only
    for the duration of the call. Return 0 for success: any other value fails the
    script's call, and the host reports the code together with the last line the
    plugin logged.

    A plugin must not let an exception or a panic escape this function: the
    runtime has no way to carry it, and the installer process is what would end.
    That is measured rather than assumed -- Rust aborts where a panic would cross
    the boundary of a function like this one, so an installer whose plugin
    panicked ends as a crash with nothing in its log to say why.
*/
typedef int32_t(NANO_PLUGIN_CALL *nano_plugin_function)(nano_plugin_host *host,
                                                       int32_t argc,
                                                       const wchar_t *const *argv);

#endif /* NANO_PLUGIN_H */
