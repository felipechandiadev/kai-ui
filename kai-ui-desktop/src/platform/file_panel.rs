//! The system's open and save panels for [`Effects::pick_file`] and
//! [`Effects::save_file`]: `NSOpenPanel` and `NSSavePanel` as sheets on the
//! window (`beginSheetModalForWindow`), so the winit loop is never nested
//! inside `runModal`. The chosen path (or `None` on cancel) comes back
//! through the completion handler.
//!
//! [`Effects::pick_file`]: crate::runtime::Effects::pick_file
//! [`Effects::save_file`]: crate::runtime::Effects::save_file

use std::path::PathBuf;

use winit::window::Window;

use crate::runtime::{PickFile, SaveFile};

/// Delivers the panel's answer (`None`: cancelled or failed to show).
pub(crate) type Done = Box<dyn FnOnce(Option<PathBuf>) + 'static>;

/// A sheet attached to the window. [`ActiveSheet::dismiss`] (or drop while
/// still open) cancels it without calling [`Done`]; a normal close calls
/// `Done` once from AppKit's completion handler.
pub(crate) struct ActiveSheet {
    #[cfg(target_os = "macos")]
    inner: mac::Inner,
}

impl ActiveSheet {
    /// Cancel the sheet without delivering a path (e.g. a newer panel
    /// replaced this one, or the window is closing).
    pub(crate) fn dismiss(self) {
        #[cfg(target_os = "macos")]
        self.inner.dismiss();
    }
}

/// Show the open panel as a sheet on `window`. Returns `None` when the
/// panel cannot start (off the main thread, or no AppKit window); then
/// `done` is called with `None` right away.
#[cfg(target_os = "macos")]
pub(crate) fn begin_pick(window: &Window, request: &PickFile, done: Done) -> Option<ActiveSheet> {
    mac::begin_pick(window, request, done)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn begin_pick(
    _window: &Window,
    _request: &PickFile,
    _done: Done,
) -> Option<ActiveSheet> {
    eprintln!("El selector de archivos solo existe en macOS");
    None
}

/// Show the save panel as a sheet on `window`. Same failure path as
/// [`begin_pick`].
#[cfg(target_os = "macos")]
pub(crate) fn begin_save(window: &Window, request: &SaveFile, done: Done) -> Option<ActiveSheet> {
    mac::begin_save(window, request, done)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn begin_save(
    _window: &Window,
    _request: &SaveFile,
    _done: Done,
) -> Option<ActiveSheet> {
    eprintln!("El panel para guardar solo existe en macOS");
    None
}

#[cfg(target_os = "macos")]
mod mac {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2_app_kit::{NSModalResponse, NSModalResponseOK, NSOpenPanel, NSSavePanel, NSView, NSWindow};
    use objc2_foundation::{NSArray, NSString};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::Window;

    use super::{Done, ActiveSheet};
    use crate::runtime::{PickFile, SaveFile};

    pub(super) struct Inner {
        panel: Retained<NSSavePanel>,
        /// `true` until this sheet delivers or is dismissed.
        live: Arc<AtomicBool>,
    }

    impl Inner {
        pub(super) fn dismiss(self) {
            if self.live.swap(false, Ordering::SeqCst) {
                // SAFETY: `cancel:` is the AppKit API to close the sheet; nil
                // sender is what Cocoa samples pass.
                unsafe { self.panel.cancel(None) };
            }
        }
    }

    impl Drop for Inner {
        fn drop(&mut self) {
            if self.live.swap(false, Ordering::SeqCst) {
                unsafe { self.panel.cancel(None) };
            }
        }
    }

    pub(super) fn begin_pick(
        window: &Window,
        request: &PickFile,
        done: Done,
    ) -> Option<ActiveSheet> {
        let mtm = MainThreadMarker::new()?;
        let ns_window = ns_window(window)?;
        let panel = NSOpenPanel::openPanel(mtm);
        panel.setCanChooseFiles(true);
        panel.setCanChooseDirectories(false);
        panel.setAllowsMultipleSelection(false);
        panel.setTitle(Some(&NSString::from_str(&request.title)));
        panel.setMessage(Some(&NSString::from_str(&request.title)));
        set_types(panel.as_ref(), request.types);
        Some(begin_sheet(panel.into_super(), &ns_window, done))
    }

    pub(super) fn begin_save(
        window: &Window,
        request: &SaveFile,
        done: Done,
    ) -> Option<ActiveSheet> {
        let mtm = MainThreadMarker::new()?;
        let ns_window = ns_window(window)?;
        let panel = NSSavePanel::savePanel(mtm);
        panel.setCanCreateDirectories(true);
        panel.setTitle(Some(&NSString::from_str(&request.title)));
        panel.setMessage(Some(&NSString::from_str(&request.title)));
        panel.setNameFieldStringValue(&NSString::from_str(&request.name));
        set_types(panel.as_ref(), request.types);
        Some(begin_sheet(panel, &ns_window, done))
    }

    fn set_types(panel: &NSSavePanel, types: &[&str]) {
        if types.is_empty() {
            return;
        }
        let types: Vec<_> = types.iter().map(|extension| NSString::from_str(extension)).collect();
        // `allowedContentTypes` needs `UTType`s; extensions are enough here.
        #[allow(deprecated)]
        panel.setAllowedFileTypes(Some(&NSArray::from_retained_slice(&types)));
    }

    fn begin_sheet(
        panel: Retained<NSSavePanel>,
        ns_window: &NSWindow,
        done: Done,
    ) -> ActiveSheet {
        let live = Arc::new(AtomicBool::new(true));
        let live_handler = Arc::clone(&live);
        let panel_handler = panel.clone();
        // `RcBlock` requires `Fn` (the block may be retained); take `done` once.
        let done = std::cell::RefCell::new(Some(done));
        let handler = RcBlock::new(move |response: NSModalResponse| {
            if !live_handler.swap(false, Ordering::SeqCst) {
                return;
            }
            let path = if response == NSModalResponseOK {
                panel_handler
                    .URL()
                    .and_then(|url| url.path())
                    .map(|path| PathBuf::from(path.to_string()))
            } else {
                None
            };
            if let Some(done) = done.borrow_mut().take() {
                done(path);
            }
        });
        panel.beginSheetModalForWindow_completionHandler(ns_window, &handler);
        ActiveSheet {
            inner: Inner { panel, live },
        }
    }

    fn ns_window(window: &Window) -> Option<Retained<NSWindow>> {
        let handle = window.window_handle().ok()?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return None;
        };
        // SAFETY: winit's AppKit handle is a live `NSView` on the main thread.
        let view = unsafe { Retained::<NSView>::retain(appkit.ns_view.as_ptr().cast()) }?;
        view.window()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::{PickFile, SaveFile};

    /// Without a real window the sheet cannot start; the API still accepts
    /// the request types the session puts in the outbox.
    #[test]
    fn pick_and_save_requests_carry_title_and_types() {
        let pick = PickFile::images("Elegir foto");
        assert_eq!(pick.title, "Elegir foto");
        assert!(!pick.types.is_empty());
        let save = SaveFile::new("Exportar", "out.xlsx").types(&["xlsx"]);
        assert_eq!(save.name, "out.xlsx");
        assert_eq!(save.types, &["xlsx"]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn dismiss_flag_only_delivers_once() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let live = Arc::new(AtomicBool::new(true));
        assert!(live.swap(false, Ordering::SeqCst), "first close wins");
        assert!(
            !live.swap(false, Ordering::SeqCst),
            "dismiss after completion must not deliver again"
        );
    }
}
