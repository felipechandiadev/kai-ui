//! What `kai dev` keeps across restarts (13, Paso 18): the window's frame
//! and focus, and whatever the app returns from [`App::dev_state`].
//!
//! Only in debug builds and with `KAI_DEV_STATE=<file>`: at startup the
//! runtime reads the file and places the window; a line
//! `guardar-y-salir` on stdin makes it write the file and quit.
//!
//! [`App::dev_state`]: crate::runtime::App::dev_state

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Environment variable with the path of the state file.
pub const ENV: &str = "KAI_DEV_STATE";
/// Line on stdin that asks the app to save its state and quit.
pub const SAVE_AND_QUIT: &str = "guardar-y-salir";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DevState {
    pub window: Option<WindowFrame>,
    /// The window had the keyboard focus.
    pub focused: bool,
    pub app: Option<Value>,
}

/// Top-left of the window as `with_position` takes it (its content on
/// macOS, its frame elsewhere) and its inner size, in logical points.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The state file of this run, when `kai dev` started it (debug builds
/// only).
pub fn path() -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var_os(ENV)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

/// The saved state; `None` when the file is missing or unreadable (the
/// first run of a `kai dev` session, or an older format).
pub fn read(path: &Path) -> Option<DevState> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write(path: &Path, state: &DevState) -> std::io::Result<()> {
    let text = serde_json::to_string(state).map_err(std::io::Error::other)?;
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("kai-dev-state-{}-{name}.json", std::process::id()))
    }

    #[test]
    fn the_state_goes_to_a_file_and_back() {
        let path = temp("ida-y-vuelta");
        let state = DevState {
            window: Some(WindowFrame {
                x: 120.0,
                y: 80.5,
                width: 1024.0,
                height: 720.0,
            }),
            focused: false,
            app: Some(json!({ "seccion": "mapa" })),
        };
        write(&path, &state).unwrap();
        assert_eq!(read(&path), Some(state));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_missing_or_broken_file_is_no_state() {
        let path = temp("roto");
        assert_eq!(read(&path), None);
        std::fs::write(&path, "{ esto no es json").unwrap();
        assert_eq!(read(&path), None);
        std::fs::remove_file(path).unwrap();
    }
}
