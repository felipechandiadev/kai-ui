//! Kai UI for desktop: Elm-style element tree, Tailwind-like classes, taffy
//! layout and a wgpu renderer.
//!
//! Apps implement [`runtime::App`] and call [`runtime::run`]; views are
//! built from [`element`] and [`components`], styled with classes resolved
//! against a [`theme`]. The demo binary (`cargo run`) and `examples/` use
//! this crate as `kai`. See `docs/dev/12-modelo-appkit.md` (Paso 1) and
//! `docs/dev/13-hoja-de-ruta-plataforma.md` (Paso 0).
//!
//! Tests find elements by role, accessible name or `test_id` and drive
//! whole apps (clicks, typing, keys) with [`testing`]. Hidden modules are
//! internal: they stay public only because captures and older tests still
//! reach into them, until those tests move to `kai::testing` (13, Paso 6).

pub mod background;
pub mod commands;
pub mod components;
pub mod element;
pub mod icons;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod media;
pub mod menu;
pub mod runtime;
pub mod snapshot;
pub mod testing;
pub mod theme;
pub mod version;

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

pub(crate) mod effects;
pub(crate) mod fonts;
pub(crate) mod platform_adapter;
pub(crate) mod session;
pub(crate) mod virtual_rows;
