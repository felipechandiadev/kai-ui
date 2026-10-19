//! Kai UI for desktop: Elm-style element tree, Tailwind-like classes, taffy
//! layout and a wgpu renderer.
//!
//! The demo binary (`src/main.rs`) and future `examples/` use this crate as
//! `kai`. See `docs/dev/12-modelo-appkit.md` (Paso 1).

pub mod a11y;
pub mod background;
pub mod components;
pub mod element;
pub mod fonts;
pub mod geometry;
pub mod grid_layout;
pub mod icons;
pub mod layout;
pub mod platform_adapter;
pub mod render;
pub mod responder;
pub mod runtime;
pub mod signal;
pub mod snapshot;
pub mod style;
pub mod text_field;
pub mod theme;
pub mod tw;
pub mod ui_engine;
pub mod version;
