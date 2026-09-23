//! The colours a machine asks for while the user has high contrast turned on.
//!
//! A page this runtime paints is a picture: Windows never draws one of its own
//! controls into it, so nothing in it follows the user's scheme on its own. The
//! scheme is still how the user chose to read a screen, so the renderer paints
//! with it in place of the colours a layout declares. This module answers the
//! two questions that takes -- whether high contrast is on, and which colour the
//! scheme keeps for a surface, a line, or text.

use windows::Win32::Graphics::Gdi::{
    GetSysColor, COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT,
    COLOR_SCROLLBAR, COLOR_WINDOW, COLOR_WINDOWFRAME, COLOR_WINDOWTEXT,
};
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

/// What a colour a layout declares is used for.
///
/// A layout names its colours freely, so what a colour paints is the only thing
/// that can hand it over to the scheme -- and Windows keeps one colour per role,
/// which a scheme is free to share between two of them. The high contrast themes
/// that ship with Windows paint a page and a control face in the same colour,
/// and draw the text on a control in a colour of its own, so a role list that
/// collapsed those pairs would lose colours the user picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    /// A surface a page fills: the page itself, a dialog, a list.
    Surface,
    /// A surface a control fills: a card, a button face, a scrollbar thumb.
    Control,
    /// Where the user is standing or what the user chose: a band, a ring.
    Highlight,
    /// Text drawn on a `Surface`.
    Text,
    /// Text drawn on a `Control`.
    ControlText,
    /// Text drawn on a `Highlight`.
    HighlightText,
    /// A line between two things: a border or an outline.
    Line,
    /// The track of a scrollbar, which Windows keeps a colour of its own for.
    Scrollbar,
}

/// The colours the scheme names, as the machine reports them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Palette {
    surface: u32,
    control: u32,
    highlight: u32,
    text: u32,
    control_text: u32,
    highlight_text: u32,
    line: u32,
    scrollbar: u32,
}

impl Palette {
    /// The colour the scheme keeps for `role`, written the way a layout writes
    /// one.
    ///
    /// It is always opaque, a scheme naming solid colours: a border a layout
    /// declared as a wash is there to be seen, and a translucent line drawn in
    /// the scheme's colour would fade back into the page it separates.
    pub(super) fn colour(&self, role: Role) -> String {
        argb(match role {
            Role::Surface => self.surface,
            Role::Control => self.control,
            Role::Highlight => self.highlight,
            Role::Text => self.text,
            Role::ControlText => self.control_text,
            Role::HighlightText => self.highlight_text,
            Role::Line => self.line,
            Role::Scrollbar => self.scrollbar,
        })
    }

    /// The colours the machine is painting with now.
    fn from_system() -> Self {
        Self {
            surface: unsafe { GetSysColor(COLOR_WINDOW) },
            control: unsafe { GetSysColor(COLOR_BTNFACE) },
            highlight: unsafe { GetSysColor(COLOR_HIGHLIGHT) },
            text: unsafe { GetSysColor(COLOR_WINDOWTEXT) },
            control_text: unsafe { GetSysColor(COLOR_BTNTEXT) },
            highlight_text: unsafe { GetSysColor(COLOR_HIGHLIGHTTEXT) },
            line: unsafe { GetSysColor(COLOR_WINDOWFRAME) },
            scrollbar: unsafe { GetSysColor(COLOR_SCROLLBAR) },
        }
    }

    /// A palette a case names all eight colours of.
    ///
    /// The high contrast themes that ship with Windows give two roles the same
    /// colour, so a case that read its colours off a machine could not always
    /// say which role a colour was painted with. A case that names its own
    /// colours can.
    // The eight colours are the eight roles a scheme names, and a case reads
    // better naming each one than passing a list in the order it must remember.
    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub(super) fn named(
        surface: u32,
        control: u32,
        highlight: u32,
        text: u32,
        control_text: u32,
        highlight_text: u32,
        line: u32,
        scrollbar: u32,
    ) -> Self {
        Self {
            surface,
            control,
            highlight,
            text,
            control_text,
            highlight_text,
            line,
            scrollbar,
        }
    }
}

