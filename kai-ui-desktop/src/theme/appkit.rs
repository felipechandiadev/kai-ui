//! Static AppKit theme: the tables of `docs/dev/11-tema-appkit.md` in code
//! (§3.1 text styles, §4.2 semantic colors, §4.3 system tones, §6.2 control
//! metrics, §6.4 focus ring), measured on macOS 15.6.1 with the default
//! (blue) accent, and the `Glass27` differences of 11 §6.3.8, measured on
//! macOS 27.0.1 with a binary linked with SDK 27. No platform dependencies:
//! usable in tests, snapshots and outside macOS. Paso 16 reads the live
//! values from AppKit instead.
//!
//! Alpha values are rounded to `u8` (`.847 → 216`, `.498 → 127`, `.549 →
//! 140`, `.259 → 66`, `.247 → 63`, `.098 → 25`, `.90 → 230`, `.35 → 89`,
//! `.50 → 128`, `.427 → 109`). `.498 × 255 = 127`, not 128: do not "fix" it.

use cosmic_text::Weight;

use super::{
    AppKitGeneration, Appearance, ColorTokens, ControlTokens, Density, SemanticColors, SpaceTokens,
    SystemInfo, SystemPrefs, TextStyleToken, TextStyles, Theme, ThemeKind, TypeTokens, density_for,
};
use crate::element::ControlSize;
use crate::render::Color;
use crate::theme::{BySize, MixStyle};

const fn k(a: u8) -> Color {
    Color::rgba(0, 0, 0, a)
}

const fn w(a: u8) -> Color {
    Color::rgba(255, 255, 255, a)
}

/// `aqua` (11 §4.2).
pub(super) const LIGHT: SemanticColors = SemanticColors {
    label: k(216),
    secondary_label: k(127),
    tertiary_label: k(66),
    quaternary_label: k(25),
    placeholder: k(63),
    control_text: k(216),
    disabled_control_text: k(63),
    on_accent: Color::hex(0xffffff),
    link: Color::hex(0x0068da),
    window_background: Color::hex(0xececec),
    under_page: Color::rgba(0x96, 0x96, 0x96, 230),
    control_background: Color::hex(0xffffff),
    text_background: Color::hex(0xffffff),
    control: Color::hex(0xffffff),
    control_pressed: Color::hex(0xd7d7d7),
    control_accent: Color::hex(0x007aff),
    accent_pressed: Color::hex(0x005cd7),
    accent_disabled: Color::rgba(0x00, 0x7a, 0xff, 89),
    selected_content: Color::hex(0x0064e1),
    selected_content_inactive: Color::hex(0xdcdcdc),
    text_selection: Color::hex(0xb3d7ff),
    text_selection_inactive: Color::hex(0xdcdcdc),
    focus_ring: Color::rgba(0x00, 0x67, 0xf4, 127),
    caret: Color::hex(0x007aff),
    separator: k(25),
    grid: Color::hex(0xe6e6e6),
    fill: [k(25), k(20), k(12), k(7), k(2)],
    switch_on_inactive: Color::hex(0xb8b8b8),
    // Mixed theme [C]: a quarter of the way from `controlColor` to its
    // `.pressed` (#d7d7d7), about Kai's hover step (#ffffff → #f7f8f9).
    bezel_hover: Color::hex(0xf5f5f5),
    // 15 % of the way to `.pressed` (#005cd7).
    accent_hover: Color::hex(0x0076f9),
    // α .16, one step above `systemFill` (.098).
    fill_pressed: k(41),
    // Kai `secondary` (#00deef) with Kai `foreground` text.
    mix_secondary: Color::hex(0x00deef),
    mix_on_secondary: Color::hex(0x131615),
};

