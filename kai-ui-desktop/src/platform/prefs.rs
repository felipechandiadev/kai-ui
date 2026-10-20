//! The Mac's accessibility preferences ([`SystemPrefs`]): System Settings ›
//! Accessibility › Display and Keyboard › Keyboard navigation. The runtime
//! reads them at launch and whenever the window becomes active, since
//! changing them means leaving the app.

use crate::theme::SystemPrefs;

/// The preferences as they are now; the defaults off the main thread.
#[cfg(target_os = "macos")]
pub(crate) fn read() -> SystemPrefs {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSWorkspace};

    let Some(mtm) = MainThreadMarker::new() else {
        return SystemPrefs::default();
    };
    let workspace = NSWorkspace::sharedWorkspace();
    SystemPrefs {
        full_keyboard_access: NSApplication::sharedApplication(mtm).isFullKeyboardAccessEnabled(),
        increase_contrast: workspace.accessibilityDisplayShouldIncreaseContrast(),
        reduce_transparency: workspace.accessibilityDisplayShouldReduceTransparency(),
        reduce_motion: workspace.accessibilityDisplayShouldReduceMotion(),
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn read() -> SystemPrefs {
    SystemPrefs::default()
}
