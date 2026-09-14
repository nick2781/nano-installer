# Native installer source

This branch contains only the native Win32 implementation:

```text
installer/
└── native/
    ├── cli/                 # project parser and setup builder
    ├── src/                 # shared bundle and Win32 runtime
    └── stubs/
        ├── lzma/
        ├── zlib/
        └── uninst/
```

Run `scripts/build.ps1` after changing any source under `installer/native/`. The script rebuilds
the builder and all stubs for the single Win7 SP1+ release baseline before packaging TapTap.