/// `darkAqua` (11 §4.2).
pub(super) const DARK: SemanticColors = SemanticColors {
    label: w(216),
    secondary_label: w(140),
    tertiary_label: w(63),
    quaternary_label: w(25),
    placeholder: w(63),
    control_text: w(216),
    disabled_control_text: w(63),
    on_accent: Color::hex(0xffffff),
    link: Color::hex(0x419cff),
    window_background: Color::hex(0x323232),
    under_page: Color::hex(0x282828),
    control_background: Color::hex(0x1e1e1e),
    text_background: Color::hex(0x1e1e1e),
    control: w(63),
    control_pressed: Color::rgba(0x6d, 0x6d, 0x6d, 109),
    control_accent: Color::hex(0x007aff),
    accent_pressed: Color::hex(0x2ea8ff),
    accent_disabled: Color::rgba(0x00, 0x7a, 0xff, 128),
    selected_content: Color::hex(0x0059d1),
    selected_content_inactive: Color::hex(0x464646),
    text_selection: Color::hex(0x3f638b),
    text_selection_inactive: Color::hex(0x464646),
    focus_ring: Color::rgba(0x1a, 0xa9, 0xff, 127),
    caret: Color::hex(0x007aff),
    separator: w(25),
    grid: Color::hex(0x1a1a1a),
    fill: [w(25), w(20), w(12), w(7), w(2)],
    // Not measured in dark (?): provisional, measure in Paso 19.
    switch_on_inactive: w(63),
    // Mixed theme [C]: `controlColor` is white α .247; hover lifts it to
    // .32 (pressed is #6d6d6d α .427).
    bezel_hover: w(82),
    // 15 % of the way to the dark `.pressed` (#2ea8ff): slightly lighter.
    accent_hover: Color::hex(0x0781ff),
    fill_pressed: w(41),
    // Kai has no dark theme: a slightly deeper cyan, still with dark text.
    mix_secondary: Color::hex(0x00c4d4),
    mix_on_secondary: Color::hex(0x131615),
};

/// `aqua` of macOS 27 with SDK ≥ 26 (11 §6.3.8): the rest is as in 15.
/// The placeholder equals `secondaryLabelColor`.
pub(super) const GLASS27_LIGHT: SemanticColors = SemanticColors {
    window_background: Color::hex(0xffffff),
    under_page: Color::hex(0xf6f6f6),
    placeholder: k(127),
    // The bezel is `secondarySystemFill` (α .078): hover is one fill level
    // up, `systemFill` (.098) [C].
    bezel_hover: k(25),
    ..LIGHT
};

/// `darkAqua` of macOS 27 with SDK ≥ 26 (11 §6.3.8). `underPage` keeps
/// `#282828`. The window equals `controlBackgroundColor` (`#1e1e1e`).
pub(super) const GLASS27_DARK: SemanticColors = SemanticColors {
    window_background: Color::hex(0x1e1e1e),
    placeholder: w(140),
    bezel_hover: w(25),
    ..DARK
};

/// System tones that fill the Kai status colors (11 §4.3, macOS 11–15).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SystemTones {
    pub red: Color,
    pub orange: Color,
    pub green: Color,
    pub gray: Color,
}

pub(super) const LIGHT_TONES: SystemTones = SystemTones {
    red: Color::hex(0xff3b30),
    orange: Color::hex(0xff9500),
    green: Color::hex(0x28cd41),
    gray: Color::hex(0x8e8e93),
};

pub(super) const DARK_TONES: SystemTones = SystemTones {
    red: Color::hex(0xff453a),
    orange: Color::hex(0xff9f0a),
    green: Color::hex(0x32d74b),
    gray: Color::hex(0x98989d),
};

/// HIG 26 tones, measured unchanged on 27.0.1 (11 §6.3.8). They apply to
/// every app on 26+ whatever its SDK, so [`Theme::appkit_for`] picks them by
/// OS; [`Theme::appkit_static`] by generation.
pub(super) const HIG26_LIGHT_TONES: SystemTones = SystemTones {
    red: Color::hex(0xff383c),
    orange: Color::hex(0xff8d28),
    green: Color::hex(0x34c759),
    gray: Color::hex(0x8e8e93),
};

pub(super) const HIG26_DARK_TONES: SystemTones = SystemTones {
    red: Color::hex(0xff4245),
    orange: Color::hex(0xff9230),
    green: Color::hex(0x30d158),
    // Measured; the HIG table says 142, 142, 147.
    gray: Color::hex(0x98989d),
};

fn tones(hig26: bool, appearance: Appearance) -> SystemTones {
    match (hig26, appearance) {
        (false, Appearance::Light) => LIGHT_TONES,
        (false, Appearance::Dark) => DARK_TONES,
        (true, Appearance::Light) => HIG26_LIGHT_TONES,
        (true, Appearance::Dark) => HIG26_DARK_TONES,
    }
}

