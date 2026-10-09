# Page layout

You list install pages in `wizard.pages` and uninstall pages in `wizard.uninstall_pages`. While a
task runs, the wizard shows the second page in that list and switches to the last one when the task
ends; a list that holds a single page stays there instead, reporting the outcome in a message box.

Layouts are mostly absolute positions, with flow containers where you need them; only the
attributes marked Supported below do anything.

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

`Image` and `Icon` draw only when you position them absolutely with left/top/width/height: the
runtime first decodes the PNG to PBGRA with WIC, then draws it with GDI alpha blending.

## DPI and image density

Coordinates in a layout assume 96 DPI, so with `ui.dpi_aware` on, the runtime reads the DPI of the
display the window is on and scales the window, coordinates, fonts, hit areas, and corner radius
with it. A setup asks for per-monitor awareness, which means that when you drag the window to a
display with a different scaling factor, the runtime lays the page out again for that display, so
you never keep a second layout per scaling level.

Give the 1x name and let the runtime pick the density:

```text
assets/logo.png
assets/logo@2x.png
```

When the system DPI reaches `ui.dpi_threshold` (144 by default) the runtime prefers the `@2x`
file, otherwise it uses the 1x file, and if the preferred one is missing it falls back to the
other. A layout that already mentions `@2x` is normalized the same way, so you never keep two copies.

## Button

```xml
<Button action="install"
        position="absolute" left="240" top="268" width="240" height="40"
        normal-image="assets/btn_primary.png"
        text="@install_button" font-size="14" font-weight="bold"
        color="#FFFFFFFF" />
```

Supported attributes include `normal-image` and text; the
`file='assets/x.png' dest='...' fade='...'` form draws an image into a sub-rectangle of the control
with 0-255 opacity. A button picks `hover-image`, `pressed-image`, and `disabled-image` for its
states and falls back to `normal-image` when a state image is missing; a state change is painted
into an offscreen bitmap and committed in one step, so hovering does not flicker.

A button is named by the words it draws, and that is what a screen reader reads; a button whose
label is artwork, or drawn elsewhere on the page, has no words to offer, and that is when
`accessible-name` names it:

```xml
<Button id="close" action="close_confirm" accessible-name="@close"
        normal-image="assets/btn_close.png" />
```

An `@key` is looked up in the current language, and anything else is used as written. When a
control declares both `text` and `accessible-name`, a client hears the latter. Link markup inside
drawn words (`[Terms](agreement)`) is not read out: the client is told the sentence without it. The
example names its minimize, close, and custom-options buttons this way.

A button can wait for another control:

```xml
<Checkbox id="terms" checked="false" ... />
<Button action="install" enabled-when="terms:checked" ... />
```

Supported states are `checked`, `unchecked`, `visible`, and `hidden`, plus `valid` and `invalid`
for a text field; a radio group or a select is named by the value it holds, written as that value
itself (`enabled-when="mode:custom"`). One condition may list several `id:state` pairs separated by
commas, and the button needs every one of them: while a condition is unmet the button uses
`disabled-image` and takes no click or hover, and once the condition holds it returns to normal. A
button without `enabled-when` is enabled by default, because the runtime has no rules tied to
specific control names.

## Flow containers

An absolutely positioned `HBox`, `VBox`, or `Content` takes over its subtree: children no longer
need coordinates, and the container lays them out along the main axis in order. Supported
attributes include fixed and measured sizes, `min-width`/`min-height`, `flex-grow`, `flex-shrink`,
`item-spacing`/`gap`, `padding`, `margin` (including single-side forms such as `margin-top`),
`justify-content`, and `align-items`. `HBox` also accepts `horizontal-align`/`vertical-align`,
while `Content` declares a vertical stack with `layout="vertical"`.

