//! Kai UI for desktop: Elm-style element tree, Tailwind-like classes, taffy
//! layout and a wgpu renderer.
//!
//! Apps implement [`runtime::App`] and call [`runtime::run`]; views are
//! built from [`element`] and [`components`], styled with classes resolved
//! against a [`theme`]. Colors are [`Color`] / [`color::Color`] (parse hex
//! with [`Color::parse_hex`]) — do not use the hidden `render` module.
//! The demo (`demo/`, an app built from folders by `kai-build`) and
//! `examples/` use this crate as `kai`.
//! See `docs/dev/12-modelo-appkit.md` (Paso 1) and
//! `docs/dev/13-hoja-de-ruta-plataforma.md` (Paso 0 / frontera pública).
//!
//! Tests find elements by role, accessible name or `test_id` and drive
//! whole apps (clicks, typing, keys) with [`testing`]. Modules marked
//! `#[doc(hidden)]` stay `pub` only so the demo's tests, the examples and
//! older paths still compile; prefer the stable surface below. The other
//! internals are already `pub(crate)`.

pub mod background;
pub mod cache;
pub mod charts;
pub mod color;
pub mod commands;
pub mod components;
pub mod document;
pub mod element;
pub mod geo;
pub mod icons;
pub mod maps;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod media;
pub mod menu;
pub mod router;
pub mod runtime;
pub mod snapshot;
pub mod store;
pub mod testing;
pub mod theme;
pub mod version;
pub mod xlsx;

pub use color::Color;

/// The root that `kai-build` generated from `src/sections/` (in the app's
/// `build.rs`): modules `state`, `sections` and `app` (`app::run`,
/// `app::app`, `app::Route`). Call it once, at the top of `main.rs`. See
/// `docs/dev/26-router-y-kai-build.md`.
#[macro_export]
macro_rules! include_app {
    () => {
        include!(concat!(env!("OUT_DIR"), "/kai_app.rs"));
    };
}

/// For the code `kai-build` generates (`dev_state`), so apps need not
/// depend on `serde_json` themselves.
#[doc(hidden)]
pub use serde_json;

// --- Hidden internals ----------------------------------------------------
// Still `pub` because the demo's tests, `examples/gallery.rs` and the
// public signatures use them: `geometry` (`Size`, `Point`, in
// `App::resized`), `layout` (`Interaction`, `ScrollOffsets`), `style`
// (`Cursor`), `gallery` and `render`. TODO(frontera): move what is API
// (geometry, `Cursor`) to a stable path and close the rest. Apps outside
// this repo must not rely on these paths; use [`Color`], not
// `kai::render::Color`.

pub(crate) mod a11y;
pub(crate) mod devtools;
#[doc(hidden)]
pub mod gallery;
#[doc(hidden)]
pub mod geometry;
pub(crate) mod grid_layout;
#[doc(hidden)]
pub mod layout;
#[doc(hidden)]
pub mod render;
pub(crate) mod responder;
pub(crate) mod signal;
#[doc(hidden)]
pub mod style;
pub(crate) mod text_field;
pub(crate) mod tw;
pub(crate) mod ui_engine;

pub(crate) mod dev_state;
pub(crate) mod effects;
pub(crate) mod fold;
pub(crate) mod fonts;
pub(crate) mod platform;
pub(crate) mod platform_adapter;
pub(crate) mod session;
pub(crate) mod virtual_rows;
