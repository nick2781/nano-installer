# Agent Notes

## Build Boundary

- If you change installer runtime or stub source code under `installer/`, you must rebuild the related stubs before rebuilding any setup package.
  - At minimum this means rebuilding `nano-installer-lzma`, `uninst`, and `nano-installer-cli` in `--release` when the packaged setup depends on them.
- If you only change assets, layouts, locales, or config under `examples/`, you do not need to rebuild stubs.
  - In that case, only rerun the setup build for the target example project.

## Practical Rule

- `installer/**` changed: rebuild stubs first, then rebuild the setup package.
- `examples/**` only changed: rebuild the setup package only.
