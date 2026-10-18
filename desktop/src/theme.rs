//! Kai design tokens for the desktop renderer.
//!
//! Values mirror `web/src/theme/tokens.css` with `1rem = 16` logical points.
//! Components never read tokens directly: they use Tailwind-like classes
//! (`tw`) that resolve against [`Theme::color`], [`Theme::spacing_unit`] and
//! the type and control tokens. Tokens the desktop cannot honor yet are listed
//! in [`UNMAPPED_TOKENS`].

use cosmic_text::Weight;

use crate::geometry::Size;
use crate::render::Color;
use crate::style::ComputedText;

pub const REM: f32 = 16.0;

/// `data-density` of the web contract. The responsive `@media` variants are
/// not mapped: a desktop window chooses its density explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Density {
    Comfortable,
    Compact,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTokens {
    pub primary: Color,
    pub background: Color,
    pub surface: Color,
    pub foreground: Color,
    pub border: Color,
    pub active: Color,
    pub accent: Color,
    pub secondary: Color,
    pub muted: Color,
    pub muted_foreground: Color,
    pub success: Color,
    pub info: Color,
    pub warning: Color,
    pub error: Color,
    pub neutral: Color,
    /// `color-mix(in srgb, neutral 65%, background)`.
    pub hover: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    /// `color-mix(in srgb, muted-foreground 72%, transparent)`.
    pub text_disabled: Color,
    pub text_on_primary: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpaceTokens {
    pub space_1: f32,
    pub space_2: f32,
    pub space_3: f32,
    pub space_4: f32,
    pub page_gap: f32,
    pub section_gap: f32,
    pub card_padding: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeTokens {
    pub font_size_body: f32,
    pub font_size_sm: f32,
    pub font_size_xs: f32,
    pub font_size_page_title: f32,
    pub font_size_section_title: f32,
    pub line_height_tight: f32,
    pub line_height_normal: f32,
    pub line_height_relaxed: f32,
    pub weight_regular: Weight,
    pub weight_medium: Weight,
    pub weight_semibold: Weight,
    pub weight_bold: Weight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlTokens {
    pub height_md: f32,
    pub padding_y_md: f32,
    pub height_compact: f32,
    pub padding_y_compact: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub density: Density,
    pub colors: ColorTokens,
    pub space: SpaceTokens,
    pub typography: TypeTokens,
    pub controls: ControlTokens,
}

/// Web tokens with no desktop equivalent yet, and why.
pub const UNMAPPED_TOKENS: &[(&str, &str)] = &[
    (
        "--font-sans, --font-mono, --font-display",
        "se usa la familia genérica SansSerif de cosmic-text (fuente del sistema); no hay carga de fuentes propias",
    ),
    ("--letter-spacing-*", "cosmic-text 0.19 no aplica tracking"),
    ("--color-link, --color-text-link", "no existe widget Link"),
    (
        "--color-text-on-success|warning|error, --color-text-positive|negative|info|warning|error",
        "no hay variantes semánticas de widgets todavía",
    ),
    (
        "transition-*, transform (scale en :active)",
        "los estados cambian de inmediato; no hay animaciones ni transformaciones",
    ),
    (
        "--datagrid-cell-padding-x, --datagrid-cell-padding-y",
        "no existe DataGrid",
    ),
    (
        "@media (max-width: 767.98px) densidad responsive",
        "la ventana elige Density explícitamente; no hay breakpoints",
    ),
];

impl Theme {
    pub fn kai(density: Density) -> Self {
        let colors = color_tokens();
        let (space, typography, controls) = match density {
            Density::Comfortable => (
                SpaceTokens {
                    space_1: 0.25 * REM,
                    space_2: 0.5 * REM,
                    space_3: 0.75 * REM,
                    space_4: REM,
                    page_gap: REM,
                    section_gap: 0.75 * REM,
                    card_padding: REM,
                },
                type_tokens(REM, 0.875 * REM, 0.75 * REM, 1.125 * REM, 1.25 * REM),
                ControlTokens {
                    height_md: 2.25 * REM,
                    padding_y_md: 0.5 * REM,
                    height_compact: 2.0 * REM,
                    padding_y_compact: 0.25 * REM,
                },
            ),
            Density::Compact => (
                SpaceTokens {
                    space_1: 0.1875 * REM,
                    space_2: 0.375 * REM,
                    space_3: 0.5 * REM,
                    space_4: 0.75 * REM,
                    page_gap: 0.75 * REM,
                    section_gap: 0.5 * REM,
                    card_padding: 0.75 * REM,
                },
                type_tokens(0.875 * REM, 0.8125 * REM, 0.6875 * REM, REM, 1.125 * REM),
                ControlTokens {
                    height_md: 2.0 * REM,
                    padding_y_md: 0.375 * REM,
                    height_compact: 1.75 * REM,
                    padding_y_compact: 0.1875 * REM,
                },
            ),
        };
        Self {
            density,
            colors,
            space,
            typography,
            controls,
        }
    }

    /// Smallest logical window supported by the demo. Content may still
    /// overflow it and be reached through vertical scrolling.
    pub fn minimum_viewport(&self) -> Size {
        Size {
            width: 420.0,
            height: 340.0,
        }
    }

    /// Tailwind spacing unit (`p-1`, `gap-1`…): `0.25rem` scaled by density,
    /// so `p-4` equals `--space-4`.
    pub fn spacing_unit(&self) -> f32 {
        self.space.space_4 / 4.0
    }

    /// Text properties of the window root, as the web `body`.
    pub fn base_text(&self) -> ComputedText {
        let t = &self.typography;
        ComputedText {
            color: self.colors.foreground,
            font_size: t.font_size_body,
            line_height: (t.font_size_body * t.line_height_normal).round(),
            weight: t.weight_regular,
            nowrap: false,
        }
    }

    /// Color token by its Tailwind name (`primary`, `muted-foreground`…).
    pub fn color(&self, name: &str) -> Option<Color> {
        let c = &self.colors;
        Some(match name {
            "primary" => c.primary,
            "background" => c.background,
            "surface" => c.surface,
            "foreground" => c.foreground,
            "border" => c.border,
            "active" => c.active,
            "accent" => c.accent,
            "secondary" => c.secondary,
            "muted" => c.muted,
            "muted-foreground" => c.muted_foreground,
            "success" => c.success,
            "info" => c.info,
            "warning" => c.warning,
            "error" => c.error,
            "neutral" => c.neutral,
            "hover" => c.hover,
            "text-primary" => c.text_primary,
            "text-secondary" => c.text_secondary,
            "text-disabled" => c.text_disabled,
            "on-primary" => c.text_on_primary,
            "white" => Color::hex(0xffffff),
            "black" => Color::hex(0x000000),
            "transparent" => Color::rgba(0, 0, 0, 0),
            _ => return None,
        })
    }
}

fn color_tokens() -> ColorTokens {
    let background = Color::hex(0xffffff);
    let neutral = Color::hex(0xf3f4f6);
    let muted_foreground = Color::hex(0x7a8280);
    let foreground = Color::hex(0x131615);
    ColorTokens {
        primary: Color::hex(0x1e73ae),
        background,
        surface: Color::hex(0xfcfffe),
        foreground,
        border: Color::hex(0xa5c2c2),
        active: Color::hex(0x02578b),
        accent: Color::hex(0x02578b),
        secondary: Color::hex(0x00deef),
        muted: Color::hex(0xcfeef0),
        muted_foreground,
        success: Color::hex(0x2e9e5b),
        info: Color::hex(0x1e73ae),
        warning: Color::hex(0xe6a100),
        error: Color::hex(0xd64545),
        neutral,
        hover: neutral.mix(background, 0.65),
        text_primary: foreground,
        text_secondary: muted_foreground,
        text_disabled: muted_foreground.with_alpha(alpha(0.72)),
        text_on_primary: Color::hex(0xffffff),
    }
}

fn type_tokens(body: f32, sm: f32, xs: f32, page_title: f32, section_title: f32) -> TypeTokens {
    TypeTokens {
        font_size_body: body,
        font_size_sm: sm,
        font_size_xs: xs,
        font_size_page_title: page_title,
        font_size_section_title: section_title,
        line_height_tight: 1.25,
        line_height_normal: 1.5,
        line_height_relaxed: 1.625,
        weight_regular: Weight(400),
        weight_medium: Weight(500),
        weight_semibold: Weight(600),
        weight_bold: Weight(700),
    }
}

fn alpha(fraction: f32) -> u8 {
    (fraction * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::{Density, Theme, UNMAPPED_TOKENS};
    use crate::render::Color;

    #[test]
    fn comfortable_theme_mirrors_web_tokens() {
        let theme = Theme::kai(Density::Comfortable);
        assert_eq!(theme.colors.primary, Color::hex(0x1e73ae));
        assert_eq!(theme.colors.surface, Color::hex(0xfcfffe));
        assert_eq!(theme.colors.border, Color::hex(0xa5c2c2));
        assert_eq!(theme.space.space_4, 16.0);
        assert_eq!(theme.space.page_gap, 16.0);
        assert_eq!(theme.controls.height_md, 36.0);
        assert_eq!(theme.typography.font_size_body, 16.0);
        assert_eq!(theme.typography.font_size_section_title, 20.0);
        assert_eq!(theme.base_text().line_height, 24.0);
        assert_eq!(theme.spacing_unit(), 4.0);
    }

    #[test]
    fn compact_density_shrinks_type_space_and_controls() {
        let comfortable = Theme::kai(Density::Comfortable);
        let compact = Theme::kai(Density::Compact);
        assert_eq!(compact.typography.font_size_body, 14.0);
        assert_eq!(compact.space.space_4, 12.0);
        assert_eq!(compact.controls.height_md, 32.0);
        assert!(compact.spacing_unit() < comfortable.spacing_unit());
        assert_eq!(compact.colors, comfortable.colors);
    }

    #[test]
    fn color_mix_tokens_follow_css_color_mix() {
        let theme = Theme::kai(Density::Comfortable);
        assert_eq!(theme.colors.hover, Color::hex(0xf7f8f9));
        assert_eq!(
            theme.colors.text_disabled,
            Color::rgba(0x7a, 0x82, 0x80, 184)
        );
        assert_eq!(
            theme.color("muted-foreground"),
            Some(theme.colors.muted_foreground)
        );
        assert_eq!(theme.color("transparent").map(|c| c.a), Some(0));
        assert_eq!(theme.color("nope"), None);
    }

    #[test]
    fn unmapped_tokens_are_listed_with_a_reason() {
        assert!(
            UNMAPPED_TOKENS
                .iter()
                .any(|(token, _)| token.contains("letter-spacing"))
        );
        assert!(UNMAPPED_TOKENS.iter().all(|(_, reason)| !reason.is_empty()));
    }
}