/// Control metrics of macOS 11–15 (11 §6.2, §6.4). `ExtraLarge` does not
/// exist before macOS 26: it repeats `Large` [C].
pub(super) const AQUA11_CONTROLS: ControlTokens = ControlTokens {
    // [C] padding.
    height_md: 20.0,
    padding_y_md: 2.0,
    height_compact: 16.0,
    padding_y_compact: 1.0,
    // Alignment rect of a push button [B-med].
    button_height: [13.0, 16.0, 20.0, 28.0, 28.0],
    // `.roundedBezel` [B-med].
    field_height: [17.0, 19.0, 22.0, 30.0, 30.0],
    // `.squareBezel`: does not grow in large [B-med].
    field_square_height: [15.0, 19.0, 21.0, 21.0, 21.0],
    // Inferred from the alpha channel, ±0.5 pt [C].
    radius: [3.0, 4.0, 5.0, 6.0, 6.0],
    // The rounded field shares the push radius; the square one is square.
    field_radius: [3.0, 4.0, 5.0, 6.0, 6.0],
    field_square_radius: [0.0; 5],
    // (alignment width − title width) / 2 of a push button, SDK 15 on
    // macOS 27 (11 §6.3.8) [B-med 27.0.1].
    padding_x: [7.0, 8.0, 10.0, 10.0, 10.0],
    // `NSFont.systemFontSize(for:)` [B-med].
    font_size: [9.0, 11.0, 13.0, 13.0, 13.0],
    // Natural line heights of 11 and 13 pt; 9 pt → 11 (?).
    line_height: [11.0, 13.0, 16.0, 16.0, 16.0],
    gap: 8.0,
    // 3 pt outward, no offset [B-med].
    focus_ring_width: 3.0,
    focus_ring_offset: 0.0,
};

/// Control metrics of macOS 27 with SDK ≥ 26 (11 §6.3.8) [B-med 27.0.1].
/// `prefersCompactControlSizeMetrics` (the Aqua11 heights) is not modeled.
pub(super) const GLASS27_CONTROLS: ControlTokens = ControlTokens {
    // Alignment = frame: no insets in 27.
    button_height: [16.0, 20.0, 24.0, 28.0, 36.0],
    // `.squareBezel` and `.roundedBezel` are the same in 27.
    field_height: [19.0, 22.0, 24.0, 24.0, 24.0],
    field_square_height: [19.0, 22.0, 24.0, 24.0, 24.0],
    // Large and extra large are capsules (height / 2).
    radius: [4.0, 5.0, 6.0, 14.0, 18.0],
    field_radius: [4.0, 5.0, 6.0, 7.0, 9.0],
    field_square_radius: [4.0, 5.0, 6.0, 7.0, 9.0],
    padding_x: [8.0, 10.0, 12.0, 14.0, 18.0],
    // `systemFontSize(for:)` unchanged, extra large included.
    ..AQUA11_CONTROLS
};

/// Kai field heights by control size (`h-control-compact` 32, `h-control`
/// 36, and the large extrapolation of `kai_controls`).
const KAI_FIELD_HEIGHT: BySize = [32.0, 32.0, 36.0, 44.0, 52.0];

/// Text-button heights mixed themes move towards: Kai's field heights, but
/// mini stays below small (28, an `IconButtonSize::Sm`), so regular = field
/// (36), small = compact field (32) and mini is smaller.
const MIX_BUTTON_HEIGHT: BySize = [28.0, 32.0, 36.0, 44.0, 52.0];

/// Window background of the mixed themes (A, B, C) in light: white, like
/// Glass27 and Kai pages, instead of Aqua11's `#ececec`. Dark keeps the
/// measured window color.
const MIX_LIGHT_WINDOW: Color = Color::hex(0xffffff);

/// Fraction of the way from the AppKit field height to Kai's.
fn mix_fraction(mix: MixStyle) -> f32 {
    match mix {
        MixStyle::AppKit => 0.0,
        MixStyle::A => 0.5,
        MixStyle::B => 0.75,
        MixStyle::C => 1.0,
    }
}

