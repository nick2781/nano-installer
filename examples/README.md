# Examples

Three folders. Each proves something different.

Only the first looks like a product you would ship. The other two exist so a document or an ABI has
something real behind it.

| Folder | What it is | What it proves |
| --- | --- | --- |
| [`TapTap`](TapTap/README.md) | A complete installer project: XML pages, eleven languages, image assets at two densities, and both scripts | The end-to-end sample. The first-page capture in the READMEs comes from here, `scripts/capture_setup_snapshots.ps1` photographs every page it declares, and the suite inspects the project and places its first page's controls at 192 dpi |
| [`nsis-migration`](nsis-migration/README.md) | Two NSIS scripts — a representative one and one holding every construct NSIS documents — with the same product written as a Nano Installer project | Every row of the migration guide's mapping table. Neither script is runnable and neither installs anything: `scripts/check_nsi_migration.ps1` reads both with `-FailOnUnknown`, so a construct with no row fails the check instead of printing `unknown` |
| [`plugin-c`](plugin-c/README.md) | A plugin written in C, exporting plain `__cdecl` functions through `include/nano_plugin.h` | That the plugin ABI belongs to the header rather than to Rust. The suite installs one setup carrying this DLL and another carrying the Rust sample in `crates/nano-installer-plugin-sample`, and drives both |

Build one from the repository root:

```powershell
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

Two things to know before you copy anything:

- **The artwork is not yours to reuse.** The TapTap name, trademarks, images and copy belong to their
  rights holders. They sit outside this project's licence. Copy the configuration's shape, then
  replace the artwork. `TapTap/README.md` says the same thing at more length.
- **The TapTap payload is not tracked.** `resources.payload_file` names `payload/app.7z`. Neither that
  archive nor the application files behind it are in the repository. A fresh clone has nothing to
  install until you put one there. `TapTap/README.md` says where. When
  `scripts/capture_setup_snapshots.ps1` needs a build for the README screenshots, it writes a
  throwaway placeholder and deletes it again. The other two examples need no payload at all.
