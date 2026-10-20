//! Kai design tokens for the desktop renderer.
//!
//! Values mirror `kai-ui-web/src/theme/tokens.css` with `1rem = 16` logical points.
//! Components never read tokens directly: they use Tailwind-like classes
//! (`tw`) that resolve against [`Theme::color`], [`Theme::spacing_unit`] and
//! the type and control tokens. Tokens the desktop cannot honor yet are listed
//! in [`UNMAPPED_TOKENS`].
//!
//! Besides the web tokens ([`ColorTokens`]) every theme fills
//! [`SemanticColors`], named after AppKit's `NSColor` (`label`, `separator`,
//! `control-accent`…). New components and the painter use only those, so the
//! same code looks Kai with the Kai theme and Mac with an AppKit theme
//! (`docs/dev/12-modelo-appkit.md`, Paso 2).
//!
//! [`Theme::appkit_static`] (in `theme/appkit.rs`) builds the AppKit theme
//! from the tables of `docs/dev/11-tema-appkit.md`.

mod appkit;
mod generation;

use cosmic_text::Weight;

use crate::element::ControlSize;
use crate::geometry::Size;
use crate::render::Color;
use crate::style::ComputedText;

pub use generation::{MacVersion, SystemInfo};

pub const REM: f32 = 16.0;

/// `data-density` of the web contract. The responsive `@media` variants are
/// not mapped: a desktop window chooses its density explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Density {
    Comfortable,
    Compact,
}

/// Which design language a theme implements. Classes can target one with
/// the `kai:` and `appkit:` variants (resolved when parsing, see `tw.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    /// The Kai web look (`kai-ui-web` tokens).
    Kai,
    /// Native macOS controls of the given generation (Paso 13+).
    AppKit(AppKitGeneration),
}

/// AppKit visual generation (`docs/dev/11-tema-appkit.md` 11.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppKitGeneration {
    /// macOS 11–15; also the compact metrics of 26+.
    Aqua11,
    /// macOS 26 Tahoe (Liquid Glass). Unmeasured: same tables as
    /// `Glass27` for now.
    Glass26,
    /// macOS 27+ (measured on 27.0.1, 11 §6.3.8).
    Glass27,
}

/// How far the AppKit themes lean towards Kai ("tema mixto"). Classes
/// target a candidate with `mix-a:`, `mix-b:`, `mix-c:`; the Kai theme
/// ignores it. `AppKit` is the strict look of the measured tables.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MixStyle {
    /// Measured AppKit tables, AppKit components unchanged.
    AppKit,
    /// "Cerca de AppKit": fields at the AppKit–Kai midpoint height, tinted
    /// secondary bezel, faint outlines, Kai-shaped icon buttons at AppKit
    /// size.
    A,
    /// "Intermedio": fields at 3/4 of the way to Kai, secondary at 60 %,
    /// `tertiary-label` outlines, Kai-sized icon buttons with AppKit tones.
    B,
    /// "Cerca de Kai" (default, chosen by Felipe): Kai field height, border
    /// and radius; solid Kai secondary; Kai-style outlined and icon buttons
    /// in AppKit colors.
    #[default]
    C,
}

/// Field-label placement of the AppKit themes (experiment; classes target
/// one with `label-top:`, `label-near:`, `label-tab:`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldLabels {
    /// Original AppKit static label above the field (`KAI_LABEL=appkit`).
    AppKit,
    /// Kai's label on the top border, its lower half masked with the
    /// field color (`KAI_LABEL=top`).
    Top,
    /// Default (chosen by Felipe): static label above, in the border
    /// label's type (`text-xs` medium) and 2 pt from the field
    /// (`KAI_LABEL=near`).
    #[default]
    Near,
    /// On the top border inside a tab that extends the field: field fill
    /// and the field's border around it (`KAI_LABEL=tab`).
    Tab,
}