/// What to paint with, or `None` while the machine is not asking for a scheme.
///
/// `NANO_INSTALLER_TEST_HIGH_CONTRAST` states the answer instead of the machine:
/// `1` says the user turned high contrast on and `0` says the user turned it
/// off, so a case can read both frames a machine produces. The setting belongs
/// to the user, so a case asks for the colours such a machine would paint with
/// rather than turning them on for real, and a case that leaves the variable
/// unset gets whatever the machine is really set to.
pub(super) fn palette() -> Option<Palette> {
    match std::env::var("NANO_INSTALLER_TEST_HIGH_CONTRAST").as_deref() {
        Ok("") | Ok("0") => return None,
        Ok(_) => {}
        Err(_) if !system_asks_for_contrast() => return None,
        Err(_) => {}
    }
    Some(Palette::from_system())
}

/// Whether the user has high contrast turned on.
///
/// The answer is in the accessibility settings rather than in the colours,
/// because a machine without high contrast still has a window colour and a
/// highlight colour to report, and a program that painted with those would
/// ignore the artwork its own pages were designed with.
fn system_asks_for_contrast() -> bool {
    let mut contrast = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some(&mut contrast as *mut HIGHCONTRASTW as *mut std::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_ok() && contrast.dwFlags.0 & HCF_HIGHCONTRASTON.0 != 0
}

/// A `COLORREF` as a layout writes a colour.
///
/// Windows keeps the components of a `COLORREF` as `0x00BBGGRR` and the layout
/// language names them `#AARRGGBB`, so the bytes are read out in reverse. What
/// comes back is opaque: a system colour has no alpha, and the page paints it
/// over its own art rather than into it.
fn argb(colour: u32) -> String {
    format!(
        "#FF{:02X}{:02X}{:02X}",
        colour & 0xFF,
        (colour >> 8) & 0xFF,
        (colour >> 16) & 0xFF
    )
}

#[cfg(test)]
mod tests {
    use super::{argb, Palette, Role};

    /// Every role is answered with the colour the scheme keeps for it, and no
    /// two roles read the same slot: a machine is free to give two roles one
    /// colour, and the renderer still has to ask each of them for its own.
    #[test]
    fn each_role_reads_the_colour_the_scheme_keeps_for_it() {
        let palette = Palette::named(
            0x0001_0203,
            0x0004_0506,
            0x0007_0809,
            0x000A_0B0C,
            0x000D_0E0F,
            0x0010_1112,
            0x0013_1415,
            0x0016_1718,
        );
        assert_eq!(palette.colour(Role::Surface), "#FF030201");
        assert_eq!(palette.colour(Role::Control), "#FF060504");
        assert_eq!(palette.colour(Role::Highlight), "#FF090807");
        assert_eq!(palette.colour(Role::Text), "#FF0C0B0A");
        assert_eq!(palette.colour(Role::ControlText), "#FF0F0E0D");
        assert_eq!(palette.colour(Role::HighlightText), "#FF121110");
        assert_eq!(palette.colour(Role::Line), "#FF151413");
        assert_eq!(palette.colour(Role::Scrollbar), "#FF181716");
    }

    /// A colour the machine reports is turned into a colour a layout could have
    /// written.
    ///
    /// Windows keeps the components of a system colour as `0x00BBGGRR` and the
    /// layout language names them `#AARRGGBB`, so they come back in reverse, and
    /// the colour is opaque: a scheme names solid colours, and a line that kept
    /// no alpha would fade back into the page it separates.
    #[test]
    fn a_system_colour_reads_as_an_opaque_layout_colour() {
        assert_eq!(argb(0x0011_2233), "#FF332211");
        assert_eq!(argb(0x0000_0000), "#FF000000");
        assert_eq!(argb(0x00FF_FFFF), "#FFFFFFFF");
    }
}
