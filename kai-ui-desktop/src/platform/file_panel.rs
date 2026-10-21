//! The system's open and save panels for [`Effects::pick_file`] and
//! [`Effects::save_file`]: `NSOpenPanel` and `NSSavePanel` run as
//! application-modal windows (`runModal`). The event loop is inside the
//! panel's own run loop until it closes; winit queues the window's events
//! meanwhile.
//!
//! [`Effects::pick_file`]: crate::runtime::Effects::pick_file
//! [`Effects::save_file`]: crate::runtime::Effects::save_file

use std::path::PathBuf;

use crate::runtime::{PickFile, SaveFile};

/// Show the open panel for `request` and wait for the person: the chosen
/// file, or `None` when they cancel (or off the main thread).
#[cfg(target_os = "macos")]
pub(crate) fn pick(request: &PickFile) -> Option<PathBuf> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
    use objc2_foundation::{NSArray, NSString};

    let mtm = MainThreadMarker::new()?;
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(false);
    panel.setAllowsMultipleSelection(false);
    panel.setTitle(Some(&NSString::from_str(&request.title)));
    panel.setMessage(Some(&NSString::from_str(&request.title)));
    if !request.types.is_empty() {
        let types: Vec<_> = request
            .types
            .iter()
            .map(|extension| NSString::from_str(extension))
            .collect();
        // `allowedContentTypes` needs `UTType`s (another framework); the
        // extensions are enough for an open panel.
        #[allow(deprecated)]
        panel.setAllowedFileTypes(Some(&NSArray::from_retained_slice(&types)));
    }
    if panel.runModal() != NSModalResponseOK {
        return None;
    }
    let path = panel.URL()?.path()?;
    Some(PathBuf::from(path.to_string()))
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn pick(_request: &PickFile) -> Option<PathBuf> {
    eprintln!("El selector de archivos solo existe en macOS");
    None
}

/// Show the save panel for `request` and wait for the person: where to
/// save, or `None` when they cancel (or off the main thread). The panel
/// asks itself before replacing a file.
#[cfg(target_os = "macos")]
pub(crate) fn save(request: &SaveFile) -> Option<PathBuf> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSModalResponseOK, NSSavePanel};
    use objc2_foundation::{NSArray, NSString};

    let mtm = MainThreadMarker::new()?;
    let panel = NSSavePanel::savePanel(mtm);
    panel.setCanCreateDirectories(true);
    panel.setTitle(Some(&NSString::from_str(&request.title)));
    panel.setMessage(Some(&NSString::from_str(&request.title)));
    panel.setNameFieldStringValue(&NSString::from_str(&request.name));
    if !request.types.is_empty() {
        let types: Vec<_> = request
            .types
            .iter()
            .map(|extension| NSString::from_str(extension))
            .collect();
        #[allow(deprecated)]
        panel.setAllowedFileTypes(Some(&NSArray::from_retained_slice(&types)));
    }
    if panel.runModal() != NSModalResponseOK {
        return None;
    }
    let path = panel.URL()?.path()?;
    Some(PathBuf::from(path.to_string()))
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn save(_request: &SaveFile) -> Option<PathBuf> {
    eprintln!("El panel para guardar solo existe en macOS");
    None
}
