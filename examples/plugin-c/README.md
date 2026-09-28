# A plugin in C

`sample.c` is the plugin ABI written the way most NSIS plugin authors would write
one: a DLL built from C, exporting functions. It is the same contract as
`crates/nano-installer-plugin-sample`, which is the Rust sample, and the suite's
`a_plugin_written_in_c_is_called_the_same_way` case installs a setup that ships
this DLL and drives every function in it — so the two implementations hold each
other to `include/nano_plugin.h`, and neither one's compiler decides what the ABI
is.

## Building it

From the repository root, with the MSVC tools on the PATH (a developer command
prompt):

```bat
cl /nologo /LD /MT /O2 /W4 /I include examples\plugin-c\sample.c ^
   /Fo:target\plugin-c\ /Fd:target\plugin-c\plugin-c.pdb ^
   /Fe:target\debug\plugin-c.dll
```

Three parts of that line are worth copying rather than retyping.

`/MT` links the C runtime statically on purpose: a plugin that used the dynamic
universal CRT would need an update installed before it ran on Windows 7 SP1, and
a setup that installs on a plain Windows 7 SP1 is the point of this runtime.

`/I include` is where `nano_plugin.h` lives. That header is the whole SDK: a
project of your own copies it into its own tree and nothing else comes with it.

`/Fo:` and `/Fd:` keep the object file and the debug database out of the source
directory, which is what makes the build repeatable in a tree that keeps its
sources clean.

`scripts/run_e2e_setup.ps1` runs the same command for the suite, and finds the
tools through `vswhere` when they are not on the PATH already; a machine without
MSVC skips the C cases and says so.

## What it exports

| Export | What it does |
| --- | --- |
| `Hello` | Answers one value |
| `Sum` | Adds up the arguments it was given |
| `Describe` | Pushes the argument count and the installation directory |
| `Install` | Writes a file and a registry value through the host |
| `Refuse` | Explains itself and fails with a code of its own |

A script calls them by DLL name: `plugin_call("plugin-c::Hello", [])`. The
`NANO_PLUGIN_EXPORT` on each definition is what puts the name in the export
table, and forgetting it is silent — the DLL builds, loads, and then has no
export by that name, which is why the header carries the macro next to the
signature it belongs to.
