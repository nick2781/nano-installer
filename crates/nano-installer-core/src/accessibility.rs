//! What a screen reader asks the wizard, and how the wizard answers it.
//!
//! The runtime draws every control itself, so the window owns no child window a
//! client could walk: the page is pixels. A screen reader does not read pixels,
//! so the wizard answers the one message Windows asks about accessibility
//! (`WM_GETOBJECT`) with an `IAccessible` object that describes the page it is
//! showing -- its controls, what each is called, what it holds, where it is, and
//! which one has the keyboard.
//!
//! Windows marshals that object into the client's own process, so a screen
//! reader running beside the wizard reads the same description a client inside
//! this process would. What the client hears is therefore always the page as it
//! is now: the description is built from the runtime's own state on every call
//! rather than cached.
//!
//! The wizard also tells clients when something moves, through the same
//! `NotifyWinEvent` calls a control with a window class makes: a reader is
//! expected to know that the keyboard landed somewhere new without asking.

use windows::core::{implement, Error, IUnknown, Interface, BSTR, GUID, PCWSTR, VARIANT};
use windows::Win32::Foundation::{
    BOOL, E_FAIL, E_INVALIDARG, E_NOTIMPL, E_POINTER, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows::Win32::System::Com::{
    IDispatch, IDispatch_Impl, ITypeInfo, DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO,
};
use windows::Win32::System::Ole::{IOleWindow, IOleWindow_Impl};
use windows::Win32::UI::Accessibility::{
    AccessibleObjectFromWindow, IAccessible, IAccessible_Impl, LresultFromObject, NotifyWinEvent,
    NAVDIR_FIRSTCHILD, NAVDIR_LASTCHILD, NAVDIR_NEXT, NAVDIR_PREVIOUS, ROLE_SYSTEM_CHECKBUTTON,
    ROLE_SYSTEM_COMBOBOX, ROLE_SYSTEM_DIALOG, ROLE_SYSTEM_LINK, ROLE_SYSTEM_PROGRESSBAR,
    ROLE_SYSTEM_PUSHBUTTON, ROLE_SYSTEM_RADIOBUTTON, ROLE_SYSTEM_STATICTEXT, ROLE_SYSTEM_TEXT,
    ROLE_SYSTEM_WINDOW, SELFLAG_TAKEFOCUS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, CHILDID_SELF, EVENT_OBJECT_FOCUS, EVENT_OBJECT_LIVEREGIONCHANGED,
    EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_REORDER, EVENT_OBJECT_STATECHANGE,
    EVENT_OBJECT_VALUECHANGE, OBJID_CLIENT, OBJID_WINDOW,
};

use crate::{
    AnnouncedLive, ControlKind, LayerRect, LiveRegionKind, RuntimeState, WindowAction, UI,
};

/// Accessibility states, spelled here so the module imports what it actually
/// uses. The values are the ones `oleacc.h` declares, and a state is a bit
/// field: a client tests one bit at a time.
const STATE_SYSTEM_CHECKED: u32 = 0x0000_0010;
const STATE_SYSTEM_FOCUSED: u32 = 0x0000_0004;
const STATE_SYSTEM_FOCUSABLE: u32 = 0x0010_0000;
const STATE_SYSTEM_HASPOPUP: u32 = 0x4000_0000;

/// What a control shows, in the terms a screen reader speaks.
///
/// The layout's own [`ControlKind`] says what a control *is*; this is the role
/// `oleacc.h` names for it, which is what a client reads out.
fn role_of(kind: ControlKind) -> u32 {
    match kind {
        ControlKind::Button => ROLE_SYSTEM_PUSHBUTTON,
        ControlKind::Link => ROLE_SYSTEM_LINK,
        ControlKind::Checkbox => ROLE_SYSTEM_CHECKBUTTON,
        ControlKind::Radio => ROLE_SYSTEM_RADIOBUTTON,
        ControlKind::Select => ROLE_SYSTEM_COMBOBOX,
        ControlKind::TextInput => ROLE_SYSTEM_TEXT,
    }
}

/// What the page shows about a task that is running, in the same terms.
///
/// These are not controls, so the role is all a client needs to say what they
/// are: words to read out, or a bar whose value is a percentage.
fn live_role(kind: LiveRegionKind) -> u32 {
    match kind {
        LiveRegionKind::Progress => ROLE_SYSTEM_PROGRESSBAR,
        // The words a task publishes and the rule a field's value breaks are both
        // lines of text: a client reads them out and says nothing else about them.
        LiveRegionKind::Status | LiveRegionKind::Hint => ROLE_SYSTEM_STATICTEXT,
    }
}

/// One control the wizard is showing, described the way a client asks for it.
struct Control {
    /// The number a client names this control by in an `IAccessible` call.
    ///
    /// Windows numbers a simple child from one, so the number is the control's
    /// place in the page's own order plus one. The numbers therefore describe
    /// the page as it is now, which is what the reorder event is for.
    id: i32,
    /// The id the layout declares. The runtime reads it back to move the
    /// keyboard or run what the control does, so a client's request lands on
    /// the same control the user sees.
    layout_id: Option<String>,
    /// The field this element explains, for a hint: a client is told about the
    /// rule a value breaks as that field's own description.
    describes: Option<String>,
    role: u32,
    name: String,
    value: String,
    /// Whether the keyboard can land on it. The words and the bar a running task
    /// publishes are described but never focused, so a client is not told they
    /// can be reached.
    focusable: bool,
    focused: bool,
    /// Whether a box or a row is filled in. `None` for everything that is not
    /// a choice: a button is never checked either way.
    checked: Option<bool>,
    /// Where the control is, in client coordinates.
    rect: LayerRect,
    action: Option<WindowAction>,
}

/// Every control the wizard is showing, in the order a client walks them.
///
/// A dialog owns the window while one is up, so the page behind it is not
/// described: what a reader reads out is what the user can reach.
fn controls(state: &RuntimeState) -> Vec<Control> {
    if state.interaction.dialog.is_some() {
        dialog_controls(state)
    } else {
        page_controls(state)
    }
}

/// The buttons of the question the runtime is asking, which are the only
/// controls a user can reach while it is up.
fn dialog_controls(state: &RuntimeState) -> Vec<Control> {
    let Some(dialog) = state.interaction.dialog.as_ref() else {
        return Vec::new();
    };
    let Some(ui) = state.ui.dialog.as_ref() else {
        return Vec::new();
    };
    let mut controls = Vec::new();
    for region in &ui.actions {
        // A dialog layout carries the two answers and nothing else, so the
        // action is what names the button: its words come from the runtime's
        // own labels for the dialog rather than from the layout.
        let name = match region.action {
            WindowAction::DialogOk => dialog.accept_label.clone(),
            WindowAction::DialogCancel => dialog.dismiss_label.clone(),
            _ => continue,
        };
        controls.push(Control {
            id: controls.len() as i32 + 1,
            layout_id: None,
            describes: None,
            role: ROLE_SYSTEM_PUSHBUTTON,
            name,
            value: String::new(),
            focusable: true,
            focused: false,
            checked: None,
            rect: LayerRect {
                left: region.left,
                top: region.top,
                width: region.right - region.left,
                height: region.bottom - region.top,
            },
            action: Some(region.action.clone()),
        });
    }
    controls
}

/// The controls of the page, in the order the layout recorded them, and then
/// what the page says about a task that is running.
///
/// That order is the page's own: it is the order Tab walks, so a client that
/// walks the same list hears the controls in the order the user reaches them.
/// The words and the bar a task publishes come after them, because they are what
/// a user listens to while there is nothing left to reach: a progress page is
/// usually one bar and one line with no control on it at all.
fn page_controls(state: &RuntimeState) -> Vec<Control> {
    let mut controls = Vec::new();
    for region in &state.ui.focus_regions {
        let rect = LayerRect {
            left: region.left,
            top: region.top,
            width: region.right - region.left,
            height: region.bottom - region.top,
        };
        let mut control = Control {
            id: controls.len() as i32 + 1,
            layout_id: Some(region.id.clone()),
            describes: None,
            role: role_of(region.kind),
            name: region.name.clone(),
            value: String::new(),
            focusable: true,
            focused: state.interaction.focused_control.as_deref() == Some(region.id.as_str()),
            checked: None,
            rect,
            action: region.action.clone(),
        };
        match region.kind {
            ControlKind::TextInput => {
                control.value = state
                    .interaction
                    .text_input_values
                    .get(&region.id)
                    .cloned()
                    .unwrap_or_default();
                if control.name.is_empty() {
                    // A field is named by the words beside it, which is where a
                    // layout writes them: what it holds is its value, and a
                    // client has to hear the two apart.
                    control.name = words_beside(state, rect).unwrap_or_else(|| region.id.clone());
                }
            }
            ControlKind::Checkbox => {
                control.checked = state.interaction.checkbox_states.get(&region.id).copied();
            }
            ControlKind::Radio => {
                // A radio answers for its group, and which row of the group is
                // filled in is the choice kept under the group's id.
                if let Some(WindowAction::ChooseOption { id, value }) = region.action.as_ref() {
                    control.checked = Some(state.interaction.choices.get(id) == Some(value));
                }
            }
            ControlKind::Select => {
                // A select shows one of its options, and that option is what a
                // client reads as the control's value.
                if let Some(WindowAction::ToggleSelectMenu { id }) = region.action.as_ref() {
                    control.value = words_inside(state, rect).unwrap_or_else(|| {
                        state
                            .interaction
                            .choices
                            .get(id)
                            .cloned()
                            .unwrap_or_default()
                    });
                }
            }
            ControlKind::Button | ControlKind::Link => {}
        }
        controls.push(control);
    }
    for region in &state.ui.live_regions {
        controls.push(Control {
            id: controls.len() as i32 + 1,
            layout_id: None,
            describes: region.field.clone(),
            role: live_role(region.kind),
            // A bar's own words are the percentage it shows, which is the value
            // a client reads off it; the line of status words is the name, the
            // way any other static text on the page is.
            name: match region.kind {
                LiveRegionKind::Status | LiveRegionKind::Hint => region.text.clone(),
                LiveRegionKind::Progress => String::new(),
            },
            value: match region.kind {
                LiveRegionKind::Progress => region.text.clone(),
                LiveRegionKind::Status | LiveRegionKind::Hint => String::new(),
            },
            focusable: false,
            focused: false,
            checked: None,
            rect: LayerRect {
                left: region.left,
                top: region.top,
                width: region.right - region.left,
                height: region.bottom - region.top,
            },
            action: None,
        });
    }
    controls
}

/// The words of a text layer, which is what the page draws there.
fn layer_words(layer: &crate::TextLayer) -> String {
    let words: String = layer
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<Vec<_>>()
        .join("");
    words.trim().to_string()
}

fn layer_rect(layer: &crate::TextLayer) -> LayerRect {
    LayerRect {
        left: layer.left,
        top: layer.top,
        width: layer.width,
        height: layer.height,
    }
}

/// The words written above a field and sharing its column, which is how a
/// layout labels one: there is no attribute for it, so the page is read.
///
/// The nearest such line wins, and a line further above than a few times the
/// field's own height belongs to something else -- the page's title, say.
fn words_beside(state: &RuntimeState, field: LayerRect) -> Option<String> {
    let mut best: Option<(i32, String)> = None;
    for layer in &state.ui.texts {
        let words = layer_words(layer);
        if words.is_empty() {
            continue;
        }
        let rect = layer_rect(layer);
        if rect.bottom() > field.top || rect.right() <= field.left || rect.left >= field.right() {
            continue;
        }
        let gap = field.top - rect.bottom();
        if gap > field.height.max(1) * 3 {
            continue;
        }
        let closer = match best.as_ref() {
            Some((closest, _)) => gap < *closest,
            None => true,
        };
        if closer {
            best = Some((gap, words));
        }
    }
    best.map(|(_, words)| words)
}

/// The words drawn inside a rectangle, which is what a select shows there.
fn words_inside(state: &RuntimeState, area: LayerRect) -> Option<String> {
    state
        .ui
        .texts
        .iter()
        .find(|layer| area.intersect(layer_rect(layer)).is_some())
        .map(layer_words)
        .filter(|words| !words.is_empty())
}

/// The client area of the window, which is the wizard as a whole.
unsafe fn client_area(window: HWND) -> LayerRect {
    let mut rect = Default::default();
    let _ = GetClientRect(window, &mut rect);
    LayerRect {
        left: rect.left,
        top: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    }
}

/// The system's own description of a window, which is what a client walks up to
/// from the wizard: a top-level window sits on the desktop.
unsafe fn standard_window_object(window: HWND) -> windows::core::Result<IDispatch> {
    let mut raw: *mut core::ffi::c_void = std::ptr::null_mut();
    AccessibleObjectFromWindow(
        window,
        OBJID_WINDOW.0 as u32,
        &IDispatch::IID,
        &mut raw as *mut *mut core::ffi::c_void,
    )?;
    Ok(IDispatch::from_raw(raw))
}

/// The object the window hands Windows when a client asks what it is showing.
///
/// A fresh object is made for each request, which is what Microsoft's own
/// reference server does: Windows keeps the reference it is given and the
/// client releases it, so nothing here has to outlive the call.
pub(crate) fn client_object(window: HWND) -> IAccessible {
    WizardAccessible { window }.into()
}

/// What changed about a control, in the terms a client listens for.
#[derive(Clone, Copy)]
pub(crate) enum ControlEvent {
    /// A box was checked or a choice was made.
    State,
    /// A field was filled in.
    Value,
}

/// Tells a client the keyboard has landed on a control.
///
/// A reader that hears this announces the control without being asked, which is
/// what makes Tab work for a user who cannot see the ring move.
pub(crate) unsafe fn announce_focus(window: HWND) {
    let focused = focused_child();
    NotifyWinEvent(EVENT_OBJECT_FOCUS, window, OBJID_CLIENT.0, focused);
}

/// Tells a client that a control changed what it holds.
///
/// The id is the one the runtime keeps the change under, which is a control's
/// own id for a box or a field and the group's for a radio, so every control
/// that answers to it is announced.
pub(crate) unsafe fn announce_control(window: HWND, event: ControlEvent, id: &str) {
    let event = match event {
        ControlEvent::State => EVENT_OBJECT_STATECHANGE,
        ControlEvent::Value => EVENT_OBJECT_VALUECHANGE,
    };
    for control in current_controls() {
        if answers_to(&control, id) {
            NotifyWinEvent(event, window, OBJID_CLIENT.0, control.id);
        }
    }
}

/// Tells a client that the page itself changed: what was on it is gone, and the
/// numbers it handed out describe the page that replaced it.
pub(crate) unsafe fn announce_page(window: HWND) {
    NotifyWinEvent(
        EVENT_OBJECT_REORDER,
        window,
        OBJID_CLIENT.0,
        CHILDID_SELF as i32,
    );
    let focused = focused_child();
    if focused != CHILDID_SELF as i32 {
        NotifyWinEvent(EVENT_OBJECT_FOCUS, window, OBJID_CLIENT.0, focused);
    }
}

/// Tells a client what a task that is running has just published.
///
/// A user who cannot see the page has nothing else to go on: an install has no
/// control left to reach while it runs, and whether anything has changed is
/// exactly the question they cannot answer by looking. The announcement is made
/// once per new status or percentage rather than once per frame, because a page
/// is repainted far more often than a task reports progress.
pub(crate) unsafe fn announce_live(window: HWND) {
    let Some(runtime) = UI.get() else {
        return;
    };
    let Ok(mut state) = runtime.lock() else {
        return;
    };
    let live = live_values(&state);
    let Some(previous) = state.announced_live.replace(live.clone()) else {
        // The first frame is what a reader finds when it opens the window: it is
        // recorded rather than read out, so nothing is heard twice.
        return;
    };
    if previous == live {
        return;
    }
    // The words a task publishes are the part worth reading out, so the status
    // line changes as a live region: a client announces it without being asked,
    // which is what a user listening for it needs. Its name is what it says, so
    // the change is a name change as well.
    if previous.status != live.status {
        if let Some(child) = live_child(&state, LiveRegionKind::Status) {
            NotifyWinEvent(EVENT_OBJECT_NAMECHANGE, window, OBJID_CLIENT.0, child);
            NotifyWinEvent(
                EVENT_OBJECT_LIVEREGIONCHANGED,
                window,
                OBJID_CLIENT.0,
                child,
            );
        }
    }
    // The bar is not read out on its own -- a reader asked to announce every
    // percentage would never stop -- so what changes about it is its value.
    if previous.progress != live.progress {
        if let Some(child) = live_child(&state, LiveRegionKind::Progress) {
            NotifyWinEvent(EVENT_OBJECT_VALUECHANGE, window, OBJID_CLIENT.0, child);
        }
    }
    // A field's value breaking a rule is what makes the button beside it inert,
    // so the words that say which rule are announced the moment they appear: a
    // user typing into a field cannot see the hint arrive. A hint that goes away
    // is recorded without being announced, because there is nothing left to read.
    if previous.hint != live.hint && live.hint.is_some() {
        if let Some(child) = live_child(&state, LiveRegionKind::Hint) {
            NotifyWinEvent(EVENT_OBJECT_NAMECHANGE, window, OBJID_CLIENT.0, child);
            NotifyWinEvent(
                EVENT_OBJECT_LIVEREGIONCHANGED,
                window,
                OBJID_CLIENT.0,
                child,
            );
        }
    }
}

/// What a page says about the task that is running, as the values a client is
/// told about: the percentage a bar shows and the words a status line shows.
fn live_values(state: &RuntimeState) -> AnnouncedLive {
    announced_live_of(&state.ui)
}

/// What a page says about a task that is running, for the frame that is about to
/// be shown to nobody in particular.
pub(crate) fn announced_live_of(ui: &crate::RuntimeUi) -> AnnouncedLive {
    let text = |kind: LiveRegionKind| {
        ui.live_regions
            .iter()
            .find(|region| region.kind == kind)
            .map(|region| region.text.clone())
    };
    AnnouncedLive {
        progress: text(LiveRegionKind::Progress),
        status: text(LiveRegionKind::Status),
        hint: text(LiveRegionKind::Hint),
    }
}

/// The number of the child that shows what a task is doing, while the page is
/// showing one.
fn live_child(state: &RuntimeState, kind: LiveRegionKind) -> Option<i32> {
    controls(state)
        .into_iter()
        .find(|control| control.role == live_role(kind))
        .map(|control| control.id)
}

/// Whether a control is one of the controls an id names.
///
/// A control answers to its own id, and a choice answers to the id the runtime
/// keeps it under -- a radio group, a select, a checkbox -- which is the id a
/// change arrives with.
fn answers_to(control: &Control, id: &str) -> bool {
    if control.layout_id.as_deref() == Some(id) {
        return true;
    }
    match control.action.as_ref() {
        Some(WindowAction::ChooseOption { id: group, .. }) => group == id,
        Some(WindowAction::ToggleSelectMenu { id: select }) => select == id,
        Some(WindowAction::ToggleCheckbox { id: checkbox, .. }) => checkbox == id,
        _ => false,
    }
}

/// The controls of the page as it is now, or nothing while the runtime is in no
/// state to answer -- a window being torn down has nothing to describe.
fn current_controls() -> Vec<Control> {
    UI.get()
        .and_then(|state| state.lock().ok())
        .map(|state| controls(&state))
        .unwrap_or_default()
}

/// The number of the control with the keyboard, or the window itself while the
/// keyboard has not landed on one.
fn focused_child() -> i32 {
    current_controls()
        .into_iter()
        .find(|control| control.focused)
        .map_or(CHILDID_SELF as i32, |control| control.id)
}

/// The object Windows hands a screen reader when it asks about the window.
///
/// One instance answers for one window and reads the runtime's state on every
/// call, so a client always hears the page as it is now.
///
/// It answers `IOleWindow` as well, which is what makes the object usable from
/// another process: handing a reference to a client is done by `LresultFromObject`
/// and the accessibility layer asks the object which window it belongs to on the
/// way across. An object without it is refused, and the window falls back to the
/// system's own description of a blank client area.
#[implement(IAccessible, IOleWindow)]
struct WizardAccessible {
    window: HWND,
}

impl WizardAccessible {
    /// The controls as they are right now, numbered from one.
    fn controls(&self) -> Vec<Control> {
        current_controls()
    }

    /// Whether the runtime is asking a question over the page.
    fn dialog_open(&self) -> bool {
        UI.get()
            .and_then(|state| state.lock().ok())
            .is_some_and(|state| state.interaction.dialog.is_some())
    }

    /// What the window itself is called: the product, or the question while one
    /// is up, which is the first thing a reader should hear.
    fn root_name(&self) -> String {
        UI.get()
            .and_then(|state| state.lock().ok())
            .map(|state| match state.interaction.dialog.as_ref() {
                Some(dialog) => dialog.message.clone(),
                None => state.ui.product_name.clone(),
            })
            .unwrap_or_default()
    }

    /// The control a client named, or `None` when it named the window itself.
    ///
    /// A number no control has is an error rather than an empty answer: a
    /// client that asks about a control the page no longer shows has been
    /// overtaken by a page change, and saying so is how it learns that.
    fn child(&self, var: &VARIANT) -> windows::core::Result<Option<Control>> {
        let id = child_id(var)?;
        if id == CHILDID_SELF as i32 {
            return Ok(None);
        }
        self.controls()
            .into_iter()
            .find(|control| control.id == id)
            .map(Some)
            .ok_or_else(|| Error::from(E_INVALIDARG))
    }
}

/// The control a client named in a `VARIANT`.
///
/// A client names a simple child with its number, and the object itself with an
/// empty variant. Anything else -- an object child, which the wizard has none
/// of -- is refused.
fn child_id(var: &VARIANT) -> windows::core::Result<i32> {
    if var.is_empty() {
        return Ok(CHILDID_SELF as i32);
    }
    i32::try_from(var).map_err(|_| Error::from(E_INVALIDARG))
}

impl IAccessible_Impl for WizardAccessible_Impl {
    fn accParent(&self) -> windows::core::Result<IDispatch> {
        unsafe { standard_window_object(self.window) }
    }

    fn accChildCount(&self) -> windows::core::Result<i32> {
        Ok(self.controls().len() as i32)
    }

    fn get_accChild(&self, _varchild: &VARIANT) -> windows::core::Result<IDispatch> {
        // Every control the wizard shows is a simple element, so a client that
        // asks for one as an object is told there is none.
        Err(Error::from(E_INVALIDARG))
    }

    fn get_accName(&self, varchild: &VARIANT) -> windows::core::Result<BSTR> {
        Ok(BSTR::from(match self.child(varchild)? {
            Some(control) => control.name,
            None => self.root_name(),
        }))
    }

    fn get_accValue(&self, varchild: &VARIANT) -> windows::core::Result<BSTR> {
        Ok(BSTR::from(match self.child(varchild)? {
            Some(control) => control.value,
            // The window itself holds nothing: what it shows is its controls.
            None => String::new(),
        }))
    }

    fn get_accDescription(&self, varchild: &VARIANT) -> windows::core::Result<BSTR> {
        // A control's words are its name and what it holds is its value. The one
        // thing further a field has to say about itself is the rule its value
        // breaks, which the page draws as a hint beside it: a client told only
        // about the field would leave a user with a button that does nothing and
        // no word about why.
        let Some(control) = self.child(varchild)? else {
            return Ok(BSTR::new());
        };
        let Some(id) = control.layout_id else {
            return Ok(BSTR::new());
        };
        Ok(BSTR::from(
            self.controls()
                .into_iter()
                .find(|hint| hint.describes.as_deref() == Some(id.as_str()))
                .map(|hint| hint.name)
                .unwrap_or_default(),
        ))
    }

    fn get_accRole(&self, varchild: &VARIANT) -> windows::core::Result<VARIANT> {
        let role = match self.child(varchild)? {
            Some(control) => control.role,
            None if self.dialog_open() => ROLE_SYSTEM_DIALOG,
            None => ROLE_SYSTEM_WINDOW,
        };
        Ok(VARIANT::from(role as i32))
    }

    fn get_accState(&self, varchild: &VARIANT) -> windows::core::Result<VARIANT> {
        let state = match self.child(varchild)? {
            Some(control) => {
                // What a running task publishes is described, not offered: a
                // client is not told it can be reached or focused.
                let mut state = if control.focusable {
                    STATE_SYSTEM_FOCUSABLE
                } else {
                    0
                };
                if control.focused {
                    state |= STATE_SYSTEM_FOCUSED;
                }
                if control.checked == Some(true) {
                    state |= STATE_SYSTEM_CHECKED;
                }
                if control.role == ROLE_SYSTEM_COMBOBOX {
                    // A select answers a press with a list of its own.
                    state |= STATE_SYSTEM_HASPOPUP;
                }
                state
            }
            // The window itself takes the keyboard when nothing on the page
            // does, which is what Escape and the caption answer to.
            None => STATE_SYSTEM_FOCUSABLE,
        };
        Ok(VARIANT::from(state as i32))
    }

    fn get_accHelp(&self, _varchild: &VARIANT) -> windows::core::Result<BSTR> {
        // The runtime has no help of its own to offer about a control.
        Ok(BSTR::new())
    }

    fn get_accHelpTopic(
        &self,
        _pszhelpfile: *mut BSTR,
        _varchild: &VARIANT,
    ) -> windows::core::Result<i32> {
        Err(Error::from(E_NOTIMPL))
    }

    fn get_accKeyboardShortcut(&self, _varchild: &VARIANT) -> windows::core::Result<BSTR> {
        // No control declares an accelerator: the page is walked with Tab and
        // acted on with Enter and Space, which a client derives from the role.
        Ok(BSTR::new())
    }

    fn accFocus(&self) -> windows::core::Result<VARIANT> {
        Ok(
            match self.controls().into_iter().find(|control| control.focused) {
                Some(control) => VARIANT::from(control.id),
                // Nothing has been reached yet, which is not the same as the window
                // having the keyboard.
                None => VARIANT::new(),
            },
        )
    }

    fn accSelection(&self) -> windows::core::Result<VARIANT> {
        // The wizard has no list of selectable children: a radio is filled in
        // rather than selected, and a client reads that from its state.
        Err(Error::from(E_NOTIMPL))
    }

    fn get_accDefaultAction(&self, varchild: &VARIANT) -> windows::core::Result<BSTR> {
        let Some(control) = self.child(varchild)? else {
            return Ok(BSTR::new());
        };
        // What a press does, in the words a client reads out. These are the
        // strings MSAA names for the actions themselves, so a reader translates
        // them into the language it speaks rather than the one the page is in.
        let action = match control.role {
            ROLE_SYSTEM_CHECKBUTTON => {
                if control.checked == Some(true) {
                    "Uncheck"
                } else {
                    "Check"
                }
            }
            ROLE_SYSTEM_RADIOBUTTON => "Select",
            ROLE_SYSTEM_COMBOBOX => "Open",
            // A field's default action is to take the caret, which a client
            // does by focusing it, and a link or a button is pressed. What a
            // running task publishes answers nothing at all.
            ROLE_SYSTEM_TEXT | ROLE_SYSTEM_STATICTEXT | ROLE_SYSTEM_PROGRESSBAR => {
                return Ok(BSTR::new())
            }
            _ => "Press",
        };
        Ok(BSTR::from(action))
    }

    fn accSelect(&self, flagsselect: i32, varchild: &VARIANT) -> windows::core::Result<()> {
        let Some(control) = self.child(varchild)? else {
            return Err(Error::from(E_INVALIDARG));
        };
        if flagsselect & SELFLAG_TAKEFOCUS as i32 == 0 {
            // Taking the focus is the one request the wizard can answer: there
            // is nothing to select besides the control the keyboard is on.
            return Err(Error::from(E_INVALIDARG));
        }
        let Some(id) = control.layout_id else {
            return Err(Error::from(E_INVALIDARG));
        };
        if unsafe { crate::set_focused_control(self.window, Some(id)) }.is_err() {
            return Err(Error::from(E_FAIL));
        }
        Ok(())
    }

    fn accLocation(
        &self,
        pxleft: *mut i32,
        pytop: *mut i32,
        pcxwidth: *mut i32,
        pcyheight: *mut i32,
        varchild: &VARIANT,
    ) -> windows::core::Result<()> {
        if pxleft.is_null() || pytop.is_null() || pcxwidth.is_null() || pcyheight.is_null() {
            return Err(Error::from(E_POINTER));
        }
        let rect = match self.child(varchild)? {
            Some(control) => control.rect,
            None => unsafe { client_area(self.window) },
        };
        // The page is measured in client coordinates while a client draws its
        // highlight where the control is on the screen.
        let mut point = POINT {
            x: rect.left,
            y: rect.top,
        };
        unsafe { ClientToScreen(self.window, &mut point) }.ok()?;
        unsafe {
            pxleft.write(point.x);
            pytop.write(point.y);
            pcxwidth.write(rect.width);
            pcyheight.write(rect.height);
        }
        Ok(())
    }

    fn accNavigate(&self, navdir: i32, varstart: &VARIANT) -> windows::core::Result<VARIANT> {
        let controls = self.controls();
        let start = child_id(varstart)?;
        let current = if start == CHILDID_SELF as i32 {
            None
        } else {
            controls.iter().position(|control| control.id == start)
        };
        // Walking the page is walking the order Tab follows, which is what the
        // layout recorded for the same reason.
        let target = match navdir as u32 {
            NAVDIR_FIRSTCHILD => controls.first(),
            NAVDIR_LASTCHILD => controls.last(),
            NAVDIR_NEXT => current.and_then(|index| controls.get(index + 1)),
            NAVDIR_PREVIOUS => current
                .and_then(|index| index.checked_sub(1))
                .and_then(|index| controls.get(index)),
            // Moving away from the page is not something the wizard describes:
            // its parent is the desktop, which owns what lies beyond it.
            _ => return Ok(VARIANT::new()),
        };
        Ok(match target {
            Some(control) => VARIANT::from(control.id),
            None => VARIANT::new(),
        })
    }

    fn accHitTest(&self, xleft: i32, ytop: i32) -> windows::core::Result<VARIANT> {
        // The point arrives in screen coordinates, and the page is measured in
        // client ones.
        let mut point = POINT { x: xleft, y: ytop };
        unsafe { ScreenToClient(self.window, &mut point) }.ok()?;
        let hit = self
            .controls()
            .into_iter()
            .find(|control| control.rect.contains(point.x, point.y));
        Ok(match hit {
            Some(control) => VARIANT::from(control.id),
            // A point on the wizard but on no control is the wizard itself.
            None => VARIANT::from(CHILDID_SELF as i32),
        })
    }

    fn accDoDefaultAction(&self, varchild: &VARIANT) -> windows::core::Result<()> {
        let Some(control) = self.child(varchild)? else {
            return Err(Error::from(E_INVALIDARG));
        };
        // A client asking for the default action is a user pressing the
        // control, so it runs the same action a press runs.
        let Some(action) = control.action else {
            return Err(Error::from(E_INVALIDARG));
        };
        unsafe { crate::handle_window_action(self.window, action) };
        Ok(())
    }

    fn put_accName(&self, _varchild: &VARIANT, _szname: &BSTR) -> windows::core::Result<()> {
        // The wizard's words come from its layout and its locale table, so a
        // client cannot rename a control behind the page's back.
        Err(Error::from(E_NOTIMPL))
    }

    fn put_accValue(&self, _varchild: &VARIANT, _szvalue: &BSTR) -> windows::core::Result<()> {
        // A field is filled in by typing, which is a keystroke the page owns
        // rather than something a client writes over it.
        Err(Error::from(E_NOTIMPL))
    }
}

impl IOleWindow_Impl for WizardAccessible_Impl {
    fn GetWindow(&self) -> windows::core::Result<HWND> {
        // The wizard is one window: what a client is describing and what it
        // draws its highlight over are the same thing.
        Ok(self.window)
    }

    fn ContextSensitiveHelp(&self, _fentermode: BOOL) -> windows::core::Result<()> {
        // The wizard has no help of its own to put a cursor into.
        Err(Error::from(E_NOTIMPL))
    }
}

impl IDispatch_Impl for WizardAccessible_Impl {
    fn GetTypeInfoCount(&self) -> windows::core::Result<u32> {
        // There is no type library to describe this object: a client uses the
        // interface it asked for.
        Ok(0)
    }

    fn GetTypeInfo(&self, _itinfo: u32, _lcid: u32) -> windows::core::Result<ITypeInfo> {
        Err(Error::from(E_NOTIMPL))
    }

    fn GetIDsOfNames(
        &self,
        _riid: *const GUID,
        _rgsznames: *const PCWSTR,
        _cnames: u32,
        _lcid: u32,
        _rgdispid: *mut i32,
    ) -> windows::core::Result<()> {
        // A name would have to come from a type library, which the runtime does
        // not ship.
        Err(Error::from(E_NOTIMPL))
    }

    fn Invoke(
        &self,
        _dispidmember: i32,
        _riid: *const GUID,
        _lcid: u32,
        _wflags: DISPATCH_FLAGS,
        _pdispparams: *const DISPPARAMS,
        _pvarresult: *mut VARIANT,
        _pexcepinfo: *mut EXCEPINFO,
        _puargerr: *mut u32,
    ) -> windows::core::Result<()> {
        // A client that reaches the wizard through `IDispatch` is told to use
        // the `IAccessible` interface it asked for. Every client Windows ships
        // -- and the bridge that puts MSAA providers under UI Automation -- uses
        // that interface rather than late binding.
        Err(Error::from(E_NOTIMPL))
    }
}

/// Answers the window's `WM_GETOBJECT` for the client area: the reference a
/// screen reader unwraps into the description above.
///
/// Returns the reference Windows is waiting for, or nothing when it could not be
/// made -- the caller then answers as it would for any message it does not know.
pub(crate) unsafe fn answer_get_object(
    window: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<LRESULT> {
    if lparam.0 as i32 != OBJID_CLIENT.0 {
        // The frame, the cursor and the title bar are the system's own: it
        // answers for them better than the runtime could.
        return None;
    }
    let accessible = client_object(window);
    let unknown: IUnknown = accessible.cast().ok()?;
    let reference = LresultFromObject(&IAccessible::IID, wparam, &unknown);
    // A reference to an object is a positive value; anything else is one of the
    // COM error codes, which the caller has to answer as an unknown message.
    (reference.0 > 0).then_some(reference)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A client names a simple child with a number, and the object itself with
    /// an empty variant: both are how every control here is asked about.
    #[test]
    fn a_client_names_a_child_with_what_it_is_told_to() {
        assert_eq!(child_id(&VARIANT::from(3)).expect("a number"), 3);
        assert_eq!(
            child_id(&VARIANT::new()).expect("nothing at all"),
            CHILDID_SELF as i32
        );
        // An object child is one the wizard has none of, so asking for one is
        // refused rather than answered with the wrong control.
        assert!(child_id(&VARIANT::from(BSTR::from("first"))).is_err());
    }

    /// The roles are the ones `oleacc.h` names, because a client reads them
    /// back to decide what to say about a control.
    #[test]
    fn a_control_is_reported_as_what_a_screen_reader_calls_it() {
        assert_eq!(role_of(ControlKind::Button), ROLE_SYSTEM_PUSHBUTTON);
        assert_eq!(role_of(ControlKind::Link), ROLE_SYSTEM_LINK);
        assert_eq!(role_of(ControlKind::Checkbox), ROLE_SYSTEM_CHECKBUTTON);
        assert_eq!(role_of(ControlKind::Radio), ROLE_SYSTEM_RADIOBUTTON);
        assert_eq!(role_of(ControlKind::Select), ROLE_SYSTEM_COMBOBOX);
        assert_eq!(role_of(ControlKind::TextInput), ROLE_SYSTEM_TEXT);
        // What a running task publishes is described with the rest, and a client
        // says different things about a percentage and about a line of words.
        assert_eq!(live_role(LiveRegionKind::Progress), ROLE_SYSTEM_PROGRESSBAR);
        assert_eq!(live_role(LiveRegionKind::Status), ROLE_SYSTEM_STATICTEXT);
        assert_eq!(live_role(LiveRegionKind::Hint), ROLE_SYSTEM_STATICTEXT);
    }
}
