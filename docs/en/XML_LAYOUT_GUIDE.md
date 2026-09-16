# Page layout

Pages are XML files listed in `wizard.pages` for install and `wizard.uninstall_pages` for
uninstall. During a task the wizard shows the second page listed and switches to the last page when
it finishes. A list with a single page stays on that page and reports the outcome in a message box.

Layouts use absolute positions plus optional flow containers. Only the attributes marked as
supported below take effect.

## Page

```xml
<Page width="720" height="450"
      background-image="assets/bg_main.png"
      border-radius="12">
  ...
</Page>
```

| Attribute | Status | Effect |
| --- | --- | --- |
| `width`, `height` | Supported | Client area size, defaults to 720x450 |
| `background-image` | Supported | Decoded with WIC and stretched to the client area |
| `border-radius` | Supported | Rounds the window through a Win32 region |
| `background` and border attributes | Not drawn | Use a background image for now |

## Images

```xml
<Image src="assets/logo.png" position="absolute"
       left="260" top="100" width="200" height="58" />
```

`Image` and `Icon` draw only when they are absolutely positioned with left/top/width/height. PNG
files are decoded to PBGRA by WIC and drawn with GDI alpha blending.

## DPI and image density

Coordinates in a layout assume 96 DPI. With `ui.dpi_aware` enabled, the runtime reads the system DPI
and scales the window, coordinates, fonts, hit areas, and corner radius together.

Provide a 1x name and let the runtime pick the density:

```text
assets/logo.png
assets/logo@2x.png
```

When the system DPI reaches `ui.dpi_threshold` (144 by default) the `@2x` file is preferred,
otherwise the 1x file is used, and each falls back to the other when missing. A layout that already
mentions `@2x` is normalized the same way, so you never maintain two copies of a layout.

## Button

```xml
<Button action="install"
        position="absolute" left="240" top="268" width="240" height="40"
        normal-image="assets/btn_primary.png"
        text="@install_button" font-size="14" font-weight="bold"
        color="#FFFFFFFF" />
```

Supported attributes include `normal-image` and text. The `file='assets/x.png' dest='...' fade='...'`
form draws an image into a sub-rectangle of the control with 0-255 opacity. Buttons pick
`hover-image`, `pressed-image`, and `disabled-image` for their states and fall back to
`normal-image` when a state image is missing. State changes are painted into an offscreen bitmap and
committed in one step, so hovering does not flicker.

A button can depend on another control:

```xml
<Checkbox id="terms" checked="false" ... />
<Button action="install" enabled-when="terms:checked" ... />
```

Supported states are `checked`, `unchecked`, `visible`, and `hidden`. While the condition is unmet
the button uses `disabled-image` and registers no click or hover handling; it returns to its normal
states when the condition holds. Buttons without `enabled-when` are enabled by default, and the
runtime contains no rules tied to specific control names.

## Flow containers

Absolutely positioned `HBox`, `VBox`, and `Content` containers take over their subtree: children no
longer need coordinates and are laid out along the main axis in order. Supported attributes include
fixed and measured sizes, `min-width`/`min-height`, `flex-grow`, `flex-shrink`,
`item-spacing`/`gap`, `padding`, `margin` (including single-side forms such as `margin-top`),
`justify-content`, and `align-items`. `HBox` also accepts `horizontal-align`/`vertical-align`, and
`Content` declares a vertical stack with `layout="vertical"`.

When children exceed the available main-axis space, shrinkable items are compressed first;
`flex-shrink="0"` keeps a button at its designed width. A `Spacer` with `flex-grow="1"` (or a fixed
`height`) absorbs the remaining space. Content inside a button supports Label, Box, and
Image/Icon, which is how text and icons are combined.

### Sizes and spacing

`width` and `height` accept pixels or a percentage of the parent:

