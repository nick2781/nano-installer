# Page layout

You list your XML pages in `wizard.pages` for install and `wizard.uninstall_pages` for uninstall.
During a task the wizard shows the second page listed and switches to the last page when it
finishes. A list with a single page stays on that page and reports the outcome in a message box.

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

`Image` and `Icon` draw only when you position them absolutely with left/top/width/height. The
runtime decodes PNG files to PBGRA by WIC and draws them with GDI alpha blending.

## DPI and image density

Coordinates in a layout assume 96 DPI. With `ui.dpi_aware` enabled, the runtime reads the DPI of the
display the window is on and scales the window, coordinates, fonts, hit areas, and corner radius
together. A setup asks for per-monitor awareness, so when you drag the window to a display with a
different scaling factor the runtime lays the page out again for that display; you do not need a
separate layout per scaling level.

Provide a 1x name and let the runtime pick the density:

```text
assets/logo.png
assets/logo@2x.png
```

When the system DPI reaches `ui.dpi_threshold` (144 by default) the runtime prefers the `@2x` file,
otherwise it uses the 1x file, and each falls back to the other when missing. It normalizes a layout
that already mentions `@2x` the same way, so you never maintain two copies of a layout.

## Button

```xml
<Button action="install"
        position="absolute" left="240" top="268" width="240" height="40"
        normal-image="assets/btn_primary.png"
        text="@install_button" font-size="14" font-weight="bold"
        color="#FFFFFFFF" />
```

Supported attributes include `normal-image` and text. The
`file='assets/x.png' dest='...' fade='...'` form draws an image into a sub-rectangle of the control
with 0-255 opacity. A button picks `hover-image`, `pressed-image`, and `disabled-image` for its
states and falls back to `normal-image` when a state image is missing. The runtime paints state
changes into an offscreen bitmap and commits them in one step, so hovering does not flicker.

A button can depend on another control:

```xml
<Checkbox id="terms" checked="false" ... />
<Button action="install" enabled-when="terms:checked" ... />
```

Supported states are `checked`, `unchecked`, `visible`, and `hidden`. While the condition is unmet
the button uses `disabled-image` and registers no click or hover handling; it returns to its normal
states once the condition holds. A button without `enabled-when` is enabled by default, and the
runtime contains no rules tied to specific control names.

## Flow containers

Absolutely positioned `HBox`, `VBox`, and `Content` containers take over their subtree: children no
longer need coordinates, and the container lays them out along the main axis in order. Supported
attributes include fixed and measured sizes, `min-width`/`min-height`, `flex-grow`, `flex-shrink`,
`item-spacing`/`gap`, `padding`, `margin` (including single-side forms such as `margin-top`),
`justify-content`, and `align-items`. `HBox` also accepts `horizontal-align`/`vertical-align`, and
`Content` declares a vertical stack with `layout="vertical"`.

When children exceed the available main-axis space, the container compresses shrinkable items first;
`flex-shrink="0"` keeps a button at its designed width. A `Spacer` with `flex-grow="1"` (or a fixed
`height`) absorbs the remaining space. Content inside a button supports Label, Box, and Image/Icon,
which is how you combine text and icons.

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
left). Every value assumes 96 DPI and scales with the system DPI. Percentages resolve against the
parent's corresponding edge. `align-items="center"` centers on the cross axis: height inside an
`HBox`, width inside a `VBox`, and one item can override that with `align-self="start"` or `"end"`.

`flex-wrap="true"` (or `wrap`) moves an item onto the next line when the current one is full, so a
narrow window reflows instead of squashing its content. With several lines, each line is as tall as
its tallest item, and `justify-content` and `align-self` still apply. Wrapping is off by default: an
overflowing row compresses its shrinkable items first.

`flex-basis` sets the size an item starts from before the container shares out free space, so
`flex-basis="0" flex-grow="1"` takes an equal share next to other flexible items. A container nested
in another one reports the extent its own children need, so a panel does not have to declare a fixed
size. Alongside `left`/`top`, you can pin an absolutely positioned element with `right`/`bottom`, or
with the `inset` shorthand (`inset="8"` or `inset-top`/`inset-right`/`inset-bottom`/`inset-left`),
which measures from the opposite edge.

