# Languages

Each language is one UTF-8 JSON file in the `locales` directory, named after the language code:
`en-US.json`, `ja.json`, `zh-CN.json`, and so on.

```json
{
  "install_button": "Install now",
  "version_info": "Version 1.11.3.10000"
}
```

XML layouts reference keys with `@`:

```xml
<Button text="@install_button" ... />
<Label text="@version_info" ... />
```

At startup the runtime loads `<locales_dir>/<default_locale>.json`, where `default_locale` comes
from your configuration and defaults to `zh-CN`. Text is drawn with Unicode APIs; there is no ANSI
code path and no ANSI variant of the runtime.

## Shipping more than one language

Add a language selector to a page and give each entry the matching locale code:

```xml
<Select action="switch_language" ...>
  <Option value="zh-CN" text="简体中文" />
  <Option value="en-US" text="English" />
</Select>
```

Selecting an entry reloads that locale file and redraws the page text immediately. The option value
must match a JSON file name in the locales directory.

## Testing a language without clicking

Pre-select a language for automated screenshots and quick checks:

```powershell
$env:NANO_INSTALLER_TEST_LOCALE = "en-US"
.\examples\TapTap\dist\TapTap_Setup.exe
```

## What is not implemented

- Automatic checks that every locale defines the same key set.
- Warnings for missing keys; a missing key falls back to the key text.
- A separate language pack format. Locales are plain JSON files inside the project.
