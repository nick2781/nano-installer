# Native branch documentation

This branch is an isolated native Win32 experiment. The previous eframe implementation and its
documentation remain on `main` and are intentionally not duplicated here.

- [Native architecture](NATIVE_ARCHITECTURE.md)
- [Windows compatibility and build](WINDOWS_COMPATIBILITY.md)
- [TapTap test project](../examples/TapTap/README.md)

The current runtime packages the complete example and renders the first XML page, but it does not
yet perform installation or uninstallation. Treat generated setups as validation outputs only.
