# Agent Notes

## Build Boundary

- If you change native runtime or stub source code under `installer/native/`, rebuild the builder and all three native stubs before rebuilding any setup package.
  - This means rebuilding `nano-installer-native-x64`, `native-lzma-x64`, `native-zlib-x64`, and `native-uninst-x64` for `x86_64-win7-windows-msvc` in `--release`.
- If you only change assets, layouts, locales, or config under `examples/`, you do not need to rebuild stubs.
  - In that case, only rerun the setup build for the target example project.

## Practical Rule

- `installer/native/**` changed: rebuild the native builder/stubs first, then rebuild the setup package.
- `examples/**` only changed: rebuild the setup package only.