/// Light or dark appearance; classes can target one with `light:` and
/// `dark:`. Increased contrast is a [`SystemPrefs`] (`contrast-more:`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    Light,
    Dark,
}

/// System accessibility and input preferences that change how controls
/// behave. The runtime reads them from macOS at launch and whenever the
/// window becomes active, over the app's theme; classes can target them
/// with `motion-reduce:`, `contrast-more:` and `transparency-reduce:`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemPrefs {
    /// `NSApp.isFullKeyboardAccessEnabled` (System Settings › Keyboard ›
    /// Keyboard navigation): Tab reaches buttons and shows their focus ring.
    /// Kai keeps it on, as on the web.
    pub full_keyboard_access: bool,
    /// Accessibility › Display › Increase contrast: control outlines turn
    /// solid.
    pub increase_contrast: bool,
    /// Accessibility › Display › Reduce transparency.
    pub reduce_transparency: bool,
    /// Accessibility › Motion › Reduce motion: elements that move by the
    /// clock and the animated background stay still.
    pub reduce_motion: bool,
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

/// Colors with the meaning of `NSColor` (table 4.2 of
/// `docs/dev/11-tema-appkit.md`). Every theme fills them; new components and
/// the painter only use these. Class names (`text-label`, `bg-window`…) are
/// listed in [`Theme::color`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticColors {
    // Text
    /// `labelColor`.
    pub label: Color,
    /// `secondaryLabelColor`.
    pub secondary_label: Color,
    /// `tertiaryLabelColor`.
    pub tertiary_label: Color,
    /// `quaternaryLabelColor`.
    pub quaternary_label: Color,
    /// `placeholderTextColor`.
    pub placeholder: Color,
    /// `controlTextColor`.
    pub control_text: Color,
    /// `disabledControlTextColor`.
    pub disabled_control_text: Color,
    /// `alternateSelectedControlTextColor`: text on the accent or a selection.
    pub on_accent: Color,
    /// `linkColor`.
    pub link: Color,
    // Surfaces
    /// `windowBackgroundColor`.
    pub window_background: Color,
    /// `underPageBackgroundColor`.
    pub under_page: Color,
    /// `controlBackgroundColor`: large controls (tables, browsers).
    pub control_background: Color,
    /// `textBackgroundColor`: behind editable text (fields).
    pub text_background: Color,
    /// `controlColor`: face of a control (push button bezel).
    pub control: Color,
    /// `controlColor` with the `.pressed` system effect.
    pub control_pressed: Color,
    // Accent and selection
    /// `controlAccentColor`.
    pub control_accent: Color,
    /// Accent with the `.pressed` system effect.
    pub accent_pressed: Color,
    /// Accent with the `.disabled` system effect.
    pub accent_disabled: Color,
    /// `selectedContentBackgroundColor` (key window).
    pub selected_content: Color,
    /// `unemphasizedSelectedContentBackgroundColor` (inactive window).
    pub selected_content_inactive: Color,
    /// `selectedTextBackgroundColor`.
    pub text_selection: Color,
    /// `unemphasizedSelectedTextBackgroundColor`.
    pub text_selection_inactive: Color,
    /// `keyboardFocusIndicatorColor`: default color of `ring`.
    pub focus_ring: Color,
    /// `textInsertionPointColor`. The painter still draws the caret with the
    /// input's text color (Paso 10 switches the AppKit theme to this).
    pub caret: Color,
    // Lines and fills
    /// `separatorColor`: default border color.
    pub separator: Color,
    /// `gridColor`.
    pub grid: Color,
    /// `systemFill` … `quinarySystemFill`.
    pub fill: [Color; 5],
    // Hover and press steps of the mixed AppKit theme (not NSColors: AppKit
    // controls have no rollover; Kai keeps hover on purpose, 11 §7).
    /// Push bezel under the pointer.
    pub bezel_hover: Color,
    /// `controlAccentColor` under the pointer (accent/default buttons, on
    /// switch).
    pub accent_hover: Color,
    /// Pressed translucent bezel (glass push, bordered) and hovered off
    /// switch: one step above `systemFill`.
    pub fill_pressed: Color,
    /// Kai `secondary` for the mixed theme (`mix-secondary`): secondary
    /// buttons of candidates A–C.
    pub mix_secondary: Color,
    /// Text on a solid `mix-secondary` (`mix-on-secondary`).
    pub mix_on_secondary: Color,
    // Inactive window (not an NSColor; measured in 11 §6.2)
    /// Track of an on switch in an inactive window.
    pub switch_on_inactive: Color,
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