/// The eleven text styles of 11 §3.1. `tracking_em` is the HIG tracking
/// table (in points) divided by the size: 13 pt → −0.08 / 13 em.
pub(super) fn aqua11_text_styles() -> TextStyles {
    let style = |size: f32, line_height: f32, weight: u16, tracking_pt: f32| TextStyleToken {
        size,
        line_height,
        weight: Weight(weight),
        tracking_em: tracking_pt / size,
    };
    TextStyles {
        large_title: style(26.0, 32.0, 400, 0.22),
        title1: style(22.0, 26.0, 400, -0.26),
        title2: style(17.0, 22.0, 400, -0.43),
        title3: style(15.0, 20.0, 400, -0.23),
        headline: style(13.0, 16.0, 700, -0.08),
        body: style(13.0, 16.0, 400, -0.08),
        callout: style(12.0, 15.0, 400, 0.0),
        subheadline: style(11.0, 14.0, 400, 0.06),
        footnote: style(10.0, 13.0, 400, 0.12),
        caption1: style(10.0, 13.0, 400, 0.12),
        caption2: style(10.0, 13.0, 500, 0.12),
    }
}

/// Kai web tokens filled from the AppKit colors (11 §11.2), so a screen
/// written only with Kai classes still looks reasonable with this theme.
pub(super) fn color_tokens(s: &SemanticColors, tones: &SystemTones) -> ColorTokens {
    ColorTokens {
        primary: s.control_accent,
        background: s.window_background,
        surface: s.control_background,
        foreground: s.label,
        border: s.separator,
        active: s.accent_pressed,
        accent: s.control_accent,
        secondary: s.control,
        muted: s.fill[2],
        muted_foreground: s.secondary_label,
        success: tones.green,
        info: s.control_accent,
        warning: tones.orange,
        error: tones.red,
        neutral: tones.gray,
        // AppKit content controls do not react to the pointer (11 §4.2);
        // the mixed theme gives Kai `hover:bg-hover` rows a faint fill.
        hover: s.fill[2],
        text_primary: s.label,
        text_secondary: s.secondary_label,
        text_disabled: s.disabled_control_text,
        text_on_primary: s.on_accent,
    }
}

impl Theme {
    /// AppKit theme from the static tables of 11 (§3.1, §4.2, §4.3, §6.2,
    /// §6.4, §6.3.8), without reading anything from the system. `Glass26`
    /// is unmeasured and uses the `Glass27` tables. The system tones follow
    /// the generation here (HIG 26 for `Glass*`); [`Theme::appkit_for`]
    /// follows the OS instead. The root control size is
    /// [`Theme::control_size`] (`Regular`); change it with
    /// `Theme { control_size, ..theme }`.
    pub fn appkit_static(generation: AppKitGeneration, appearance: Appearance) -> Theme {
        let glass = generation != AppKitGeneration::Aqua11;
        let semantic = match (glass, appearance) {
            (false, Appearance::Light) => LIGHT,
            (false, Appearance::Dark) => DARK,
            (true, Appearance::Light) => GLASS27_LIGHT,
            (true, Appearance::Dark) => GLASS27_DARK,
        };
        let tones = tones(glass, appearance);
        let kai = Theme::kai(Density::Comfortable);
        let control_size = ControlSize::Regular;
        Theme {
            kind: ThemeKind::AppKit(generation),
            appearance,
            density: density_for(control_size),
            control_size,
            colors: color_tokens(&semantic, &tones),
            semantic,
            // Mixed theme: hover on, like Kai ([`Theme::with_hover`]).
            hover_effects: true,
            // Strict tables; `appkit_for` applies the default candidate.
            mix: MixStyle::AppKit,
            // `near` everywhere, strict mix included (`KAI_LABEL=appkit`
            // brings back the original label).
            field_labels: crate::theme::FieldLabels::Near,
            // AppKit changes text sizes, not the spacing scale (11 §11.2);
            // HIG window margins (11 §8.1).
            space: SpaceTokens {
                card_padding: 12.0,
                page_gap: 20.0,
                ..kai.space
            },
            typography: TypeTokens {
                font_size_body: 13.0,
                font_size_sm: 11.0,
                font_size_xs: 10.0,
                font_size_page_title: 15.0,
                font_size_section_title: 17.0,
                styles: aqua11_text_styles(),
                ..kai.typography
            },
            controls: if glass {
                GLASS27_CONTROLS
            } else {
                AQUA11_CONTROLS
            },
            // Full keyboard access is off by default on macOS (11 §6.4);
            // Paso 16 reads the real setting.
            prefs: SystemPrefs::default(),
        }
    }

