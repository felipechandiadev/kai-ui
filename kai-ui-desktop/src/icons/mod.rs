//! Lucide icons (`assets/icons`, ISC) as SVG sources for the renderer.
//!
//! Only the icons used by Kai web are embedded; `scripts/gen-icons.mjs`
//! regenerates [`Icon`] and the assets from `lucide-react`.

mod catalog;

pub use catalog::Icon;

/// Lucide's default stroke width, in units of the 24×24 view box.
pub const DEFAULT_STROKE_WIDTH: f32 = 2.0;

/// Standalone SVG document for `icon`, drawn in black on transparent with
/// Lucide's default attributes; the renderer only keeps its coverage.
pub fn svg(icon: Icon, stroke_width: f32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" \
         viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#000\" stroke-width=\"{stroke_width}\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\">{}</svg>",
        icon.body()
    )
}
