//! Kai UI for desktop: Elm-style element tree, Tailwind-like classes, taffy
//! layout and a wgpu renderer.
//!
//! Apps implement [`runtime::App`] and call [`runtime::run`]; views are
//! built from [`element`] and [`components`], styled with classes resolved
//! against a [`theme`]. Colors are [`Color`] / [`color::Color`] (parse hex
//! with [`Color::parse_hex`]) — do not use the hidden `render` module.
//! The demo binary (`cargo run`) and `examples/` use this crate as `kai`.
//! See `docs/dev/12-modelo-appkit.md` (Paso 1) and
//! `docs/dev/13-hoja-de-ruta-plataforma.md` (Paso 0 / frontera pública).
//!
//! Tests find elements by role, accessible name or `test_id` and drive
//! whole apps (clicks, typing, keys) with [`testing`]. Modules marked
//! `#[doc(hidden)]` stay `pub` only so in-crate captures, examples and
//! older paths still compile; prefer the stable surface below. Closing
//! them to `pub(crate)` needs moving remaining types into [`testing`]
//! (see TODOs on each hidden `pub mod`).

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

// --- Hidden internals ----------------------------------------------------
// TODO(frontera): pass each of these to `pub(crate)` once captures/tests
// only touch them through `kai::testing` (or a dedicated feature). Apps
// outside this repo must not rely on these paths; `kai::render::Color`
// is the known leak that DSP desktop still uses — migrate to [`Color`].

#[doc(hidden)]
pub mod a11y;
#[doc(hidden)]
pub mod demo;
#[doc(hidden)]
pub mod devtools;
#[doc(hidden)]
pub mod gallery;
#[doc(hidden)]
pub mod geometry;
#[doc(hidden)]
pub mod grid_layout;
#[doc(hidden)]
pub mod layout;
#[doc(hidden)]
pub mod render;
#[doc(hidden)]
pub mod responder;
#[doc(hidden)]
pub mod signal;
#[doc(hidden)]
pub mod style;
#[doc(hidden)]
pub mod text_field;
#[doc(hidden)]
pub mod tw;
#[doc(hidden)]
pub mod ui_engine;

pub(crate) mod dev_state;
pub(crate) mod effects;
pub(crate) mod fold;
pub(crate) mod fonts;
pub(crate) mod platform;
pub(crate) mod platform_adapter;
pub(crate) mod session;
pub(crate) mod virtual_rows;