When children need more main-axis space than there is, the container compresses the shrinkable
items first, while `flex-shrink="0"` keeps a button at its designed width; a `Spacer` with
`flex-grow="1"` (or a fixed `height`) takes up the rest. Content inside a button supports Label,
Box, and Image/Icon, which is how you build text together with icons.

### Sizes and spacing

`width` and `height` accept pixels, or a percentage of the parent:

```xml
<VBox position="absolute" left="0" top="0" width="100%" height="100%" padding="1">
  <Spacer height="70" />
  <HBox width="100%" padding="0 32" justify-content="center">
    <Label text="@uninstall_confirm" width="100%" text-align="center" />
  </HBox>
  <Spacer flex-grow="1" />
</VBox>
```

`padding` and `margin` accept 1-4 space-separated values with CSS meaning (top, right, bottom,
left). Every value assumes 96 DPI and scales with the system DPI, while a percentage resolves
against the parent's matching edge. `align-items="center"` centres on the cross axis: height inside
an `HBox`, width inside a `VBox`; one item can override that with `align-self="start"` or `"end"`.

`flex-wrap="true"` (or `wrap`) moves an item onto the next line when the current one is full, so
a narrow window reflows instead of squashing its content. With several lines, each line is as
tall as its tallest item, and `justify-content` and `align-self` still apply. Wrapping is off by
default, so an overflowing row compresses its shrinkable items first.

`flex-basis` is the size an item starts from before the container shares out free space, so
`flex-basis="0" flex-grow="1"` takes an equal share next to other flexible items. A container
nested in another one is measured by what its own children need, so a panel needs no fixed size.
Besides `left`/`top`, you can pin an absolutely positioned element with `right`/`bottom`, or with
the `inset` shorthand (`inset="8"`, or `inset-top`/`inset-right`/`inset-bottom`/`inset-left`),
which measures from the opposite edge.