A checkbox picks `checked-image` or `unchecked-image`, draws localized text, and renders Markdown
link markup as text in the `linkcolor` color. Without a fixed width a checkbox measures to its text,
and a `Spacer` absorbs the remaining space in an `HBox`; wrapping only happens at the available
width limit. Clicking toggles the checkbox, and clicking a link opens its target, so `linkcolor` is
what opts a label into clickable links. See [Links](#links).

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

`background` paints a rounded track; `bar-image` is a full gradient strip that the runtime clips
from the left by percentage. `progress` is the value you author; a running install or uninstall
overrides it with live progress and restores the authored value afterwards.

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

You can edit a TextInput that is not `readonly` in place, with the shortcuts a Windows text field
usually offers:

- Clicking puts a blinking caret where the click landed; typing inserts text, Backspace and Delete
  remove it, and Left/Right/Home/End move the caret.
- Dragging with the left button held selects a run of text, drawn with a light blue highlight, a
  double click selects the word under the pointer, and Ctrl+A selects everything.
- Ctrl+C, Ctrl+X, and Ctrl+V copy, cut, and paste; Ctrl+Z undoes and Ctrl+Y redoes, with a run of
  typing collapsed into a single undo step.
- Ctrl+Backspace and Ctrl+Delete remove a word at a time, and Ctrl+Left and Ctrl+Right step over
  one.
- An IME composition window opens at the caret and the candidate list is placed just below it, so
  East Asian input is composed inside the field rather than in a floating box.
- Typing or pasting while text is selected replaces that selection.

Words are grouped the way Windows groups them: letters, digits, and underscore form a word, runs of
whitespace form their own run, and every other character stands alone, so double-clicking the
backslash in `C:\Program Files` selects just that backslash.

A field you want to be display-only declares `readonly="true"`; it then ignores clicks and typing,
and `pick_directory` treats it as a display field. Values the user types override the
`value`/`value-source` default for the rest of the run, and `disk-free:` bindings that read the same
control recompute immediately.

`status` replaces the authored text with the locale key the runtime publishes for the current step,
which is how progress pages show live status. While no task is running, the runtime keeps the `text`
placeholder.

Colors accept `#RRGGBB` or `#AARRGGBB`; text drawing currently ignores alpha and uses RGB only.

## Visibility

The runtime draws no element when it, or any ancestor, has `visible="false"`.

## Actions

| Action | Behaviour |
| --- | --- |
| `minimize` | Minimizes the window |
| `close` | Closes immediately |
| `close_confirm` | Opens the skinned confirmation drawn from `ui.dialog_layout`, then closes on Yes |
| `pick_directory` | Opens the Windows folder picker and writes the result into a TextInput, see [Folder picker](#folder-picker) |
| `open_url:<key>` | Opens the URL configured under that `links` key |
| `open_url:<https://...>` | Opens the URL as written |
| `install` | Starts the install task, switches to the second page, and reports progress |
| `uninstall` | Starts the uninstall task, switches to the second page, and reports progress |
| `launch_app` | Launches the executable this installation deployed, from its own directory |
| `finish` | Same as `close`, intended for the finish page |
| `switch_language` | Expands the language list and switches the locale on selection |
| `toggle_panel:<id>:show/hide` | Shows or hides a target panel and switches the paired show/hide control |
| `dialog_ok` | Confirms the open dialog: a close question exits the setup, a notice just closes |
| `dialog_cancel` | Dismisses the open dialog and returns to the page under it |

## Dialogs

A confirmation is not a system message box but an ordinary layout. The runtime draws the file named
by `ui.dialog_layout` (default `layouts/msgBox.xml`) in the middle of the window, over a scrim that
separates it from the page. Only the dialog's own controls respond while it is open, so a page
button underneath cannot be clicked by mistake; `Enter` confirms it and `Escape` dismisses it.

A dialog takes its text from the question being asked rather than from the layout, through
`value-source`:

```xml
<Page width="400" height="180" background="#FF2A3844" border-radius="16">
  <Label id="lblMsg" value-source="dialog:message" wrap="true" width="336" />
  <Button id="btnCancel" action="dialog_cancel" value-source="dialog:dismiss"
          visible-with="dismiss" width="160" height="40" />
  <Button id="btnOK" action="dialog_ok" value-source="dialog:accept"
          width="160" height="40" />
</Page>
```

| `value-source` | Value |
| --- | --- |
| `dialog:message` | The question or notice being shown |
| `dialog:accept` | Label of the confirming button |
| `dialog:dismiss` | Label of the dismissing button, empty for a notice |

`visible-with="dismiss"` draws a control only when the dialog offers two answers, which is how one
layout serves both a close question and a notice with a single "OK".

The `height` on `Page` is a minimum. A dialog's text follows the language it is shown in, and the
same sentence can take another line elsewhere; the card then grows to fit and stays centred instead
of pushing its answers out through the bottom edge. Your layout's own `padding` decides how much
room is left below the answers; the example keeps 24 pixels there.

## Links

A `[label](target)` run inside `text`, `value`, or a locale string becomes clickable when the
element also declares `linkcolor`:

```xml
<Label text="@agree_full" linkcolor="#00C4B2" />
```

The runtime resolves the target in this order: an absolute URL (`https://`, `http://`, `mailto:`),
then the project's `links` table in `installer_config.json`, then the historical aliases `agreement`
and `policy`, which map onto `terms_of_service` and `privacy_policy`. A target that resolves to
nothing stays plain text rather than failing.

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
over a clickable area, which is how you promote an icon or a label into a link.

## Folder picker

```xml
<TextInput id="editDir" value-source="config:install.default_path" cursor="text" />
<Image id="editDirIcon" action="pick_directory" target="editDir"
       cursor="hand" src="assets/folder-icon.png"
       position="absolute" left="436" top="12" width="16" height="16" />
```

`action="pick_directory"` opens the shell folder chooser. The runtime writes the chosen path to the
TextInput named by `target`, or, when `target` is absent, to the first writable TextInput on the
page; a `readonly` field is a display field and is only used as a last resort. Because it stores the
value as user input it overrides the `value`/`value-source` default from then on, and `disk-free:`
bindings that read the same control pick up the new path immediately.

## Not implemented yet

- Implicit minimum sizes beyond `min-width`/`min-height`, and `min-height` on a flow container.
- The composition window is repositioned when focus or the caret moves, but not while the user is
  scrolling the page under an active composition.
- An item's cross-axis size is still the container's extent unless the item declares one; there is
  no `stretch`/`baseline` distinction beyond that.