    /// Same theme with mixed-theme candidate `mix`. In an AppKit theme it
    /// also sets the field and text-button heights: the measured AppKit
    /// height moved towards Kai's (fields `[32, 32, 36, 44, 52]`, buttons
    /// `[28, 32, 36, 44, 52]`) by 0 (strict), 1/2 (A), 3/4 (B) or all the
    /// way (C), rounded. In C a regular button is as tall as a field (36),
    /// a small one as a compact field (32) and a mini one smaller (28).
    /// Mixed themes in light also get a white window (`MIX_LIGHT_WINDOW`);
    /// strict keeps the measured one. Padding, font and radius stay
    /// AppKit's. Idempotent; the Kai theme only records `mix`.
    pub fn with_mix(self, mix: MixStyle) -> Theme {
        let ThemeKind::AppKit(generation) = self.kind else {
            return Theme { mix, ..self };
        };
        let base = if generation == AppKitGeneration::Aqua11 {
            AQUA11_CONTROLS
        } else {
            GLASS27_CONTROLS
        };
        let towards_kai = mix_fraction(mix);
        let lerp = |appkit: BySize, kai: BySize| -> BySize {
            std::array::from_fn(|i| (appkit[i] + towards_kai * (kai[i] - appkit[i])).round())
        };
        let mut controls = self.controls;
        controls.field_height = lerp(base.field_height, KAI_FIELD_HEIGHT);
        controls.field_square_height = lerp(base.field_square_height, KAI_FIELD_HEIGHT);
        controls.button_height = lerp(base.button_height, MIX_BUTTON_HEIGHT);
        let measured = match (generation == AppKitGeneration::Aqua11, self.appearance) {
            (true, Appearance::Light) => LIGHT,
            (true, Appearance::Dark) => DARK,
            (false, Appearance::Light) => GLASS27_LIGHT,
            (false, Appearance::Dark) => GLASS27_DARK,
        }
        .window_background;
        let window = if mix != MixStyle::AppKit && self.appearance == Appearance::Light {
            MIX_LIGHT_WINDOW
        } else {
            measured
        };
        let mut semantic = self.semantic;
        semantic.window_background = window;
        let mut colors = self.colors;
        colors.background = window;
        Theme {
            mix,
            controls,
            semantic,
            colors,
            ..self
        }
    }

    /// Static AppKit theme for `system`: the generation of its native
    /// controls ([`SystemInfo::generation`]) with the system tones of its
    /// OS (HIG 26 on 26+ even when the SDK keeps the Aqua11 look), and the
    /// default mixed-theme candidate ([`MixStyle::C`]).
    pub fn appkit_for(system: SystemInfo, appearance: Appearance) -> Theme {
        let mut theme =
            Theme::appkit_static(system.generation(), appearance).with_mix(MixStyle::default());
        theme.colors = color_tokens(
            &theme.semantic,
            &tones(system.has_hig26_tones(), appearance),
        );
        theme
    }

    /// [`Theme::appkit_for`] this process on this Mac.
    pub fn appkit_current(appearance: Appearance) -> Theme {
        Theme::appkit_for(SystemInfo::detect(), appearance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light() -> Theme {
        Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light)
    }

    fn dark() -> Theme {
        Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Dark)
    }

    #[test]
    fn label_is_black_847() {
        // Criterion of phase A0 (11 §11.7): `#000000` α .847.
        assert_eq!(light().color("label"), Some(Color::rgba(0, 0, 0, 216)));
        assert_eq!(dark().color("label"), Some(Color::rgba(255, 255, 255, 216)));
        // α .498 rounds to 127, not 128.
        assert_eq!(light().semantic.secondary_label.a, 127);
        assert_eq!(light().semantic.focus_ring.a, 127);
    }

    #[test]
    fn dark_window_background() {
        assert_eq!(dark().color("window"), Some(Color::hex(0x323232)));
        assert_eq!(light().color("window"), Some(Color::hex(0xececec)));
        assert_eq!(dark().color("background"), Some(Color::hex(0x323232)));
    }