```xml
<VBox position="absolute" left="0" top="0" width="100%" height="100%" padding="1">
  <Spacer height="70" />
  <HBox width="100%" padding="0 32" justify-content="center">
    <Label text="@uninstall_confirm" width="100%" text-align="center" />
  </HBox>
  <Spacer flex-grow="1" />
</VBox>
```

`padding` and `margin` accept 1-4 space-separated values with CSS semantics (top, right, bottom,
left). All values assume 96 DPI and scale with the system DPI. Percentages resolve against the
parent's corresponding edge. `align-items="center"` centers on the cross axis: height inside an
`HBox`, width inside a `VBox`.

Checkboxes pick `checked-image` or `unchecked-image`, draw localized text, and render Markdown link
markup as text in the `linkcolor` color. Without a fixed width they measure to their text, and a
`Spacer` absorbs the remaining space in an `HBox`; wrapping only happens at the available width
limit. Clicking toggles the checkbox; link clicks are not wired yet.

## Label and Select

- Absolutely positioned Label: supports `text`/`value`, font-size, font-weight, color, and
  `textalign=center`.
- A Select with `action="switch_language"`: selects the option matching the current locale, draws a
  DPI-aware arrow, and expands the options declared in XML. Selecting one reloads the locale JSON
  and redraws the page text.
- Select background, border, and keyboard handling are not implemented yet.

## ProgressBar

```xml
<ProgressBar id="slrProgress" position="absolute" left="72" top="326"
             width="576" height="10" progress="0" border-radius="5"
             bar-image="assets/bar_installing.png"
             background="#FF4C5868" />
```

`background` paints a rounded track and `bar-image` is a full gradient strip clipped from the left
by percentage. `progress` is the authored value; a running install or uninstall overrides it with
live progress and restores the authored value afterwards.

## Live value bindings

TextInput and Label can bind to runtime data through `value-source`:

```xml
<TextInput id="installDir" value-source="config:install.default_path" />
<Label text="@required_space"
       value-source="config:install.required_space_mb" value-format="size-mb" />
<Label text="@available_space"
       value-source="disk-free:installDir" value-format="size" />
<Label id="progress_pos" text="@installing_text" value-source="status" />
```

Everything after `config:` is a dot-separated path into `installer_config.json`. `disk-free:` takes
a TextInput id; the runtime extracts the Windows volume root from that path and queries free space
with `GetDiskFreeSpaceExW`. `size-mb` turns the configured MiB value into readable text, and `size`
formats bytes as B/KB/MB/GB/TB. TextInput is read-only display today; the folder picker and user
editing are not wired.

`status` replaces the authored text with the locale key the runtime publishes for the current step,
which is how progress pages show live status. While no task is running, the `text` placeholder is
kept.

Colors accept `#RRGGBB` or `#AARRGGBB`; text drawing currently ignores alpha and uses RGB only.

## Visibility

An element is not drawn when it, or any ancestor, has `visible="false"`.

## Actions

| Action | Behaviour |
| --- | --- |
| `minimize` | Minimizes the window |
| `close`, `close_confirm` | Closes immediately; the confirmation dialog is not implemented |
| `install` | Starts the install task, switches to the second page, and reports progress |
| `uninstall` | Starts the uninstall task, switches to the second page, and reports progress |
| `launch_app` | Launches the executable this installation deployed, from its own directory |
| `finish` | Same as `close`, intended for the finish page |
| `switch_language` | Expands the language list and switches the locale on selection |
| `toggle_panel:<id>:show/hide` | Shows or hides a target panel and switches the paired show/hide control |

## Not implemented yet

- Nested flow layout: an inner container keeps its fixed size and does not take part in the outer
  flex calculation.
- `flex-basis`, `flex-wrap`, `align-self`, `inset`, and implicit minimum sizes beyond `min-height`.
- Text measured for flex uses single-line width; multi-line height with `wrap="true"` is not part
  of a parent's measurement.
- The `close_confirm` dialog, link clicks, the folder picker, and keyboard handling for Select.
