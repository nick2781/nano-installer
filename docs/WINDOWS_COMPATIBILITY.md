# Windows compatibility

All published executables use one target: `x86_64-win7-windows-msvc`. The minimum supported system
is Windows 7 SP1 x64; the same binaries run on later Windows versions. ANSI, x86, and a separate
Windows 10 build are not provided.

The release build uses:

- the pinned `nightly-2025-11-08` toolchain with `rust-src`;
- `-Z build-std=std,panic_abort`;
- a statically linked CRT from `.cargo/config.toml`;
- Unicode Win32 APIs, WIC, and GDI;
- PE import auditing for the builder, all three stubs, and generated setup.

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

Public artifacts are copied to `target/release/`. `target/x86_64-win7-windows-msvc/` is Cargo's
internal build cache and is not a second product variant.

PE auditing proves the files do not statically import the known blocked Win8/10 APIs. Formal Win7
support still requires execution on a clean Windows 7 SP1 x64 VM, including WIC PNG decoding,
GDI text, window interactions, UAC, extraction, installation, and uninstall tests. SHA-2 update
KB3033929 should be part of the supported machine baseline.