    #[test]
    fn text_styles_match_hig() {
        let s = light().typography.styles;
        let table = [
            (s.large_title, 26.0, 32.0, 400),
            (s.title1, 22.0, 26.0, 400),
            (s.title2, 17.0, 22.0, 400),
            (s.title3, 15.0, 20.0, 400),
            (s.headline, 13.0, 16.0, 700),
            (s.body, 13.0, 16.0, 400),
            (s.callout, 12.0, 15.0, 400),
            (s.subheadline, 11.0, 14.0, 400),
            (s.footnote, 10.0, 13.0, 400),
            (s.caption1, 10.0, 13.0, 400),
            (s.caption2, 10.0, 13.0, 500),
        ];
        for (style, size, line_height, weight) in table {
            assert_eq!(
                (style.size, style.line_height, style.weight),
                (size, line_height, Weight(weight))
            );
        }
        assert!((s.body.tracking_em - (-0.08 / 13.0)).abs() < 1e-6);
        assert_eq!(s.callout.tracking_em, 0.0);
        let base = light().base_text();
        assert_eq!((base.font_size, base.line_height), (13.0, 16.0));
    }

    #[test]
    fn control_heights_match_measurements() {
        let c = light().controls;
        assert_eq!(c.button_height, [13.0, 16.0, 20.0, 28.0, 28.0]);
        assert_eq!(c.field_height, [17.0, 19.0, 22.0, 30.0, 30.0]);
        assert_eq!(c.field_square_height, [15.0, 19.0, 21.0, 21.0, 21.0]);
        assert_eq!(c.font_size, [9.0, 11.0, 13.0, 13.0, 13.0]);
        assert_eq!((c.focus_ring_width, c.focus_ring_offset), (3.0, 0.0));
        assert_eq!(light().control_size, ControlSize::Regular);
        assert_eq!(light().density, Density::Comfortable);
        assert_eq!(density_for(ControlSize::Small), Density::Compact);
        assert_eq!(density_for(ControlSize::Mini), Density::Compact);
        assert_eq!(density_for(ControlSize::Large), Density::Comfortable);
    }

    #[test]
    fn kai_tokens_are_filled_from_semantic() {
        for theme in [light(), dark()] {
            let (c, s) = (theme.colors, theme.semantic);
            assert_eq!(c.primary, s.control_accent);
            assert_eq!(c.accent, s.control_accent);
            assert_eq!(c.info, s.control_accent);
            assert_eq!(c.background, s.window_background);
            assert_eq!(c.surface, s.control_background);
            assert_eq!(c.secondary, s.control);
            assert_eq!(c.foreground, s.label);
            assert_eq!(c.text_primary, s.label);
            assert_eq!(c.muted_foreground, s.secondary_label);
            assert_eq!(c.text_disabled, s.disabled_control_text);
            assert_eq!(c.border, s.separator);
            assert_eq!(c.muted, s.fill[2]);
            assert_eq!(c.active, s.accent_pressed);
            assert_eq!(c.hover, s.fill[2]);
            assert_eq!(c.text_on_primary, s.on_accent);
        }
        assert_eq!(light().color("system-red"), Some(Color::hex(0xff3b30)));
        assert_eq!(dark().color("error"), Some(Color::hex(0xff453a)));
    }

    fn glass(appearance: Appearance) -> Theme {
        Theme::appkit_static(AppKitGeneration::Glass27, appearance)
    }

    #[test]
    fn glass27_control_metrics_match_macos_27() {
        let c = glass(Appearance::Light).controls;
        assert_eq!(c.button_height, [16.0, 20.0, 24.0, 28.0, 36.0]);
        assert_eq!(c.field_height, [19.0, 22.0, 24.0, 24.0, 24.0]);
        assert_eq!(c.field_square_height, c.field_height);
        assert_eq!(c.radius, [4.0, 5.0, 6.0, 14.0, 18.0]);
        // Large and extra large push buttons are capsules.
        assert_eq!(
            c.radius[3..],
            [c.button_height[3] / 2.0, c.button_height[4] / 2.0]
        );
        assert_eq!(c.field_radius, [4.0, 5.0, 6.0, 7.0, 9.0]);
        assert_eq!(c.field_square_radius, c.field_radius);
        assert_eq!(c.padding_x, [8.0, 10.0, 12.0, 14.0, 18.0]);
        assert_eq!(c.font_size, [9.0, 11.0, 13.0, 13.0, 13.0]);
        assert_eq!((c.focus_ring_width, c.focus_ring_offset), (3.0, 0.0));
        assert_eq!(glass(Appearance::Light).typography, light().typography);
    }

    #[test]
    fn aqua11_fields_and_padding() {
        let c = light().controls;
        assert_eq!(c.padding_x, [7.0, 8.0, 10.0, 10.0, 10.0]);
        assert_eq!(c.field_radius, c.radius);
        assert_eq!(c.field_square_radius, [0.0; 5]);
    }

