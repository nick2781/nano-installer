# TapTap sample project

This is the repository's end-to-end sample: a complete installer project used to validate XML
pages, translations, image assets, payload packaging, and the Windows 7 SP1 runtime.

## Asset ownership

The TapTap name, trademarks, images, and copy under `examples/TapTap` belong to
易玩（上海）网络科技有限公司 and their respective rights holders. They are used only to develop,
test, and validate `nano-installer`, are outside this project's open-source license, and are not
product assets you may reuse.

## Contents

```text
TapTap/
├── installer_config.json
├── assets/                     backgrounds, buttons, icons
├── layouts/                    XML pages
├── locales/                    one JSON file per language
├── scripts/                    install.rhai and uninstall.rhai
└── payload/app.7z              not stored in the repository
```

The payload is excluded by `.gitignore` (`*.7z`). Put a 7z archive at
`examples/TapTap/payload/app.7z` before building.

## Build it

From the repository root:

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

The toolbar tools land in `target/release/`; the sample setup lands in
`examples/TapTap/dist/TapTap_Setup.exe` and is for local validation only, never a release artifact.

## Use it as a starting point

Copy the directory and replace at least:

- product information, install path, executable, registry key, and output names in
  `installer_config.json`
- icons and brand assets in `assets/`
- page text keys, links, and product interactions in `layouts/`
- all user-visible strings in `locales/`
- product-specific install and uninstall behaviour in `scripts/`
- your application files in `payload/app.7z`

The runtime already unpacks the payload, runs install and uninstall tasks with progress pages, and
executes `scripts/install.rhai` and `scripts/uninstall.rhai`; see the
[script API](../../docs/en/SCRIPT_API.md). There is no code signing yet, so it cannot be used for a
production release.
