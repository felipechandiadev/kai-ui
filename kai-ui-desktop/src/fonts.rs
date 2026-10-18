//! Platform UI font family for cosmic-text / fontdb.

use cosmic_text::Family;

/// Family used for labels, buttons, and text fields (system UI on macOS/Windows).
pub fn ui_font_family() -> Family<'static> {
    #[cfg(target_os = "macos")]
    {
        Family::Name(".AppleSystemUIFont")
    }
    #[cfg(target_os = "windows")]
    {
        Family::Name("Segoe UI")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Family::SansSerif
    }
}

pub fn ui_font_attrs() -> cosmic_text::Attrs<'static> {
    cosmic_text::Attrs::new().family(ui_font_family())
}
