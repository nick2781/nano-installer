/*
    sample.c -- a plugin written in C, against include/nano_plugin.h.

    This is the same ABI as the Rust sample next to it in `crates/`, written the
    way most NSIS plugin authors would write one: a plain DLL, exported
    functions, no runtime of its own beyond the C library.

    Build it with the MSVC tools on the PATH (a developer command prompt), from
    the repository root:

        cl /nologo /LD /MT /O2 /W4 /I include examples\\plugin-c\\sample.c ^
           /Fo:target\\plugin-c\\ /Fd:target\\plugin-c\\plugin-c.pdb ^
           /Fe:target\\debug\\plugin-c.dll

    `/MT` links the C runtime statically on purpose: a plugin that used the
    dynamic universal CRT would need an update installed on Windows 7 SP1, and a
    setup that installs on a plain Windows 7 SP1 is the point of this runtime.
*/

#include <stdint.h>
#include <wchar.h>

#include "nano_plugin.h"

/* The argument at `index`, or an empty string when the script passed fewer. */
static const wchar_t *argument(int32_t argc, const wchar_t *const *argv, int32_t index)
{
    if (argv == NULL || index < 0 || index >= argc) {
        return L"";
    }
    return argv[index];
}

/* How long a string is, without pulling in the wide-character C library. */
static int32_t length_of(const wchar_t *value)
{
    int32_t length = 0;
    if (value == NULL) {
        return 0;
    }
    while (value[length] != 0) {
        length++;
    }
    return length;
}

/* Whether this host speaks an ABI this plugin knows. */
static int understood(const nano_plugin_host *host)
{
    return host != NULL && host->abi_version == NANO_PLUGIN_ABI_VERSION &&
           host->struct_size >= (uint32_t)sizeof(nano_plugin_host);
}

static int32_t push(nano_plugin_host *host, const wchar_t *value)
{
    if (host->push == NULL) {
        return NANO_PLUGIN_FAILED;
    }
    return host->push(host->host, value);
}

static void say(nano_plugin_host *host, int32_t level, const wchar_t *message)
{
    if (host->log != NULL) {
        host->log(host->host, level, message);
    }
}

/* A whole number the way a script reads it, formatted here rather than with the
   C library's printf, so the sample needs no format machinery at all. */
static void format_number(wchar_t *buffer, int32_t size, long long value)
{
    wchar_t digits[32];
    int32_t used = 0;
    int32_t written = 0;
    int negative = value < 0;
    unsigned long long magnitude = negative ? (unsigned long long)(-(value + 1)) + 1ULL
                                            : (unsigned long long)value;

    if (magnitude == 0) {
        digits[used++] = L'0';
    }
    while (magnitude > 0 && used < 32) {
        digits[used++] = (wchar_t)(L'0' + (int)(magnitude % 10));
        magnitude /= 10;
    }
    if (negative && written < size - 1) {
        buffer[written++] = L'-';
    }
    while (used > 0 && written < size - 1) {
        buffer[written++] = digits[--used];
    }
    buffer[written] = 0;
}

/* A number a script passed, or zero when it did not pass one. */
static long long number_of(const wchar_t *value)
{
    long long total = 0;
    int negative = 0;
    int32_t index = 0;

    if (value == NULL) {
        return 0;
    }
    if (value[0] == L'-') {
        negative = 1;
        index = 1;
    }
    for (; value[index] >= L'0' && value[index] <= L'9'; index++) {
        total = total * 10 + (value[index] - L'0');
    }
    return negative ? -total : total;
}

/* sample-c::Hello -- one value back, which is what most calls answer with. */
NANO_PLUGIN_EXPORT int32_t NANO_PLUGIN_CALL Hello(nano_plugin_host *host, int32_t argc, const wchar_t *const *argv)
{
    (void)argc;
    (void)argv;
    if (!understood(host)) {
        return 1;
    }
    return push(host, L"hello from C");
}

/* sample-c::Sum -- adds up what it was given, in order. */
NANO_PLUGIN_EXPORT int32_t NANO_PLUGIN_CALL Sum(nano_plugin_host *host, int32_t argc, const wchar_t *const *argv)
{
    long long total = 0;
    wchar_t answer[32];
    int32_t index;

    if (!understood(host)) {
        return 1;
    }
    for (index = 0; index < argc; index++) {
        total += number_of(argument(argc, argv, index));
    }
    format_number(answer, (int32_t)(sizeof(answer) / sizeof(answer[0])), total);
    return push(host, answer);
}

/* sample-c::Describe -- reports the argument count and the installation
   directory, which is how a case sees that the services arrive in C too. */
NANO_PLUGIN_EXPORT int32_t NANO_PLUGIN_CALL Describe(nano_plugin_host *host, int32_t argc, const wchar_t *const *argv)
{
    wchar_t answer[64];
    const wchar_t *directory;

    (void)argv;
    if (!understood(host)) {
        return 1;
    }
    directory = host->install_dir != NULL ? host->install_dir(host->host) : L"";
    format_number(answer, (int32_t)(sizeof(answer) / sizeof(answer[0])), argc);
    if (push(host, answer) != NANO_PLUGIN_OK) {
        return 1;
    }
    return push(host, directory == NULL ? L"" : directory);
}

/* sample-c::Install -- writes a file and a registry value through the host, so
   both are recorded and the uninstall takes them back.

   Arguments: the key to write into, the name of the value, and the text. */
NANO_PLUGIN_EXPORT int32_t NANO_PLUGIN_CALL Install(nano_plugin_host *host, int32_t argc, const wchar_t *const *argv)
{
    const wchar_t *key;
    const wchar_t *name;
    const wchar_t *value;

    if (!understood(host)) {
        return 1;
    }
    key = argument(argc, argv, 0);
    name = argument(argc, argv, 1);
    value = argument(argc, argv, 2);
    if (length_of(key) == 0) {
        say(host, NANO_PLUGIN_LOG_ERROR, L"install: the first argument is the registry key to write");
        return 2;
    }
    if (host->write_file == NULL ||
        host->write_file(host->host, L"c-plugin.txt", L"written by the C plugin\n") != NANO_PLUGIN_OK) {
        say(host, NANO_PLUGIN_LOG_ERROR, L"install: write_file failed");
        return 3;
    }
    if (host->write_registry == NULL ||
        host->write_registry(host->host, key, name, L"string", value) != NANO_PLUGIN_OK) {
        say(host, NANO_PLUGIN_LOG_ERROR, L"install: write_registry failed");
        return 3;
    }
    say(host, NANO_PLUGIN_LOG_INFO, L"install: the file and the value are written");
    return push(host, L"installed by C");
}

/* sample-c::Refuse -- explains itself and fails, with a code of its own. */
NANO_PLUGIN_EXPORT int32_t NANO_PLUGIN_CALL Refuse(nano_plugin_host *host, int32_t argc, const wchar_t *const *argv)
{
    (void)argc;
    (void)argv;
    if (!understood(host)) {
        return 1;
    }
    say(host, NANO_PLUGIN_LOG_ERROR, L"the C plugin was asked to refuse");
    return 7;
}
