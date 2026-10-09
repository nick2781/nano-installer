# Languages

One language is one UTF-8 JSON file in the `locales` directory. The file name is the language code:
`en-US.json`, `ja.json`, `zh-CN.json`, and so on.

```json
{
  "install_button": "Install now",
  "version_info": "Version 1.11.3.10000"
}
```

Your XML layouts reference keys with `@`:

```xml
<Button text="@install_button" ... />
<Label text="@version_info" ... />
```

At startup the runtime loads `<locales_dir>/<default_locale>.json`. `default_locale` comes from your
configuration and defaults to `zh-CN`. Text is drawn with Unicode APIs, so there is no ANSI code
path. The runtime has no ANSI variant either.

## Shipping more than one language

Add a language selector to a page. Give each entry the matching locale code:

```xml
<Select action="switch_language" ...>
  <Option value="zh-CN" text="简体中文" />
  <Option value="en-US" text="English" />
</Select>
```

Pick an entry and the runtime reloads that locale file right away. It redraws the page text too. Each
option value must match a JSON file name in the locales directory.

## Testing a language without clicking

Pre-select a language for automated screenshots and quick checks:

```powershell
$env:NANO_INSTALLER_TEST_LOCALE = "en-US"
.\examples\TapTap\dist\TapTap_Setup.exe
```

## What the build checks for you

Every build compares your locale files against the default locale. It also compares them against the
keys your pages actually ask for. Gaps go into the builder's warning list:

- A locale that is missing text the default locale defines. The key names are listed.
- A language named in `localization.supported_locales` with no matching JSON file.

The build reports only keys the default locale answers. So a page may deliberately ask for a key a
particular translation leaves out. At run time a missing key still falls back to the default locale.
A partly translated installer runs instead of showing an empty label.

## What is not implemented

- No separate language pack format. Locales are plain JSON files inside the project.
- Nothing compares two translations against each other. Only the default locale is the reference.
