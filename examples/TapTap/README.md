# TapTap CN Installer Example

This is the domestic TapTap installer example built with nano-installer. It demonstrates the full multi-page wizard flow, i18n, custom XML layouts, and DPI-aware asset handling for the CN release.

## Directory Structure

```
examples/TapTap/
  installer_config.json     # Main configuration file
  layouts/                  # XML layout files for each wizard page
    configpage.xml          # Install configuration page (path, options)
    installingpage.xml      # Installation progress page
    finishpage.xml          # Install complete page
    uninstallpage.xml       # Uninstall confirmation page
    uninstallingpage.xml    # Uninstall progress page
    uninstallfinishpage.xml # Uninstall complete page
    msgBox.xml              # Confirmation dialog layout
  assets/                   # Images and icons (supports @2x for HiDPI)
  locales/                  # i18n string files
    zh-CN.json              # Simplified Chinese
    zh-TW.json              # Traditional Chinese
    en-US.json              # English
    ja.json                 # Japanese
    ko.json                 # Korean
    ...                     # Additional languages
  dist/                     # Build output
    TapTap_Setup.exe        # Built installer
  .build/                   # Intermediate build artifacts
```

## Building

From the repository root:

```bash
cargo run --bin nano-installer -- build --project examples/TapTap
```

This will:
1. Parse `installer_config.json` for project settings
2. Load and validate all XML layouts
3. Compile locale files into `.pak` format
4. Package the payload and generate `dist/TapTap_Setup.exe`

## Creating Your Own Installer

Use this example as a template for building your own installer:

### 1. Copy the example directory

```bash
cp -r examples/TapTap examples/MyApp
```

### 2. Edit `installer_config.json`

Update these key fields:

- `project.name` - Your application name
- `project.version` - Your version number
- `project.publisher` - Your company/publisher name
- `install.exe_name` - Your main executable filename
- `install.default_path` - Default install location
- `links` - Your URLs for agreement, privacy policy, etc.

### 3. Customize XML layouts

Each page is defined by an XML file in `layouts/`. Modify the existing files or create new ones:

- Button behavior is driven by the `action` attribute (e.g., `action="install"`, `action="browse_folder"`)
- Text uses locale keys via `{key}` syntax (e.g., `text="{install_button}"`)
- See [XML Layout Guide](../../docs/XML_LAYOUT_GUIDE.md) for the full attribute reference

### 4. Update locale files

Edit `locales/zh-CN.json` (and other language files) to replace TapTap-specific strings with your own.

### 5. Replace assets

Replace images in `assets/` with your own branding. Provide both 1x and @2x versions for HiDPI support:

```
assets/
  logo.png          # Standard resolution
  logo@2x.png       # HiDPI resolution (2x dimensions)
```

### 6. Add your payload

Place your application files in the directory specified by `resources.payload_dir` in the config, or provide a pre-compressed 7z archive via `resources.payload_file`.

### 7. Build

```bash
cargo run --bin nano-installer -- build --project examples/MyApp
```

## Key Files to Modify

| File | What to change |
|------|---------------|
| `installer_config.json` | All project metadata, paths, wizard page flow |
| `layouts/configpage.xml` | Main install page UI (logo, buttons, options) |
| `layouts/finishpage.xml` | Post-install page (launch button, links) |
| `locales/*.json` | All user-facing text strings |
| `assets/*` | Logo, background, button images, icons |

## Documentation

- [Configuration Reference](../../docs/CONFIG_REFERENCE.md) - All config fields explained
- [XML Layout Guide](../../docs/XML_LAYOUT_GUIDE.md) - Layout format and attributes
- [Locale Keys Reference](../../docs/LOCALE_KEYS.md) - All i18n string keys