    #[test]
    fn glass27_colors_differ_only_where_measured() {
        let (l, d) = (glass(Appearance::Light), glass(Appearance::Dark));
        assert_eq!(l.color("window"), Some(Color::hex(0xffffff)));
        assert_eq!(d.color("window"), Some(Color::hex(0x1e1e1e)));
        assert_eq!(l.color("under-page"), Some(Color::hex(0xf6f6f6)));
        assert_eq!(d.color("under-page"), Some(Color::hex(0x282828)));
        assert_eq!(l.semantic.placeholder, l.semantic.secondary_label);
        assert_eq!(d.semantic.placeholder, Color::rgba(255, 255, 255, 140));
        assert_eq!(l.color("background"), Some(Color::hex(0xffffff)));
        let unchanged = |g: SemanticColors, a: SemanticColors| SemanticColors {
            window_background: a.window_background,
            under_page: a.under_page,
            placeholder: a.placeholder,
            bezel_hover: a.bezel_hover,
            ..g
        };
        assert_eq!(unchanged(l.semantic, LIGHT), LIGHT);
        assert_eq!(unchanged(d.semantic, DARK), DARK);
        assert_eq!(l.color("system-red"), Some(Color::hex(0xff383c)));
        assert_eq!(d.color("error"), Some(Color::hex(0xff4245)));
        assert_eq!(l.color("success"), Some(Color::hex(0x34c759)));
        assert_eq!(d.color("warning"), Some(Color::hex(0xff9230)));
        assert_eq!(d.color("neutral"), Some(Color::hex(0x98989d)));
    }