A checkbox picks `checked-image` or `unchecked-image`, draws localized text, and shows Markdown
link markup as text in the `linkcolor` colour. Without a fixed width it measures to its text, and
a `Spacer` takes up the rest of an `HBox`; it wraps only at the available width. Clicking toggles
the checkbox and clicking a link opens its target, so `linkcolor` is exactly what turns a label
into a link. See [Links](#links).

## Scrolling containers

An absolutely positioned `HBox`, `VBox` or `Content` that declares `scrollable="true"` and an
`id` keeps the size it was given and lays its children out at the sizes they ask for, instead of
squeezing them into the room it has. What does not fit is cut off at the container's edge: a row
the list has scrolled past is neither drawn nor clickable, so a button below the list answers the
click that row would otherwise have taken. Wrapping wins over scrolling, so a container that
declares `flex-wrap` stays a plain wrapping container, and one that asks to scroll without an
`id` stays plain too, because there is no name to keep its position under.

The position is kept under the container's `id`, so a redraw and a return to the page both find
it where it was. A wheel over the container moves it 48 pixels a notch, scaled with the display,
and the scrollbar the runtime draws for it does the same job: an 8 pixel strip along the trailing
edge, with a thumb as long as the share of the list in view, and a click on the track outside the
thumb pages the view by one of its own extents. `scrollbar-background` and
`scrollbar-thumb-background` set the two colours, which default to a translucent white.

```xml
<VBox id="components" scrollable="true" position="absolute" left="32" top="96"
      width="320" height="120" item-spacing="8">
  <Checkbox id="core" text="@component_core" width="100%" height="24" />
  <Checkbox id="shell" text="@component_shell" width="100%" height="24" />
  <Checkbox id="tools" text="@component_tools" width="100%" height="24" />
  <Checkbox id="docs" text="@component_docs" width="100%" height="24" />
  <Checkbox id="samples" text="@component_samples" width="100%" height="24" />
</VBox>
```

Those checkboxes are more than a list of drawn options: a project can cut its content into
components with `components.items`, a checkbox's `id` is a component's name, and what the user
leaves ticked decides which components the run installs. See the
[configuration reference](CONFIG_REFERENCE.md#components).

## Label, Select and RadioButton

- An absolutely positioned Label supports `text`/`value`, font-size, font-weight, color, and
  `textalign=center`.
- A Select states the options it offers, draws a DPI-aware arrow over its `background` fill and
  `border-color` outline, and shows the words of the option in use. Clicking the control opens its
  rows, and clicking a row records that row's `value` and closes the menu, while an option the
  layout marks `visible="false"` is never offered. A Select whose `action` is `switch_language`
  lists the languages the project ships, and picking one reloads that language file and redraws the
  page text.
- A RadioButton belongs to the group named by `group` and stands for the `value` it carries. It
  picks `checked-image`/`unchecked-image` and draws its text the way a checkbox does, and
  `checked="true"` marks the row its group starts on. A click records the clicked row for the
  whole group, so one group holds one value at a time; a group the layout starts with nothing
  checked holds none until a row is clicked.
- What the click recorded is what the rest of the page reads: a select answers to its own `id`, a
  radio button to its group, and `enabled-when="<id>:<value>"` waits for one of those values.
- While a select's menu is open the keyboard takes over: Up and Down move the highlight, wrapping
  at both ends, and Enter records the highlighted row -- the language it stands for, or the value --
  while Escape only closes the menu, neither leaving the page nor recording anything. Opening the
  menu puts the highlight on the value the control holds now: the current locale for a language
  control, and for a project's own select the option the user picked, falling back to the first
  option until one is picked. The highlight uses `popup-highlight-background`, falling back to
  `popup-selected-background`, and the entry in use uses `popup-selected-background`.

Two controls that record a value, and a button that waits for both:

```xml
<RadioButton id="quick" group="mode" value="quick" text="@mode_quick" checked="true" />
<RadioButton id="custom" group="mode" value="custom" text="@mode_custom" />
<Select id="edition" background="#FF1E1E1E" border-color="#FF3A3A3A">
  <Option value="standard" text="@edition_standard" />
  <Option value="portable" text="@edition_portable" />
</Select>
<Button action="install" enabled-when="mode:custom, edition:portable" />
```

## ProgressBar

```xml
<ProgressBar id="slrProgress" position="absolute" left="72" top="326"
             width="576" height="10" progress="0" border-radius="5"
             bar-image="assets/bar_installing.png"
             background="#FF4C5868" />
```

`background` paints a rounded track; `bar-image` is a full gradient strip that the runtime clips
from the left by percentage. `progress` is the value you author, though a running install or
uninstall overrides it with live progress and puts the authored value back afterwards.

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

Everything after `config:` is a dot-separated path into `installer_config.json`, and `disk-free:`
takes a TextInput id: the runtime takes the Windows volume root out of that path and queries free
space with `GetDiskFreeSpaceExW`. `size-mb` turns the configured MiB value into readable text,
while `size` formats bytes as B/KB/MB/GB/TB.

You can edit a TextInput that is not `readonly` in place, with the shortcuts a Windows text field
usually offers:

- Clicking puts a blinking caret where the click landed; typing inserts text, Backspace and Delete
  remove it, and Left/Right/Home/End move the caret.
- Dragging with the left button held selects a run of text, drawn with a light blue highlight, and
  a double click selects the word under the pointer; Ctrl+A selects everything.
- Ctrl+C, Ctrl+X, and Ctrl+V copy, cut, and paste; Ctrl+Z undoes and Ctrl+Y redoes, with a run of
  typing collapsed into a single undo step.
- Ctrl+Backspace and Ctrl+Delete remove a word at a time, and Ctrl+Left and Ctrl+Right step over
  one.
- An IME composition window opens at the caret and the candidate list sits just below it, so East
  Asian text is composed inside the field.
- Typing or pasting while text is selected replaces that selection.

Words are grouped the way Windows groups them: letters, digits, and underscore form a word, runs
of whitespace form their own run, and every other character stands alone. So double-clicking the
backslash in `C:\Program Files` selects just that backslash.

A display-only field declares `readonly="true"`. It then ignores clicks and typing, and
`pick_directory` treats it as a display field. Values the user types override the
`value`/`value-source` default for the rest of the run, so `disk-free:` bindings that read the
same control recompute at once.

`status` replaces the authored text with the locale key the runtime publishes for the current
step, which is how a progress page shows live status. While no task is running, the runtime keeps
the `text` placeholder.

A field can also say what makes a value acceptable, and the rest of the page acts on it:

```xml
<TextInput id="editDir" required="true" required-message="@dir_required"
           pattern="?:*" pattern-message="@dir_absolute" />
<Button action="install" enabled-when="chkAgree:checked, editDir:valid" />
<Label value-source="field-error:editDir" color="#FFFF7A7A" />
```

| Attribute | Effect |
| --- | --- |
| `required="true"` | The field cannot be empty |
| `min-length`, `max-length` | How many characters the value may have, counted as characters rather than bytes |
| `pattern` | A mask the whole value has to fit: `*` for any run of characters including none, `?` for exactly one, and every other character for itself |
| `required-message`, `min-length-message`, `max-length-message`, `pattern-message` | The locale key (`@key`) to show while that rule is the one the value breaks |

A field is valid while it breaks none of the rules it declares, and so is an optional field left
empty or a field that declares no rule at all. `enabled-when` takes `valid` and `invalid` for a
field id beside the four states a checkbox or a panel answers. A label with
`value-source="field-error:<field id>"` draws the words of the rule that field's value breaks
first, and nothing at all while the value is one the project accepts; those words come from the
locale file like any other `@key`, so a language that leaves the message out is reported by the build.

A field with nothing in it is still a field: it takes the caret when you click it, which is how
the value a page asks for gets typed in at all.

Colors accept `#RRGGBB` or `#AARRGGBB`; text drawing currently ignores alpha and uses RGB only.

## Visibility

The runtime draws no element that has `visible="false"`, or whose ancestor has it.

## Actions

| Action | Behaviour |
| --- | --- |
| `minimize` | Minimizes the window |
| `close` | Closes now; while a task runs the click is ignored, because the task owns the window |
| `close_confirm` | Opens the skinned confirmation from `ui.dialog_layout`, then closes on Yes, or stops the running task on Yes |
| `pick_directory` | Opens the Windows folder picker and writes the result into a TextInput, see [Folder picker](#folder-picker) |
| `open_url:<key>` | Opens the URL configured under that `links` key |
| `open_url:<https://...>` | Opens the URL as written |
| `install` | Starts the install task, switches to the page that reports it, and reports progress there |
| `uninstall` | Starts the uninstall task, switches to the page that reports it, and reports progress there |
| `launch_app` | Launches the executable this installation deployed, from its own directory |
| `finish` | Same as `close`, meant for the finish page |
| `cancel` | Stops the running task; with nothing running it closes the wizard |
| `switch_language` | Expands the language list and switches the locale when you pick one |
| `next` | Switches to the next page: the one the project declares, or the one a page hook in `scripts/pages.rhai` names |
| `back` | Switches back to the page the user came from; a page a hook skipped is not one of them |
| `toggle_panel:<id>:show/hide` | Shows or hides a target panel and switches the paired show/hide control |
| `dialog_ok` | Confirms the open dialog: a close question stops the running task or exits the setup, a question a script asks answers yes, a notice just closes |
| `dialog_cancel` | Dismisses the open dialog: a question a script asks answers no, and the page under it takes clicks again |

A project that declares more than one page walks it with `next` and `back`: the first page has
nothing behind it and the last page nothing in front, so each stops there instead of wrapping
around. With a `scripts/pages.rhai` that defines `next_page(from)`, every click on `next` asks it
first: naming a page takes the wizard there and skips whatever the declared order has in between,
and an empty answer walks to the declared next page. `back` never asks the hook; it retraces the
pages the user really visited, so a page the hook skipped does not come back, see the
[script API](SCRIPT_API.md#page-hooks). The task itself reports on the second page and ends on the
last one, and a page can say otherwise with `role`: `progress` marks the page a task reports on and
`finish` the page it ends on, which is how a licence page or an options page gets in front of the
task. The roles are described under
[files, languages, and pages](CONFIG_REFERENCE.md#files-languages-and-pages).

A task can also be stopped while it runs: `cancel` does it on the spot, and answering the
`close_confirm` question with Yes does the same. A click may not leave a half-installed product
behind, so the task gives up at the checkpoint after the step it is on, undoes what it has
written, and the wizard returns to the page the task started from. A script sees the same request
through `is_cancelled()` and can end a long step of its own, see the [script API](SCRIPT_API.md).

## Dialogs

A confirmation is not a system message box but an ordinary layout. The runtime draws the file
named by `ui.dialog_layout` (default `layouts/msgBox.xml`) in the middle of the window, over a
scrim between it and the page. Only the dialog's own controls respond while it is open, so a page
button underneath cannot be clicked by mistake; `Enter` confirms it and `Escape` dismisses it.

The same card serves everything a script asks or says: the close confirmation, the notices the
runtime raises, and what a script says through `show_message`, `show_error`, and `ask_yes_no` are
all drawn on this layout. A question takes its two labels from the `yes` and `no` keys of the
locale file; a notice takes the single label its dismissing button shows.

A dialog takes its text from the question being asked, not from the layout, through
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

`visible-with="dismiss"` draws a control only when the dialog offers two answers, so one layout
serves a close question, a script's `ask_yes_no`, and a notice with a single "OK".

The `height` on `Page` is a minimum. A dialog's text follows the language it is shown in, and the
same sentence can take another line elsewhere, so the card grows to fit and stays centred instead
of pushing its answers out through the bottom edge. Your layout's own `padding` decides how much
room is left below the answers, and the example keeps 24 pixels there.

## Links

A `[label](target)` run inside `text`, `value`, or a locale string becomes clickable when the
element also declares `linkcolor`:

```xml
<Label text="@agree_full" linkcolor="#00C4B2" />
```

The runtime resolves the target in this order: an absolute URL (`https://`, `http://`, `mailto:`),
then the project's `links` table in `installer_config.json`, then the historical aliases
`agreement` and `policy`, which map onto `terms_of_service` and `privacy_policy`. A target that
resolves to nothing stays plain text rather than failing.

```json
"links": {
  "terms_of_service": "https://www.taptap.cn/doc/terms/",
  "privacy_policy": "https://www.taptap.cn/doc/privacy-policy/",
  "help": "https://www.taptap.cn/help"
}
```

A `Button` or any other element can also link through `action="open_url:help"`, which uses the
same table, or `action="open_url:https://..."` with the URL written out in full.

## Keyboard and focus

The controls a page declares enter the Tab order in the order they are written: `Tab` walks to the
next one, `Shift+Tab` back to the previous one, and either end wraps around. The control the
keyboard is on is marked with a one-pixel dotted ring drawn over its own rectangle, in the colour
the control names with `focus-color`, then the colour the page names, then the default accent
`#FF1F6FEB`:

```xml
<Page width="720" height="450" focus-color="#FF00FF00">
  <Button id="next" action="next" focus-color="#FFFF8000" text="@next" ... />
</Page>
```

The ring says where the keyboard is; it does not mean the control was pressed. `Enter` and Space do
what the control under the ring does: a button runs the action it declares, a checkbox or radio
button is toggled, and a text field takes the caret with the ring, so typing goes straight in. A
control pressed with the mouse takes the ring too, so the keyboard carries on from where you just
clicked.

A control that cannot be reached is never marked: a button held back by `enabled="false"` or
`enabled-when`, a `readonly` field, an element with `visible="false"`, a label with no `action`,
and a field the layout gives no `id` to, because there would be no name to remember it by. While a
menu or a dialog is open, `Tab` leaves the page's ring alone, and the keyboard belongs to them.

## High-contrast themes

While the user has high contrast on, the colours a layout declares give way to the ones the scheme
keeps for what they paint. A page this runtime produces is a picture, and Windows never draws one
of its own controls into it, so nothing in it follows the setting on its own. A colour is handed
over by role instead, by what it paints:

| Colour in a layout | System colour |
| --- | --- |
| the page's `background`, a popup's `popup-background` | `COLOR_WINDOW` |
| a `Box`'s `background`, a control's own `background`, the open `Select` popup, a progress bar's track, a scrollbar's thumb | `COLOR_BTNFACE` |
| the focus ring, the menu row the keyboard is on | `COLOR_HIGHLIGHT` |
| words on the page, such as a `Label` | `COLOR_WINDOWTEXT` |
| words on a button or a select | `COLOR_BTNTEXT` |
| words on the highlighted menu row | `COLOR_HIGHLIGHTTEXT` |
| a `border-color` outline | `COLOR_WINDOWFRAME` |
| a scrollbar's track | `COLOR_SCROLLBAR` |

Eight roles are kept apart on purpose. In the high contrast themes that ship with Windows, the
page colour and the control face are often the same, while the words on a button and the words on
the page usually are not, and a scrollbar's track has a colour of its own; painting them all alike
would lose part of the scheme the user picked.

Artwork keeps the colours it was drawn with: `background-image`, `Image`, and the two pictures a
button swaps in on hover or press are the project's own pixels, and a scheme has no name for them.
`linkcolor`, and the links inside text, are drawn in the highlight colour.

An open menu gains a line under high contrast. It is drawn over the page, and a scheme paints a
page and a list in the same colour, so without that line there is nothing to say where the menu starts.

When the user turns high contrast on or off, or picks another scheme (`WM_SETTINGCHANGE`,
`WM_SYSCOLORCHANGE`, `WM_THEMECHANGED`), the window lays the page out again and repaints it. A
message that asks for the colours already on screen lays nothing out.

## Click targets and the pointer

Buttons, selects, checkboxes and radio buttons, and any element that declares an `action` respond
to clicks and turn the pointer into a hand over them. A control with no `action` stays inert even
when it sits over a clickable area. That is how you promote an icon or a label into a clickable
link.

## Folder picker

```xml
<TextInput id="editDir" value-source="config:install.default_path" cursor="text" />
<Image id="editDirIcon" action="pick_directory" target="editDir"
       cursor="hand" src="assets/folder-icon.png"
       position="absolute" left="436" top="12" width="16" height="16" />
```

`action="pick_directory"` opens the shell folder chooser. The runtime writes the chosen path to
the TextInput named by `target`, or, when `target` is absent, to the first writable TextInput on
the page; a `readonly` field is a display field and is used only as a last resort. The runtime
stores the value as user input, so it overrides the `value`/`value-source` default from then on,
and `disk-free:` bindings that read the same control pick up the new path at once.

## Not implemented yet

- Implicit minimum sizes beyond `min-width`/`min-height`, and `min-height` on a flow container.
- Screen readers: the window describes the page's controls over MSAA (role, name, value, state and
  place for each of them), announces focus, state, value and page changes on its own, describes
  what a running task publishes -- the status line and the progress bar -- announcing both as they
  change, announces the value of a field as it is typed (once per keystroke, and not when a key only
  moves the caret inside the text), and serves a client that reaches it through `IDispatch` late
  binding with the same members it answers `IAccessible` with.
- The composition window is repositioned when focus or the caret moves, but not while the user is
  scrolling the page under an active composition.
- An item's cross-axis size is still the container's extent unless the item declares one; there is
  no `stretch`/`baseline` distinction beyond that.
