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
| `background` | Supported | Base colour, drawn under `background-image` |
| `border-color`, `border-width` | Supported | Outline drawn inside the page edges |

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
`HBox`, width inside a `VBox`, and one item can override that with `align-self="start"` or `"end"`.

`flex-wrap="true"` (or `wrap`) moves an item onto the next line when the current one is full, so a
narrow window reflows instead of squashing its content. With several lines, each line is as tall as
its tallest item, and `justify-content` and `align-self` still apply. Wrapping is off by default:
an overflowing row compresses its shrinkable items first.

`flex-basis` sets the size an item starts from before free space is shared out, so
`flex-basis="0" flex-grow="1"` takes an equal share next to other flexible items. A container nested
in another one reports the extent its own children need, so a panel does not have to declare a fixed
size. Alongside `left`/`top`, an absolutely positioned element can be pinned with `right`/`bottom`,
or with the `inset` shorthand (`inset="8"` or `inset-top`/`inset-right`/`inset-bottom`/`inset-left`),
which measures from the opposite edge.

Checkboxes pick `checked-image` or `unchecked-image`, draw localized text, and render Markdown link
markup as text in the `linkcolor` color. Without a fixed width they measure to their text, and a
`Spacer` absorbs the remaining space in an `HBox`; wrapping only happens at the available width
limit. Clicking toggles the checkbox, and clicking a link opens its target, so `linkcolor` is what
opts a label into clickable links. See [Links](#links).

## Label and Select

- Absolutely positioned Label: supports `text`/`value`, font-size, font-weight, color, and
  `textalign=center`.
- A Select with `action="switch_language"`: selects the option matching the current locale, draws a
  DPI-aware arrow over its `background` fill and `border-color` outline, and expands the options
  declared in XML. Selecting one reloads the locale JSON and redraws the page text.
- While the menu is open the keyboard takes over: Up and Down move the highlight (wrapping at both
  ends), Enter switches to the highlighted language, and Escape closes the menu without leaving the
  page. Opening the menu highlights the language in use. The highlight uses
  `popup-highlight-background`, falling back to `popup-selected-background`.

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
formats bytes as B/KB/MB/GB/TB.

A TextInput that is not `readonly` is editable in place, with the shortcuts a Windows text field
usually offers:

- Clicking puts a blinking caret where the click landed; typing inserts text, Backspace and Delete
  remove it, and Left/Right/Home/End move the caret.
- Dragging with the left button held selects a run of text, drawn with a light blue highlight, a
  double click selects the word under the pointer, and Ctrl+A selects everything.
- Ctrl+C, Ctrl+X, and Ctrl+V copy, cut, and paste; Ctrl+Z undoes and Ctrl+Y redoes, with a run of
  typing collapsed into a single undo step.
- Ctrl+Backspace and Ctrl+Delete remove a word at a time, and Ctrl+Left and Ctrl+Right step over
  one.
- Typing or pasting while text is selected replaces that selection.

Words are grouped the way Windows groups them: letters, digits, and underscore form a word, runs of
whitespace form their own run, and every other character stands alone, so double-clicking the
backslash in `C:\Program Files` selects just that backslash.

A field a project wants to be display-only declares `readonly="true"`; it then ignores clicks and
typing, and `pick_directory` treats it as a display field. Values typed by the user override the
`value`/`value-source` default for the rest of the run, and `disk-free:` bindings that read the same
control recompute immediately.

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
| `close` | Closes immediately |
| `close_confirm` | Asks the question from the `close_confirm_message` locale key first, then closes on Yes |
| `pick_directory` | Opens the Windows folder picker and writes the result into a TextInput, see [Folder picker](#folder-picker) |
| `open_url:<key>` | Opens the URL configured under that `links` key |
| `open_url:<https://...>` | Opens the URL as written |
| `install` | Starts the install task, switches to the second page, and reports progress |
| `uninstall` | Starts the uninstall task, switches to the second page, and reports progress |
| `launch_app` | Launches the executable this installation deployed, from its own directory |
| `finish` | Same as `close`, intended for the finish page |
| `switch_language` | Expands the language list and switches the locale on selection |
| `toggle_panel:<id>:show/hide` | Shows or hides a target panel and switches the paired show/hide control |

## Links

A `[label](target)` run inside `text`, `value`, or a locale string becomes clickable when the element
also declares `linkcolor`:

```xml
<Label text="@agree_full" linkcolor="#00C4B2" />
```

The target is resolved in this order: an absolute URL (`https://`, `http://`, `mailto:`), then the
project's `links` table in `installer_config.json`, then the historical aliases `agreement` and
`policy`, which map onto `terms_of_service` and `privacy_policy`. A target that resolves to nothing
stays plain text rather than failing.

```json
"links": {
  "terms_of_service": "https://www.taptap.cn/doc/terms/",
  "privacy_policy": "https://www.taptap.cn/doc/privacy-policy/",
  "help": "https://www.taptap.cn/help"
}
```

A `Button` or any other element can also link through `action="open_url:help"`, which uses the same
table, or `action="open_url:https://..."` with the URL written out in full.

## Click targets and the pointer

Buttons, selects, and any element that declares an `action` respond to clicks and turn the pointer
into a hand while it is over them. A control with no `action` is inert even when it is positioned
over a clickable area, which is how an icon or a label is promoted into a link.

## Folder picker

```xml
<TextInput id="editDir" value-source="config:install.default_path" cursor="text" />
<Image id="editDirIcon" action="pick_directory" target="editDir"
       cursor="hand" src="assets/folder-icon.png"
       position="absolute" left="436" top="12" width="16" height="16" />
```

`action="pick_directory"` opens the shell folder chooser. The chosen path is written to the
TextInput named by `target`, or, when `target` is absent, to the first writable TextInput on the
page; a `readonly` field is a display field and is only used as a last resort. Because the value is
stored as user input it overrides the `value`/`value-source` default from then on, and `disk-free:`
bindings that read the same control pick up the new path immediately.

## Not implemented yet

- Implicit minimum sizes beyond `min-width`/`min-height`, and `min-height` on a flow container.
- A text field has no IME composition window, so East Asian input relies on the system IME's own
  candidate display.
- An item's cross-axis size is still the container's extent unless the item declares one; there is
  no `stretch`/`baseline` distinction beyond that.
