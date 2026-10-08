# Migrating from NSIS

This folder is the worked example of the migration guide:
[English](../../docs/en/MIGRATION_FROM_NSIS.md) ·
[简体中文](../../docs/zh-CN/MIGRATION_FROM_NSIS.md).

| Path | What it is |
| --- | --- |
| `legacy.nsi` | A representative NSIS script — one of every construct a real installer uses |
| `instructions.nsi` | One line for every instruction, attribute and header macro NSIS documents |
| `migrated/` | The same product as a Nano Installer project: configuration, pages, languages and the two scripts |

`legacy.nsi` is not runnable and installs nothing: there is no `app\`, `docs\`,
`samples\`, icon or licence file next to it. It exists so every row of the
guide's mapping table has something behind it, and so the checker has something
to classify:

```powershell
.\scripts\check_nsi_migration.ps1 -Script .\examples\nsis-migration\legacy.nsi
```

`instructions.nsi` is not runnable either, and does not try to be: it is the
NSIS language written out one construct per line, so the guide's table can be
held to the language rather than to the one script next to it. CI runs the
checker over both with `-FailOnUnknown`, which turns a construct the table has
no row for into a failing check instead of a line a reader has to interpret.

```powershell
.\scripts\check_nsi_migration.ps1 -Script .\examples\nsis-migration\instructions.nsi
```

`migrated/` is a real project of the shape the guide builds up. Building it
needs the payload it names (`payload/app.zip`, holding `LegacyApp.exe`) and the
launcher icon it names (`assets/app.ico`); neither is tracked, for the same
reason `examples/TapTap/payload/app.7z` is not. CI checks the configuration
against the settings the build reads, so the example cannot drift into a key the
build ignores.

The pages carry no artwork at all — they are drawn from colours, text and
buttons — so the example can be read and built without shipping bitmaps.
