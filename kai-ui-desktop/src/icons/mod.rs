//! Lucide icons (`assets/icons`, ISC) as SVG sources for the renderer.
//!
//! Only the icons used by Kai web are embedded; `scripts/gen-icons.mjs`
//! regenerates [`Icon`] and the assets from `lucide-react`.
//!
//! Las apps externas no regeneran ese catálogo: copian SVG propios a
//! `assets/icons/` con `kai icons add` y los embeben con
//! [`crate::element::icon_svg`] + `include_str!`.

mod catalog;

pub use catalog::Icon;

/// Lucide's default stroke width, in units of the 24×24 view box.
pub const DEFAULT_STROKE_WIDTH: f32 = 2.0;

/// Fuente rasterizable: catálogo Lucide de Kai o cuerpo SVG de la app.
///
/// El cuerpo de [`IconSource::Svg`] son solo los hijos del viewBox 24×24
/// (mismo formato que `assets/icons/*.svg` del crate), tipicamente
/// `include_str!("../assets/icons/foo.svg")` tras `kai icons add foo`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconSource {
    Catalog(Icon),
    Svg(&'static str),
}

impl From<Icon> for IconSource {
    fn from(icon: Icon) -> Self {
        Self::Catalog(icon)
    }
}

impl PartialEq<Icon> for IconSource {
    fn eq(&self, other: &Icon) -> bool {
        matches!(self, Self::Catalog(icon) if icon == other)
    }
}

impl IconSource {
    /// Hijos SVG del viewBox 24×24.
    pub fn body(self) -> &'static str {
        match self {
            Self::Catalog(icon) => icon.body(),
            Self::Svg(body) => body,
        }
    }
}

/// Standalone SVG document for `icon`, drawn in black on transparent with
/// Lucide's default attributes; the renderer only keeps its coverage.
pub fn svg(icon: Icon, stroke_width: f32) -> String {
    svg_from_body(icon.body(), stroke_width)
}

/// Igual que [`svg`], pero con un cuerpo SVG arbitrario (iconos de app).
pub fn svg_from_body(body: &str, stroke_width: f32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" \
         viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#000\" stroke-width=\"{stroke_width}\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\">{body}</svg>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_from_body_matches_catalog_wrapper() {
        let from_icon = svg(Icon::Search, DEFAULT_STROKE_WIDTH);
        let from_body = svg_from_body(Icon::Search.body(), DEFAULT_STROKE_WIDTH);
        assert_eq!(from_icon, from_body);
        assert_eq!(IconSource::from(Icon::Search), Icon::Search);
        assert_ne!(IconSource::Svg("x"), Icon::Search);
    }
}