/// One macOS text style (`NSFont.TextStyle`, table 3.1 of
/// `docs/dev/11-tema-appkit.md`): `text-body`, `text-headline`…
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyleToken {
    pub size: f32,
    pub line_height: f32,
    pub weight: Weight,
    /// Letter spacing in em (`tracking`); 0 in the Kai theme.
    pub tracking_em: f32,
}

/// The eleven macOS text styles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyles {
    pub large_title: TextStyleToken,
    pub title1: TextStyleToken,
    pub title2: TextStyleToken,
    pub title3: TextStyleToken,
    pub headline: TextStyleToken,
    pub body: TextStyleToken,
    pub callout: TextStyleToken,
    pub subheadline: TextStyleToken,
    pub footnote: TextStyleToken,
    pub caption1: TextStyleToken,
    pub caption2: TextStyleToken,
}

impl TextStyles {
    /// Style by its class name (`text-large-title` … `text-caption2`).
    pub fn get(&self, name: &str) -> Option<TextStyleToken> {
        Some(match name {
            "large-title" => self.large_title,
            "title1" => self.title1,
            "title2" => self.title2,
            "title3" => self.title3,
            "headline" => self.headline,
            "body" => self.body,
            "callout" => self.callout,
            "subheadline" => self.subheadline,
            "footnote" => self.footnote,
            "caption1" => self.caption1,
            "caption2" => self.caption2,
            _ => return None,
        })
    }
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
    pub styles: TextStyles,
}

/// One value per [`crate::element::ControlSize`], indexed by
/// `ControlSize::index()`: mini, small, regular, large, extra large.
pub type BySize = [f32; 5];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlTokens {
    /// `h-control-compact`, `py-control` and the Kai fields keep using these.
    pub height_md: f32,
    pub padding_y_md: f32,
    pub height_compact: f32,
    pub padding_y_compact: f32,
    /// `h-control`, `min-h-control`: alignment height of a push button.
    pub button_height: BySize,
    /// `h-field`: rounded text field (`.roundedBezel`).
    pub field_height: BySize,
    /// `h-field-square`: square text field (`.squareBezel`).
    pub field_square_height: BySize,
    /// `rounded-control`.
    pub radius: BySize,
    /// `rounded-field`: rounded text field (`.roundedBezel`).
    pub field_radius: BySize,
    /// `rounded-field-square`: square text field (`.squareBezel`), square
    /// until macOS 26 and rounded like `rounded-field` from 27 on.
    pub field_square_radius: BySize,
    /// `px-control`.
    pub padding_x: BySize,
    /// `text-control`: font size and line height.
    pub font_size: BySize,
    pub line_height: BySize,
    /// `size-check`: side of a checkbox's box.
    pub check_size: BySize,
    /// `gap-control`: space between neighboring controls.
    pub gap: f32,
    /// `ring-focus`: 3 / 0 in AppKit, 2 / 2 in Kai.
    pub focus_ring_width: f32,
    pub focus_ring_offset: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub kind: ThemeKind,
    pub appearance: Appearance,
    pub density: Density,
    pub colors: ColorTokens,
    pub semantic: SemanticColors,
    pub space: SpaceTokens,
    pub typography: TypeTokens,
    pub controls: ControlTokens,
    pub prefs: SystemPrefs,
    /// Control size of the window root; nodes without `control_size`
    /// inherit it. Change it with `Theme { control_size, ..theme }`.
    pub control_size: ControlSize,
    /// `hover:`/`group-hover:` apply. On by default in every theme: the
    /// AppKit themes are "mixed" (AppKit look and logic, with hover).
    /// [`Theme::with_hover`]`(false)` gives strict AppKit. Hover is always
    /// off on disabled controls and, in AppKit, in an inactive window.
    pub hover_effects: bool,
    /// Mixed-theme candidate of the AppKit themes ([`Theme::with_mix`]).
    pub mix: MixStyle,
    /// Experiment (`KAI_LABEL=top|near|tab` in the demo): where AppKit
    /// text fields put their label. Default [`FieldLabels::Near`] in every
    /// AppKit theme (strict mix too); Kai ignores it.
    pub field_labels: FieldLabels,
}

