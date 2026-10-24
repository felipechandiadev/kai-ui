//! `kai-build`: the root of a Kai app from its folders (docs/dev/26, R4).
//!
//! In the app's `build.rs`:
//!
//! ```ignore
//! fn main() {
//!     kai_build::app().generate().unwrap_or_else(|error| panic!("{error}"));
//! }
//! ```
//!
//! and in `src/main.rs`:
//!
//! ```ignore
//! kai::include_app!();
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     app::run(state::State::default(), kai::theme::Theme::kai(Default::default()))
//! }
//! ```
//!
//! It reads `src/state.rs` and `src/sections/` ([`scan`]) and writes
//! `$OUT_DIR/kai_app.rs` ([`generate`]): the modules of every section and
//! page, `app::Route`, `app::AppRoot` and `app::run`.

pub mod generate;
pub mod scan;

use std::fmt;
use std::path::PathBuf;

pub use scan::{AppTree, ScanError};

/// Why the root could not be generated.
#[derive(Debug)]
pub enum Error {
    /// The folders do not follow the conventions: every problem found.
    Scan(Vec<ScanError>),
    /// Reading the environment or writing the output failed.
    Io(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scan(errors) => {
                writeln!(f, "kai-build: la app no sigue la estructura de carpetas:")?;
                for error in errors {
                    writeln!(f, "  {error}")?;
                }
                Ok(())
            }
            Self::Io(error) => write!(f, "kai-build: {error}"),
        }
    }
}

impl std::error::Error for Error {}

/// Builder for the app in `CARGO_MANIFEST_DIR`.
pub struct App {
    root: Option<PathBuf>,
    out_dir: Option<PathBuf>,
}

/// The app being built (from inside its `build.rs`).
pub fn app() -> App {
    App {
        root: None,
        out_dir: None,
    }
}

impl App {
    /// Another app folder (tests); default `CARGO_MANIFEST_DIR`.
    pub fn root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    /// Another output folder (tests); default `OUT_DIR`.
    pub fn out_dir(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = Some(out_dir.into());
        self
    }

    /// Scan, generate `kai_app.rs` and tell Cargo to run again only when
    /// the folders change.
    pub fn generate(self) -> Result<PathBuf, Error> {
        let env = |name: &str| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .ok_or_else(|| Error::Io(format!("falta {name}: corre desde build.rs")))
        };
        let root = match self.root {
            Some(root) => root,
            None => env("CARGO_MANIFEST_DIR")?,
        };
        let out_dir = match self.out_dir {
            Some(out) => out,
            None => env("OUT_DIR")?,
        };
        // Cargo scans these for changes (a folder: everything inside).
        for watched in ["src/sections", "src/state.rs"] {
            println!("cargo:rerun-if-changed={}", root.join(watched).display());
        }
        let tree = scan::scan(&root).map_err(Error::Scan)?;
        let code = generate::generate(&tree);
        let out = out_dir.join("kai_app.rs");
        // Unchanged output keeps its timestamp: no rebuild for nothing.
        if std::fs::read_to_string(&out).ok().as_deref() != Some(code.as_str()) {
            std::fs::write(&out, code).map_err(|error| {
                Error::Io(format!("no se pudo escribir {}: {error}", out.display()))
            })?;
        }
        Ok(out)
    }
}