    #[test]
    fn glass26_uses_the_glass27_tables_for_now() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let g26 = Theme::appkit_static(AppKitGeneration::Glass26, appearance);
            assert_eq!(g26.kind, ThemeKind::AppKit(AppKitGeneration::Glass26));
            assert_eq!(
                Theme {
                    kind: ThemeKind::AppKit(AppKitGeneration::Glass27),
                    ..g26
                },
                glass(appearance)
            );
        }
    }

    #[test]
    fn appkit_for_picks_generation_by_sdk_and_tones_by_os() {
        let v = system;
        // macOS 27 with SDK 15.5 (this Mac today): Aqua11 look, HIG 26 tones.
        let old_sdk = Theme::appkit_for(v((27, 0), Some((15, 5))), Appearance::Light);
        assert_eq!(old_sdk.kind, ThemeKind::AppKit(AppKitGeneration::Aqua11));
        assert_eq!(old_sdk.controls, light().with_mix(MixStyle::C).controls);
        assert_eq!(old_sdk.mix, MixStyle::C);
        assert_eq!(old_sdk.semantic, light().with_mix(MixStyle::C).semantic);
        assert_eq!(old_sdk.color("error"), Some(Color::hex(0xff383c)));
        let native = Theme::appkit_for(v((27, 0), Some((27, 0))), Appearance::Dark);
        assert_eq!(native, glass(Appearance::Dark).with_mix(MixStyle::C));
        let sequoia = Theme::appkit_for(v((15, 6), Some((15, 5))), Appearance::Light);
        assert_eq!(sequoia, light().with_mix(MixStyle::C));
    }

    #[test]
    fn mix_moves_field_heights_towards_kai() {
        let regular = |theme: Theme| {
            let c = theme.controls;
            (c.field_height[2], c.field_square_height[2])
        };
        for (mix, aqua, glass_h) in [
            (MixStyle::AppKit, (22.0, 21.0), (24.0, 24.0)),
            (MixStyle::A, (29.0, 29.0), (30.0, 30.0)),
            (MixStyle::B, (33.0, 32.0), (33.0, 33.0)),
            (MixStyle::C, (36.0, 36.0), (36.0, 36.0)),
        ] {
            assert_eq!(regular(light().with_mix(mix)), aqua, "{mix:?}");
            assert_eq!(
                regular(glass(Appearance::Dark).with_mix(mix)),
                glass_h,
                "{mix:?}"
            );
            // Idempotent, and nothing else changes (but the light window,
            // see `mixed_light_window_is_white`).
            let twice = light().with_mix(MixStyle::C).with_mix(mix);
            assert_eq!(twice, light().with_mix(mix));
            let strict = Theme {
                mix: MixStyle::AppKit,
                controls: light().controls,
                semantic: light().semantic,
                colors: light().colors,
                ..light().with_mix(mix)
            };
            assert_eq!(strict, light());
        }
        assert_eq!(light().mix, MixStyle::AppKit);
        assert_eq!(MixStyle::default(), MixStyle::C);
        assert_eq!(
            light().with_mix(MixStyle::C).controls.field_height,
            KAI_FIELD_HEIGHT
        );
        // The Kai theme only records the candidate.
        let kai = Theme::kai(Density::Comfortable);
        assert_eq!(kai.with_mix(MixStyle::C).controls, kai.controls);
        let (l, d) = (light().semantic, dark().semantic);
        assert_eq!(l.mix_secondary, kai.colors.secondary);
        assert_eq!(
            (l.mix_on_secondary, d.mix_on_secondary),
            (kai.colors.foreground, kai.colors.foreground)
        );
        assert_eq!(d.mix_secondary, Color::hex(0x00c4d4));
    }

    /// Mix moves text-button heights like the fields; in C regular buttons
    /// match regular fields (36), small ones compact fields (32) and mini
    /// ones are smaller (28). Mini < small at every mix level.
    #[test]
    fn mix_moves_button_heights_towards_kai() {
        for (theme, strict) in [
            (light(), [13.0, 16.0, 20.0, 28.0, 28.0]),
            (glass(Appearance::Light), [16.0, 20.0, 24.0, 28.0, 36.0]),
        ] {
            let buttons = |mix| theme.with_mix(mix).controls.button_height;
            assert_eq!(theme.controls.button_height, strict);
            assert_eq!(buttons(MixStyle::AppKit), strict);
            assert_eq!(buttons(MixStyle::C), [28.0, 32.0, 36.0, 44.0, 52.0]);
            for i in 0..5 {
                let (a, b) = (buttons(MixStyle::A)[i], buttons(MixStyle::B)[i]);
                assert!(strict[i] <= a && a <= b && b <= MIX_BUTTON_HEIGHT[i], "{i}");
            }
            for mix in [MixStyle::AppKit, MixStyle::A, MixStyle::B, MixStyle::C] {
                let h = buttons(mix);
                assert!(h[0] < h[1], "{mix:?} mini {} < small {}", h[0], h[1]);
            }
            let c = theme.with_mix(MixStyle::C).controls;
            assert_eq!(c.button_height[2], c.field_square_height[2]);
            assert_eq!(c.button_height[1], c.field_square_height[1]);
            assert_eq!(c.button_height[1..], c.field_height[1..]);
            assert!(c.button_height[0] < c.field_height[0]);
        }
        // Kai unchanged.
        let kai = Theme::kai(Density::Comfortable);
        assert_eq!(
            kai.with_mix(MixStyle::C).controls.button_height,
            kai.controls.button_height
        );
    }

    /// Mixed themes are white in light; strict and dark keep the measured
    /// window color; round trips back to strict restore it.
    #[test]
    fn mixed_light_window_is_white() {
        let white = Some(Color::hex(0xffffff));
        for generation in [AppKitGeneration::Aqua11, AppKitGeneration::Glass27] {
            for appearance in [Appearance::Light, Appearance::Dark] {
                let strict = Theme::appkit_static(generation, appearance);
                let measured = strict.color("window");
                for mix in [MixStyle::A, MixStyle::B, MixStyle::C] {
                    let mixed = strict.with_mix(mix);
                    let expected = if appearance == Appearance::Light {
                        white
                    } else {
                        measured
                    };
                    assert_eq!(mixed.color("window"), expected, "{generation:?} {mix:?}");
                    assert_eq!(mixed.color("background"), expected);
                    let back = mixed.with_mix(MixStyle::AppKit);
                    assert_eq!(back.color("window"), measured);
                    assert_eq!(back.color("background"), measured);
                }
            }
        }
        assert_eq!(light().color("window"), Some(Color::hex(0xececec)));
        let kai = Theme::kai(Density::Comfortable);
        assert_eq!(kai.with_mix(MixStyle::C).colors, kai.colors);
    }

    fn system(os: (u32, u32), sdk: Option<(u32, u32)>) -> SystemInfo {
        let v = |(major, minor)| crate::theme::MacVersion::new(major, minor);
        SystemInfo {
            os: Some(v(os)),
            sdk: sdk.map(v),
        }
    }
}