/// `Density` that matches a control size (11 §6.5): `Small` and `Mini` are
/// compact, the rest comfortable.
pub fn density_for(size: ControlSize) -> Density {
    match size {
        ControlSize::Mini | ControlSize::Small => Density::Compact,
        ControlSize::Regular | ControlSize::Large | ControlSize::ExtraLarge => Density::Comfortable,
    }
}

/// Web tokens with no desktop equivalent yet, and why.
pub const UNMAPPED_TOKENS: &[(&str, &str)] = &[
    (
        "--font-sans, --font-mono, --font-display",
        "se usa la familia genérica SansSerif de cosmic-text (fuente del sistema); no hay carga de fuentes propias",
    ),
    (
        "--letter-spacing-*",
        "los tokens web de tracking no tienen clase propia; usar tracking-[Nem] o los estilos de texto",
    ),
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
                (2.25 * REM, 0.5 * REM, 2.0 * REM, 0.25 * REM),
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
                (2.0 * REM, 0.375 * REM, 1.75 * REM, 0.1875 * REM),
            ),
        };
        let controls = kai_controls(controls, &space, &typography);
        Self {
            kind: ThemeKind::Kai,
            // The web has no published dark tokens yet.
            appearance: Appearance::Light,
            density,
            semantic: kai_semantic(&colors),
            colors,
            space,
            typography,
            controls,
            // Kai buttons show their focus ring with Tab, as on the web.
            prefs: SystemPrefs {
                full_keyboard_access: true,
                ..SystemPrefs::default()
            },
            control_size: ControlSize::Regular,
            hover_effects: true,
            mix: MixStyle::default(),
            field_labels: FieldLabels::default(),
        }
    }

    /// Same theme with that field-label experiment ([`FieldLabels`]).
    pub fn with_field_labels(self, field_labels: FieldLabels) -> Self {
        Self {
            field_labels,
            ..self
        }
    }

    /// Same theme with hover effects on or off (`false` = strict AppKit,
    /// no rollover).
    pub fn with_hover(self, hover_effects: bool) -> Self {
        Self {
            hover_effects,
            ..self
        }
    }

    pub fn is_appkit(&self) -> bool {
        matches!(self.kind, ThemeKind::AppKit(_))
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

    /// Text properties of the window root, as the web `body`; the `body`
    /// text style (13 / 16) in AppKit.
    pub fn base_text(&self) -> ComputedText {
        let t = &self.typography;
        if self.is_appkit() {
            let body = t.styles.body;
            return ComputedText {
                color: self.semantic.label,
                font_size: body.size,
                line_height: body.line_height,
                weight: body.weight,
                nowrap: false,
                stroke_width: crate::icons::DEFAULT_STROKE_WIDTH,
                // Tracking only through `text-<style>` and `tracking-*`:
                // inherited, the 13 pt value would be wrong for other sizes.
                letter_spacing: 0.0,
            };
        }
        ComputedText {
            color: self.semantic.label,
            font_size: t.font_size_body,
            line_height: (t.font_size_body * t.line_height_normal).round(),
            weight: t.weight_regular,
            nowrap: false,
            stroke_width: crate::icons::DEFAULT_STROKE_WIDTH,
            letter_spacing: 0.0,
        }
    }

    /// Color token by its Tailwind name: the web names (`primary`,
    /// `muted-foreground`…) and the semantic AppKit names (`label`,
    /// `separator`, `control-accent`…).
    pub fn color(&self, name: &str) -> Option<Color> {
        let c = &self.colors;
        let s = &self.semantic;
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
            // `color-mix(in srgb, muted 50%, background)`: disabled and
            // read-only field surfaces.
            "field-muted" => c.muted.mix(c.background, 0.5),
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
            // Semantic (AppKit) names, `docs/dev/11-tema-appkit.md` 11.3.
            "label" => s.label,
            "secondary-label" => s.secondary_label,
            "tertiary-label" => s.tertiary_label,
            "quaternary-label" => s.quaternary_label,
            "placeholder" => s.placeholder,
            "control-text" => s.control_text,
            "disabled-control-text" => s.disabled_control_text,
            "on-accent" => s.on_accent,
            "link" => s.link,
            "window" => s.window_background,
            "under-page" => s.under_page,
            "control-background" => s.control_background,
            "field" => s.text_background,
            "control" => s.control,
            "control-pressed" => s.control_pressed,
            "control-accent" => s.control_accent,
            "accent-pressed" => s.accent_pressed,
            "accent-disabled" => s.accent_disabled,
            "selection" => s.selected_content,
            "selection-inactive" => s.selected_content_inactive,
            "text-selection" => s.text_selection,
            "text-selection-inactive" => s.text_selection_inactive,
            "focus" => s.focus_ring,
            "caret" => s.caret,
            "separator" => s.separator,
            "grid" => s.grid,
            "fill" => s.fill[0],
            "fill-2" => s.fill[1],
            "fill-3" => s.fill[2],
            "fill-4" => s.fill[3],
            "fill-5" => s.fill[4],
            "switch-on-inactive" => s.switch_on_inactive,
            "bezel-hover" => s.bezel_hover,
            "accent-hover" => s.accent_hover,
            "fill-pressed" => s.fill_pressed,
            "mix-secondary" => s.mix_secondary,
            "mix-on-secondary" => s.mix_on_secondary,
            // Selected or active choice (sidebar row, menu option, table
            // row, segment…): Kai `secondary` in every theme, as a tint
            // with a border. Never a text color: it is too light on white.
            "choice" => s.mix_secondary,
            // `systemRed`: `error` is filled from it in the AppKit theme.
            "system-red" => c.error,
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

/// Semantic colors of the Kai theme, derived from the web tokens so every
/// current use keeps its exact color.
fn kai_semantic(c: &ColorTokens) -> SemanticColors {
    SemanticColors {
        label: c.foreground,
        secondary_label: c.text_secondary,
        tertiary_label: c.text_disabled,
        quaternary_label: c.border,
        // Default placeholder of `paint.rs::input_contents`.
        placeholder: c.muted_foreground,
        control_text: c.foreground,
        disabled_control_text: c.text_disabled,
        on_accent: c.text_on_primary,
        link: c.primary,
        // Clear color of `paint.rs::paint`.
        window_background: c.background,
        under_page: c.neutral,
        control_background: c.surface,
        text_background: c.background,
        control: c.background,
        control_pressed: c.active,
        control_accent: c.primary,
        accent_pressed: c.active,
        accent_disabled: c.primary.with_alpha(128),
        selected_content: c.primary,
        selected_content_inactive: c.neutral,
        // Selection of `paint.rs::input_contents`.
        text_selection: c.primary.with_alpha(71),
        text_selection_inactive: c.neutral,
        // Default `ring` color of `paint.rs::decorations`.
        focus_ring: c.primary.with_alpha(128),
        caret: c.foreground,
        // Default border color of `paint.rs::decorations`.
        separator: c.border,
        grid: c.border,
        fill: [c.muted, c.muted, c.neutral, c.neutral, c.surface],
        // Kai does not dim the switch in inactive windows.
        switch_on_inactive: c.primary,
        // Unused by Kai components (they keep their own `hover:` classes).
        bezel_hover: c.hover,
        accent_hover: c.accent,
        fill_pressed: c.muted,
        mix_secondary: c.secondary,
        mix_on_secondary: c.foreground,
    }
}

/// Kai control metrics: `Regular` keeps today's `h-control` (`height_md`),
/// `text-sm leading-5`, `px-4` and `rounded-md`. Kai only had two heights;
/// the large sizes are an extrapolation no Kai component uses yet.
fn kai_controls(
    (height_md, padding_y_md, height_compact, padding_y_compact): (f32, f32, f32, f32),
    space: &SpaceTokens,
    t: &TypeTokens,
) -> ControlTokens {
    let heights = [
        height_compact,
        height_compact,
        height_md,
        height_md + 8.0,
        height_md + 16.0,
    ];
    // Tailwind line height scaled with the font size, as `tw::Parser::text`.
    let line = |size: f32, default_size: f32, default_line: f32| {
        (default_line * size / default_size).round()
    };
    // `leading-4` / `leading-5` are fixed rem values, as in Kai buttons.
    let (xs, sm, lg) = (
        REM,
        1.25 * REM,
        line(t.font_size_page_title, 1.125 * REM, 1.75 * REM),
    );
    ControlTokens {
        height_md,
        padding_y_md,
        height_compact,
        padding_y_compact,
        button_height: heights,
        field_height: heights,
        field_square_height: heights,
        radius: [0.375 * REM; 5],
        // Unused by Kai components; `rounded-md` like the controls.
        field_radius: [0.375 * REM; 5],
        field_square_radius: [0.375 * REM; 5],
        padding_x: [
            space.space_2,
            space.space_3,
            space.space_4,
            1.5 * space.space_4,
            2.0 * space.space_4,
        ],
        font_size: [
            t.font_size_xs,
            t.font_size_sm,
            t.font_size_sm,
            t.font_size_page_title,
            t.font_size_page_title,
        ],
        line_height: [xs, sm, sm, lg, lg],
        // `size-4` like the web checkbox; larger with the large text.
        check_size: [REM, REM, REM, 1.25 * REM, 1.25 * REM],
        gap: space.space_2,
        focus_ring_width: 2.0,
        focus_ring_offset: 2.0,
    }
}

/// Closest Kai equivalents of the macOS text styles (no tracking).
fn kai_text_styles(body: f32, sm: f32, xs: f32, page_title: f32, section_title: f32) -> TextStyles {
    let style = |size: f32, default_size: f32, default_line: f32, weight: u16| TextStyleToken {
        size,
        line_height: (default_line * size / default_size).round(),
        weight: Weight(weight),
        tracking_em: 0.0,
    };
    let body_style = |weight| style(body, REM, 1.5 * REM, weight);
    let sm_style = |weight| style(sm, 0.875 * REM, 1.25 * REM, weight);
    let xs_style = |weight| style(xs, 0.75 * REM, REM, weight);
    TextStyles {
        large_title: style(1.5 * REM, 1.5 * REM, 2.0 * REM, 600),
        title1: style(section_title, 1.25 * REM, 1.75 * REM, 600),
        title2: style(page_title, 1.125 * REM, 1.75 * REM, 600),
        title3: style(page_title, 1.125 * REM, 1.75 * REM, 500),
        headline: body_style(600),
        body: body_style(400),
        callout: sm_style(400),
        subheadline: sm_style(400),
        footnote: xs_style(400),
        caption1: xs_style(400),
        caption2: xs_style(500),
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
        styles: kai_text_styles(body, sm, xs, page_title, section_title),
    }
}

fn alpha(fraction: f32) -> u8 {
    (fraction * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::{AppKitGeneration, Appearance, Density, Theme, ThemeKind, UNMAPPED_TOKENS};
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
        assert_eq!(theme.color("field-muted"), Some(Color::hex(0xe7f7f8)));
        assert_eq!(theme.color("nope"), None);
    }

    #[test]
    fn kai_theme_is_kai_light() {
        let theme = Theme::kai(Density::Comfortable);
        assert_eq!(theme.kind, ThemeKind::Kai);
        assert_eq!(theme.appearance, Appearance::Light);
        assert!(!theme.is_appkit());
        let appkit = Theme {
            kind: ThemeKind::AppKit(AppKitGeneration::Aqua11),
            ..theme
        };
        assert!(appkit.is_appkit());
        assert_ne!(appkit, theme);
    }

    #[test]
    fn kai_semantic_matches_current_defaults() {
        let theme = Theme::kai(Density::Comfortable);
        let (c, s) = (theme.colors, theme.semantic);
        assert_eq!(s.focus_ring, c.primary.with_alpha(128));
        assert_eq!(s.text_selection, c.primary.with_alpha(71));
        assert_eq!(s.separator, c.border);
        assert_eq!(s.window_background, c.background);
        assert_eq!(s.placeholder, c.muted_foreground);
        assert_eq!(s.label, c.foreground);
        assert_eq!(theme.base_text().color, c.foreground);
        assert_eq!(Theme::kai(Density::Compact).semantic, s);
    }

    #[test]
    fn semantic_names_resolve() {
        let theme = Theme::kai(Density::Comfortable);
        for name in [
            "label",
            "secondary-label",
            "tertiary-label",
            "quaternary-label",
            "placeholder",
            "control-text",
            "disabled-control-text",
            "on-accent",
            "link",
            "window",
            "under-page",
            "control-background",
            "field",
            "control",
            "control-pressed",
            "control-accent",
            "accent-pressed",
            "accent-disabled",
            "selection",
            "selection-inactive",
            "text-selection",
            "text-selection-inactive",
            "focus",
            "caret",
            "separator",
            "grid",
            "fill",
            "fill-2",
            "fill-3",
            "fill-4",
            "fill-5",
            "bezel-hover",
            "accent-hover",
            "fill-pressed",
            "mix-secondary",
            "mix-on-secondary",
            "choice",
        ] {
            assert!(theme.color(name).is_some(), "{name}");
        }
        assert_eq!(theme.color("choice"), Some(theme.colors.secondary));
        let glass_dark = Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Dark);
        assert_eq!(glass_dark.color("choice"), Some(Color::hex(0x00c4d4)));
        assert_eq!(theme.color("separator"), Some(theme.colors.border));
        assert_eq!(theme.color("window"), Some(theme.colors.background));
        assert_eq!(theme.color("fill-5"), Some(theme.colors.surface));
        assert_eq!(theme.color("control-accent-nope"), None);
    }

    #[test]
    fn kai_control_metrics_keep_todays_regular_values() {
        for density in [Density::Comfortable, Density::Compact] {
            let theme = Theme::kai(density);
            let c = theme.controls;
            assert_eq!(c.button_height[2], c.height_md);
            assert_eq!(c.button_height[1], c.height_compact);
            assert_eq!(c.padding_x[2], theme.space.space_4, "px-4");
            assert_eq!(c.font_size[2], theme.typography.font_size_sm, "text-sm");
            assert_eq!(c.radius[2], 6.0, "rounded-md");
            assert_eq!((c.focus_ring_width, c.focus_ring_offset), (2.0, 2.0));
        }
        let comfortable = Theme::kai(Density::Comfortable);
        assert_eq!(comfortable.controls.line_height[2], 20.0, "leading-5");
        let body = comfortable.typography.styles.body;
        assert_eq!((body.size, body.line_height), (16.0, 24.0));
        assert_eq!(body.tracking_em, 0.0);
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
