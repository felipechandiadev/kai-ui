//! Testing API: lay out a tree without a window, fonts or GPU and find
//! elements the way users and assistive technologies see them (by role,
//! accessible name, label, text or `test_id`); drive a whole app with
//! [`TestApp`] (clicks, typing, keys, message log, element captures). See
//! `docs/dev/13-hoja-de-ruta-plataforma.md` (Pasos 3 and 4).
//!
//! ```
//! use kai::components::{ButtonVariant, button};
//! use kai::element::{Element, Role};
//! use kai::testing::{By, Screen, layout_of};
//!
//! let mut tree: Element<()> = button("Enviar", ButtonVariant::Primary)
//!     .test_id("form.enviar")
//!     .on_click(());
//! tree.assign_ids();
//! let layout = layout_of(&tree);
//! let screen = Screen::new(&layout);
//! let send = screen.get(By::role(Role::Button).name("Enviar"));
//! assert_eq!(send.test_id(), Some("form.enviar"));
//! ```

mod app;
#[cfg(feature = "mcp")]
mod mcp;
mod query;

pub use app::{AgentReply, Confirm, MemoryClipboard, TestApp, TestAppBuilder};
#[cfg(feature = "mcp")]
pub use mcp::{MCP_CLIENT, McpClient};
pub use query::{By, Found, Screen};

pub use crate::devtools::MsgSource;
pub use crate::ui_engine::{Key, Modifiers};

use crate::element::Element;
use crate::geometry::Size;
use crate::layout::testing::FixedMeasure;
use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
use crate::theme::{Density, Theme};

/// Viewport used by [`layout_of`], in logical points.
pub const VIEWPORT: Size = Size {
    width: 1024.0,
    height: 720.0,
};

/// Layout of `tree` (ids already assigned) in a [`VIEWPORT`] window with the
/// Kai theme and deterministic text measurement.
pub fn layout_of<Msg>(tree: &Element<Msg>) -> Layout {
    layout_with(tree, VIEWPORT, &Theme::kai(Density::Comfortable))
}

/// Like [`layout_of`] with a given viewport and theme.
pub fn layout_with<Msg>(tree: &Element<Msg>, viewport: Size, theme: &Theme) -> Layout {
    Layout::compute(
        tree,
        &LayoutInput {
            viewport,
            theme,
            interaction: &Interaction::default(),
            scroll: &ScrollOffsets::new(),
        },
        &mut StyleCache::default(),
        &mut FixedMeasure,
    )
}
