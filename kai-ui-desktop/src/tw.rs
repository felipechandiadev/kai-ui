//! Tailwind-like utility classes resolved against the Kai theme.
//!
//! `ClassList::parse("flex items-center px-4 hover:bg-accent", &theme)` turns
//! a class string into declarations; [`ClassList::resolve`] applies them for
//! an interaction [`State`] and returns the computed [`Style`]. Spacing
//! utilities use [`Theme::spacing_unit`] (`p-4` = `--space-4`), colors use the
//! Kai token names (`bg-primary`, `text-muted-foreground`, `border-border/70`)
//! and arbitrary values are written as in Tailwind (`w-[180px]`,
//! `bg-[#1e73ae]`). Unknown classes are kept in [`ClassList::unknown`] so
//! tests can reject typos.
//!
//! Like generated Tailwind CSS, `leading-*` always wins over the line height
//! set by `text-*` sizes, and variants apply in a fixed order regardless of
//! their position in the string: base, group, hover, focus-within, focus,
//! control size (`control-small:`…), active, disabled. `hover:` and
//! `active:` do not apply to disabled elements.
//!
//! `focus-visible:` follows the browser heuristic: text fields always,
//! other controls only when the focus did not come from a pointer press.
//!
//! AppKit state variants: `keyboard-focus:` (focus shown by the keyboard:
//! text fields always, buttons only with full keyboard access),
//! `selected:`/`checked:` and `mixed:`/`indeterminate:` (control state),
//! `window-inactive:` (neither key nor main) and `control-mini:` …
//! `control-xl:` (effective control size). Priority, lowest first: base,
//! control size, group-hover, group-focus-within, hover, focus-within,
//! focus, focus-visible, keyboard-focus, selected, mixed, active,
//! window-inactive, disabled.
//!
//! Responsive variants as in Tailwind: `sm:` (window ≥ 640 pt), `md:`
//! (768), `lg:` (1024), `xl:` (1280), by the window width
//! ([`State::viewport_width`]); they rank just above base, wider ones
//! last: `grid grid-cols-1 sm:grid-cols-2` stacks in a narrow window.
//!
//! Control metrics follow the node's effective control size: `h-control`,
//! `min-h-control`, `h-field`, `h-field-square`, `size-check`, `rounded-control`,
//! `px-control` and `text-control` read the per-size arrays of
//! [`crate::theme::ControlTokens`]; `gap-control` and `ring-focus` (width,
//! offset and `focus` color) read the theme too. The macOS text styles are
//! `text-large-title`, `text-title1`…`text-title3`, `text-headline`,
//! `text-body`, `text-callout`, `text-subheadline`, `text-footnote`,
//! `text-caption1` and `text-caption2`; `tracking-[Nem]` sets letter spacing.
//!
//! Theme variants (`kai:`, `appkit:`, `dark:`, `light:`) are evaluated while
//! parsing: a class whose theme condition does not hold for the parsing
//! theme is dropped. They go before any state variant and can be stacked:
//! `appkit:dark:hover:bg-control`. `hover:appkit:x` is an unknown class.
//! [`crate::layout::StyleCache`] re-parses when the theme changes.

use cosmic_text::Weight;

use crate::element::ControlSize;
use crate::render::Color;
use crate::style::{
    Align, Cursor, Display, Edges, FlexDirection, Justify, Length, LineHeight, ObjectFit, Overflow,
    Position, Shadow, Style,
};
use crate::theme::{
    AppKitGeneration, Appearance, BySize, FieldLabels, MixStyle, REM, Theme, ThemeKind,
};

/// Interaction state of an element, and of its nearest `group` ancestor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub hover: bool,
    pub focus: bool,
    pub focus_within: bool,
    pub active: bool,
    pub disabled: bool,
    pub group_hover: bool,
    pub group_focus_within: bool,
    /// `:focus-visible`: focused text fields, and other focused controls
    /// unless the focus came from a pointer press.
    pub focus_visible: bool,
    /// Focused and shown as such: text fields always, other controls only
    /// when the focus came from the keyboard and full keyboard access is on.
    pub keyboard_focus: bool,
    /// The window is neither key nor main.
    pub window_inactive: bool,
    /// `ControlState::On`.
    pub selected: bool,
    /// `ControlState::Mixed`.
    pub mixed: bool,
    /// Effective (inherited) control size.
    pub control_size: ControlSize,
    /// Window (viewport) width in points, for the `sm:` … `xl:` breakpoints.
    pub viewport_width: u32,
    /// A drop zone while files are dragged over the window (`drag:`).
    pub file_drag: bool,
    /// Working on a task it started (`busy:`); see
    /// [`crate::element::Element::busy`].
    pub busy: bool,
    /// The nearest `group` ancestor is busy (`group-busy:`).
    pub group_busy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Base,
    /// Tailwind breakpoint (`sm:` 640, `md:` 768, `lg:` 1024, `xl:` 1280):
    /// the window is at least that wide.
    MinWidth(Breakpoint),
    ControlSize(ControlSize),
    GroupHover,
    GroupFocusWithin,
    GroupBusy,
    Hover,
    FocusWithin,
    Focus,
    FocusVisible,
    KeyboardFocus,
    Selected,
    Mixed,
    Active,
    FileDrag,
    Busy,
    WindowInactive,
    Disabled,
}

impl Variant {
    fn parse(prefix: &str) -> Option<Self> {
        Some(match prefix {
            "hover" => Self::Hover,
            "focus" => Self::Focus,
            "focus-visible" => Self::FocusVisible,
            "focus-within" => Self::FocusWithin,
            "active" => Self::Active,
            "disabled" => Self::Disabled,
            "group-hover" => Self::GroupHover,
            "group-focus-within" => Self::GroupFocusWithin,
            "group-busy" => Self::GroupBusy,
            "busy" => Self::Busy,
            "keyboard-focus" => Self::KeyboardFocus,
            "selected" | "checked" => Self::Selected,
            "mixed" | "indeterminate" => Self::Mixed,
            "window-inactive" => Self::WindowInactive,
            "drag" => Self::FileDrag,
            "sm" => Self::MinWidth(Breakpoint::Sm),
            "md" => Self::MinWidth(Breakpoint::Md),
            "lg" => Self::MinWidth(Breakpoint::Lg),
            "xl" => Self::MinWidth(Breakpoint::Xl),
            "control-mini" => Self::ControlSize(ControlSize::Mini),
            "control-small" => Self::ControlSize(ControlSize::Small),
            "control-regular" => Self::ControlSize(ControlSize::Regular),
            "control-large" => Self::ControlSize(ControlSize::Large),
            "control-xl" => Self::ControlSize(ControlSize::ExtraLarge),
            _ => return None,
        })
    }

    /// Priority: declarations of a higher rank override lower ones; within
    /// a rank, the string order decides.
    fn rank(self) -> u8 {
        match self {
            Self::Base => 0,
            // Wider breakpoints override narrower ones, as in Tailwind.
            Self::MinWidth(breakpoint) => 1 + breakpoint as u8,
            Self::ControlSize(_) => 5,
            Self::GroupHover => 6,
            Self::GroupFocusWithin => 7,
            Self::GroupBusy => 8,
            Self::Hover => 9,
            Self::FocusWithin => 10,
            Self::Focus => 11,
            Self::FocusVisible => 12,
            Self::KeyboardFocus => 13,
            Self::Selected => 14,
            Self::Mixed => 15,
            Self::Active => 16,
            Self::FileDrag => 17,
            Self::Busy => 18,
            // Beats selected/active: the accent turns off.
            Self::WindowInactive => 19,
            // Beats everything.
            Self::Disabled => 20,
        }
    }

    fn applies(self, state: State) -> bool {
        match self {
            Self::Base => true,
            Self::MinWidth(breakpoint) => state.viewport_width >= breakpoint.min_width(),
            Self::ControlSize(size) => state.control_size == size,
            Self::GroupHover => state.group_hover,
            Self::GroupFocusWithin => state.group_focus_within,
            Self::GroupBusy => state.group_busy,
            Self::Hover => state.hover && !state.disabled && !state.busy,
            Self::FocusWithin => state.focus_within,
            Self::Focus => state.focus,
            Self::FocusVisible => state.focus_visible,
            Self::KeyboardFocus => state.keyboard_focus,
            Self::Selected => state.selected,
            Self::Mixed => state.mixed,
            Self::Active => state.active && !state.disabled && !state.busy,
            Self::FileDrag => state.file_drag && !state.disabled,
            Self::Busy => state.busy,
            Self::WindowInactive => state.window_inactive,
            Self::Disabled => state.disabled,
        }
    }
}

/// Tailwind's default screens, by window width (Kai web uses the same).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Breakpoint {
    Sm,
    Md,
    Lg,
    Xl,
}

impl Breakpoint {
    fn min_width(self) -> u32 {
        match self {
            Self::Sm => 640,
            Self::Md => 768,
            Self::Lg => 1024,
            Self::Xl => 1280,
        }
    }
}

/// Condition on the theme; evaluated in [`ClassList::parse`], not in
/// [`ClassList::resolve`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeVariant {
    Kai,
    AppKit,
    /// AppKit 26+ (`Glass26`, `Glass27`).
    Glass,
    /// AppKit theme with that mixed-theme candidate (`mix-a:` …).
    Mix(MixStyle),
    /// AppKit theme with that field-label experiment (`label-top:`,
    /// `label-near:`, `label-tab:`; [`Theme::field_labels`]).
    Labels(FieldLabels),
    Dark,
    Light,
    /// System preferences ([`Theme::prefs`]): `motion-reduce:`,
    /// `contrast-more:`, `transparency-reduce:`.
    ReduceMotion,
    IncreaseContrast,
    ReduceTransparency,
}

impl ThemeVariant {
    fn parse(prefix: &str) -> Option<Self> {
        Some(match prefix {
            "kai" => Self::Kai,
            "appkit" => Self::AppKit,
            "glass" => Self::Glass,
            "mix-a" => Self::Mix(MixStyle::A),
            "mix-b" => Self::Mix(MixStyle::B),
            "mix-c" => Self::Mix(MixStyle::C),
            "label-top" => Self::Labels(FieldLabels::Top),
            "label-near" => Self::Labels(FieldLabels::Near),
            "label-tab" => Self::Labels(FieldLabels::Tab),
            "dark" => Self::Dark,
            "light" => Self::Light,
            "motion-reduce" => Self::ReduceMotion,
            "contrast-more" => Self::IncreaseContrast,
            "transparency-reduce" => Self::ReduceTransparency,
            _ => return None,
        })
    }

    fn holds(self, theme: &Theme) -> bool {
        match self {
            Self::Kai => !theme.is_appkit(),
            Self::AppKit => theme.is_appkit(),
            Self::Mix(mix) => theme.is_appkit() && theme.mix == mix,
            Self::Labels(labels) => theme.is_appkit() && theme.field_labels == labels,
            Self::Glass => matches!(
                theme.kind,
                ThemeKind::AppKit(AppKitGeneration::Glass26 | AppKitGeneration::Glass27)
            ),
            Self::Dark => theme.appearance == Appearance::Dark,
            Self::Light => theme.appearance == Appearance::Light,
            Self::ReduceMotion => theme.prefs.reduce_motion,
            Self::IncreaseContrast => theme.prefs.increase_contrast,
            Self::ReduceTransparency => theme.prefs.reduce_transparency,
        }
    }
}

/// Strips the leading theme variants of `class`, returning whether all of
/// them hold for `theme` and the rest of the class.
fn strip_theme_variants<'a>(mut class: &'a str, theme: &Theme) -> (bool, &'a str) {
    let mut holds = true;
    while let Some((prefix, rest)) = class.split_once(':')
        && let Some(variant) = ThemeVariant::parse(prefix)
    {
        holds &= variant.holds(theme);
        class = rest;
    }
    (holds, class)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Sides {
    All,
    X,
    Y,
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Decl {
    Display(Display, bool),
    Direction(FlexDirection),
    Wrap(bool),
    Flex(f32, f32, Length),
    Grow(f32),
    Shrink(f32),
    AlignItems(Align),
    AlignSelf(Align),
    Justify(Justify),
    Gap(Option<f32>, Option<f32>),
    Padding(Sides, f32),
    Margin(Sides, Length),
    Inset(Sides, Length),
    Width(Length),
    Height(Length),
    MinWidth(Length),
    MinHeight(Length),
    MaxWidth(Length),
    MaxHeight(Length),
    /// Overflow on the x and y axes; `None` leaves that axis.
    Overflow(Option<Overflow>, Option<Overflow>),
    Position(Position),
    Z(i32),
    GridColumns(u16),
    Background(Color),
    /// `bg-current`.
    BackgroundCurrent,
    BorderWidth(Sides, f32),
    BorderColor(Color),
    Radius(f32),
    Shadow(Option<Shadow>),
    RingWidth(f32),
    RingOffset(f32),
    RingColor(Color),
    Opacity(f32),
    Cursor(Cursor),
    PointerEvents(bool),
    ObjectFit(ObjectFit),
    /// Width ÷ height; `None` is `aspect-auto`.
    Aspect(Option<f32>),
    Placeholder(Color),
    TextColor(Color),
    FontSize(f32, Option<f32>),
    Leading(LineHeight),
    Weight(Weight),
    NoWrap(bool),
    Stroke(f32),
    // By control size (`h-control`, `px-control`…), indexed by
    // `ControlSize::index()` of the resolved state.
    HeightBySize(BySize),
    SizeBySize(BySize),
    MinHeightBySize(BySize),
    RadiusBySize(BySize),
    PaddingXBySize(BySize),
    FontBySize {
        size: BySize,
        line: BySize,
    },
    /// Letter spacing in em.
    Tracking(f32),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClassList {
    decls: Vec<(Variant, Decl)>,
    /// The element has the `group` class: descendants can use `group-*:`.
    pub group: bool,
    pub unknown: Vec<String>,
}

impl ClassList {
    pub fn parse(classes: &str, theme: &Theme) -> Self {
        let mut list = Self::default();
        for class in classes.split_whitespace() {
            let (applies, rest) = strip_theme_variants(class, theme);
            // `group` may carry a theme variant (`kai:group`, from `themed`).
            if rest == "group" {
                list.group |= applies;
                continue;
            }
            let (variant, utility) = match rest.split_once(':') {
                Some((prefix, utility)) => match Variant::parse(prefix) {
                    Some(variant) => (variant, utility),
                    None => {
                        list.unknown.push(class.to_owned());
                        continue;
                    }
                },
                None => (Variant::Base, rest),
            };
            let parser = Parser { theme };
            match parser.utility(utility) {
                // Still parsed when the theme variant does not hold, so a
                // typo behind `appkit:` is reported with the Kai theme too.
                Some(_) if !applies => {}
                Some(decls) => list
                    .decls
                    .extend(decls.into_iter().map(|decl| (variant, decl))),
                None => list.unknown.push(class.to_owned()),
            }
        }
        // Stable: within a rank the string order is kept.
        list.decls.sort_by_key(|(variant, _)| variant.rank());
        list
    }

    pub fn resolve(&self, state: State) -> Style {
        let mut style = Style::default();
        for rank in self.decls.chunk_by(|a, b| a.0.rank() == b.0.rank()) {
            let decls = rank.iter().filter(|(v, _)| v.applies(state));
            for (_, decl) in decls.clone() {
                if !matches!(decl, Decl::Leading(_)) {
                    apply(&mut style, decl, &state);
                }
            }
            for (_, decl) in decls {
                if matches!(decl, Decl::Leading(_)) {
                    apply(&mut style, decl, &state);
                }
            }
        }
        // As in CSS, an axis left visible clips when the other does not
        // (`overflow-y-auto` clips x without scrolling it).
        if style.overflow_x == Overflow::Visible && style.overflow_y != Overflow::Visible {
            style.overflow_x = Overflow::Hidden;
        } else if style.overflow_y == Overflow::Visible && style.overflow_x != Overflow::Visible {
            style.overflow_y = Overflow::Hidden;
        }
        style
    }

    /// Whether any declaration depends on the given interaction, so callers
    /// can skip re-resolving styles that cannot change.
    pub fn is_static(&self) -> bool {
        self.decls
            .iter()
            .all(|(variant, _)| *variant == Variant::Base)
    }
}

fn set_sides<T: Copy>(edges: &mut Edges<T>, sides: Sides, value: T) {
    match sides {
        Sides::All => *edges = Edges::all(value),
        Sides::X => {
            edges.left = value;
            edges.right = value;
        }
        Sides::Y => {
            edges.top = value;
            edges.bottom = value;
        }
        Sides::Top => edges.top = value,
        Sides::Right => edges.right = value,
        Sides::Bottom => edges.bottom = value,
        Sides::Left => edges.left = value,
    }
}

fn apply(style: &mut Style, decl: &Decl, state: &State) {
    let size = state.control_size.index();
    match *decl {
        Decl::Display(display, inline) => {
            style.display = display;
            style.inline = inline;
        }
        Decl::Direction(direction) => style.flex_direction = direction,
        Decl::Wrap(wrap) => style.flex_wrap = wrap,
        Decl::Flex(grow, shrink, basis) => {
            style.flex_grow = grow;
            style.flex_shrink = shrink;
            style.flex_basis = basis;
        }
        Decl::Grow(grow) => style.flex_grow = grow,
        Decl::Shrink(shrink) => style.flex_shrink = shrink,
        Decl::AlignItems(align) => style.align_items = Some(align),
        Decl::AlignSelf(align) => style.align_self = Some(align),
        Decl::Justify(justify) => style.justify_content = Some(justify),
        Decl::Gap(x, y) => {
            if let Some(x) = x {
                style.gap_x = x;
            }
            if let Some(y) = y {
                style.gap_y = y;
            }
        }
        Decl::Padding(sides, value) => set_sides(&mut style.padding, sides, value),
        Decl::Margin(sides, value) => set_sides(&mut style.margin, sides, value),
        Decl::Inset(sides, value) => set_sides(&mut style.inset, sides, value),
        Decl::Width(value) => style.width = value,
        Decl::Height(value) => style.height = value,
        Decl::MinWidth(value) => style.min_width = value,
        Decl::MinHeight(value) => style.min_height = value,
        Decl::MaxWidth(value) => style.max_width = value,
        Decl::MaxHeight(value) => style.max_height = value,
        Decl::Overflow(x, y) => {
            if let Some(x) = x {
                style.overflow_x = x;
            }
            if let Some(y) = y {
                style.overflow_y = y;
            }
        }
        Decl::Position(position) => style.position = position,
        Decl::Z(z) => style.z_index = z,
        Decl::GridColumns(columns) => style.grid_columns = columns,
        Decl::Background(color) => {
            style.background = Some(color);
            style.background_current = false;
        }
        Decl::BackgroundCurrent => style.background_current = true,
        Decl::BorderWidth(sides, width) => set_sides(&mut style.border_width, sides, width),
        Decl::BorderColor(color) => style.border_color = Some(color),
        Decl::Radius(radius) => style.radius = radius,
        Decl::Shadow(shadow) => style.shadow = shadow,
        Decl::RingWidth(width) => style.ring.width = width,
        Decl::RingOffset(offset) => style.ring.offset = offset,
        Decl::RingColor(color) => style.ring.color = Some(color),
        Decl::Opacity(opacity) => style.opacity = opacity,
        Decl::Cursor(cursor) => style.cursor = Some(cursor),
        Decl::PointerEvents(enabled) => style.pointer_events = enabled,
        Decl::ObjectFit(fit) => style.object_fit = fit,
        Decl::Aspect(ratio) => style.aspect_ratio = ratio,
        Decl::Placeholder(color) => style.placeholder_color = Some(color),
        Decl::TextColor(color) => style.text.color = Some(color),
        Decl::FontSize(size, line_height) => {
            style.text.font_size = Some(size);
            style.text.line_height = line_height.map(LineHeight::Px);
        }
        Decl::Leading(line_height) => style.text.line_height = Some(line_height),
        Decl::Weight(weight) => style.text.weight = Some(weight),
        Decl::NoWrap(nowrap) => style.text.nowrap = Some(nowrap),
        Decl::Stroke(width) => style.text.stroke_width = Some(width),
        Decl::HeightBySize(values) => style.height = Length::Px(values[size]),
        Decl::SizeBySize(values) => {
            style.width = Length::Px(values[size]);
            style.height = Length::Px(values[size]);
        }
        Decl::MinHeightBySize(values) => style.min_height = Length::Px(values[size]),
        Decl::RadiusBySize(values) => style.radius = values[size],
        Decl::PaddingXBySize(values) => set_sides(&mut style.padding, Sides::X, values[size]),
        Decl::FontBySize { size: sizes, line } => {
            style.text.font_size = Some(sizes[size]);
            style.text.line_height = Some(LineHeight::Px(line[size]));
        }
        Decl::Tracking(em) => style.text.letter_spacing = Some(em),
    }
}

struct Parser<'a> {
    theme: &'a Theme,
}

impl Parser<'_> {
    fn utility(&self, class: &str) -> Option<Vec<Decl>> {
        if let Some(decl) = self.keyword(class) {
            return Some(vec![decl]);
        }
        if let Some(decls) = self.compound(class) {
            return Some(decls);
        }
        if class == "truncate" {
            let hidden = Some(Overflow::Hidden);
            return Some(vec![Decl::NoWrap(true), Decl::Overflow(hidden, hidden)]);
        }
        let (negative, class) = match class.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, class),
        };
        let sign = |length: Length| match length {
            Length::Px(px) if negative => Length::Px(-px),
            Length::Percent(fraction) if negative => Length::Percent(-fraction),
            other => other,
        };
        let (prefix, value) = split_value(class)?;
        let decls = match prefix {
            "m" | "mx" | "my" | "mt" | "mr" | "mb" | "ml" => {
                vec![Decl::Margin(
                    sides(&prefix[1..])?,
                    sign(self.length(value)?),
                )]
            }
            "inset" | "inset-x" | "inset-y" | "top" | "right" | "bottom" | "left" => {
                let sides = match prefix {
                    "inset" => Sides::All,
                    "inset-x" => Sides::X,
                    "inset-y" => Sides::Y,
                    "top" => Sides::Top,
                    "right" => Sides::Right,
                    "bottom" => Sides::Bottom,
                    _ => Sides::Left,
                };
                vec![Decl::Inset(sides, sign(self.length(value)?))]
            }
            _ if negative => return None,
            "p" | "px" | "py" | "pt" | "pr" | "pb" | "pl" => {
                vec![Decl::Padding(sides(&prefix[1..])?, self.spacing(value)?)]
            }
            "gap" => {
                let gap = self.spacing(value)?;
                vec![Decl::Gap(Some(gap), Some(gap))]
            }
            "gap-x" => vec![Decl::Gap(Some(self.spacing(value)?), None)],
            "gap-y" => vec![Decl::Gap(None, Some(self.spacing(value)?))],
            "w" => vec![Decl::Width(self.length(value)?)],
            "h" => vec![Decl::Height(self.height(value)?)],
            "size" => {
                let length = self.length(value)?;
                vec![Decl::Width(length), Decl::Height(length)]
            }
            "min-w" => vec![Decl::MinWidth(self.length(value)?)],
            "min-h" => vec![Decl::MinHeight(self.height(value)?)],
            "max-w" => vec![Decl::MaxWidth(
                max_width(value).or_else(|| self.length(value))?,
            )],
            "max-h" => vec![Decl::MaxHeight(self.height(value)?)],
            "z" => vec![Decl::Z(value.parse().ok()?)],
            "grid-cols" => vec![Decl::GridColumns(value.parse().ok().filter(|n| *n > 0)?)],
            "grow" => vec![Decl::Grow(
                arbitrary(value)
                    .unwrap_or(value)
                    .parse()
                    .ok()
                    .filter(|grow: &f32| *grow >= 0.0)?,
            )],
            "flex" | "shrink" => return None,
            "bg" if value == "current" => vec![Decl::BackgroundCurrent],
            "bg" => vec![Decl::Background(self.color(value)?)],
            "border" => vec![self.border(Sides::All, value)?],
            "border-x" | "border-y" | "border-t" | "border-r" | "border-b" | "border-l" => {
                let sides = sides(&prefix[7..])?;
                vec![Decl::BorderWidth(sides, border_width(value)?)]
            }
            "rounded" => vec![Decl::Radius(radius(value)?)],
            "shadow" => vec![Decl::Shadow(shadow(value)?)],
            "ring" => vec![match value.parse::<f32>() {
                Ok(width) => Decl::RingWidth(width),
                Err(_) => Decl::RingColor(self.color(value)?),
            }],
            "ring-offset" => vec![Decl::RingOffset(value.parse().ok()?)],
            "opacity" => vec![Decl::Opacity(value.parse::<f32>().ok()? / 100.0)],
            "placeholder" => vec![Decl::Placeholder(self.color(value)?)],
            "text" => vec![self.text(value)?],
            "leading" => vec![Decl::Leading(leading(value)?)],
            "stroke" => {
                let width: f32 = arbitrary(value).unwrap_or(value).parse().ok()?;
                vec![Decl::Stroke(width)]
            }
            "tracking" => vec![Decl::Tracking(match value {
                "normal" => 0.0,
                _ => arbitrary(value)?.strip_suffix("em")?.parse().ok()?,
            })],
            _ => return None,
        };
        Some(decls)
    }

    fn keyword(&self, class: &str) -> Option<Decl> {
        let t = &self.theme.typography;
        let controls = &self.theme.controls;
        Some(match class {
            "block" => Decl::Display(Display::Block, false),
            "inline-block" => Decl::Display(Display::Block, true),
            "flex" => Decl::Display(Display::Flex, false),
            "inline-flex" => Decl::Display(Display::Flex, true),
            "grid" => Decl::Display(Display::Grid, false),
            "inline-grid" => Decl::Display(Display::Grid, true),
            "hidden" => Decl::Display(Display::None, false),
            "flex-row" => Decl::Direction(FlexDirection::Row),
            "flex-col" => Decl::Direction(FlexDirection::Column),
            "flex-wrap" => Decl::Wrap(true),
            "flex-nowrap" => Decl::Wrap(false),
            "flex-1" => Decl::Flex(1.0, 1.0, Length::Percent(0.0)),
            "flex-auto" => Decl::Flex(1.0, 1.0, Length::Auto),
            "flex-initial" => Decl::Flex(0.0, 1.0, Length::Auto),
            "flex-none" => Decl::Flex(0.0, 0.0, Length::Auto),
            "grow" => Decl::Grow(1.0),
            "grow-0" => Decl::Grow(0.0),
            "shrink" => Decl::Shrink(1.0),
            "shrink-0" => Decl::Shrink(0.0),
            "items-start" => Decl::AlignItems(Align::Start),
            "items-center" => Decl::AlignItems(Align::Center),
            "items-end" => Decl::AlignItems(Align::End),
            "items-stretch" => Decl::AlignItems(Align::Stretch),
            "items-baseline" => Decl::AlignItems(Align::Baseline),
            "self-start" => Decl::AlignSelf(Align::Start),
            "self-center" => Decl::AlignSelf(Align::Center),
            "self-end" => Decl::AlignSelf(Align::End),
            "self-stretch" => Decl::AlignSelf(Align::Stretch),
            "justify-start" => Decl::Justify(Justify::Start),
            "justify-center" => Decl::Justify(Justify::Center),
            "justify-end" => Decl::Justify(Justify::End),
            "justify-between" => Decl::Justify(Justify::Between),
            "justify-around" => Decl::Justify(Justify::Around),
            "justify-evenly" => Decl::Justify(Justify::Evenly),
            "relative" => Decl::Position(Position::Relative),
            "absolute" => Decl::Position(Position::Absolute),
            "overflow-visible" => Decl::Overflow(Some(Overflow::Visible), Some(Overflow::Visible)),
            "overflow-hidden" | "overflow-clip" => {
                Decl::Overflow(Some(Overflow::Hidden), Some(Overflow::Hidden))
            }
            "overflow-auto" | "overflow-scroll" => {
                Decl::Overflow(Some(Overflow::Auto), Some(Overflow::Auto))
            }
            "overflow-x-visible" => Decl::Overflow(Some(Overflow::Visible), None),
            "overflow-x-hidden" | "overflow-x-clip" => Decl::Overflow(Some(Overflow::Hidden), None),
            "overflow-x-auto" | "overflow-x-scroll" => Decl::Overflow(Some(Overflow::Auto), None),
            "overflow-y-visible" => Decl::Overflow(None, Some(Overflow::Visible)),
            "overflow-y-hidden" | "overflow-y-clip" => Decl::Overflow(None, Some(Overflow::Hidden)),
            "overflow-y-auto" | "overflow-y-scroll" => Decl::Overflow(None, Some(Overflow::Auto)),
            "border" => Decl::BorderWidth(Sides::All, 1.0),
            "border-x" => Decl::BorderWidth(Sides::X, 1.0),
            "border-y" => Decl::BorderWidth(Sides::Y, 1.0),
            "border-t" => Decl::BorderWidth(Sides::Top, 1.0),
            "border-r" => Decl::BorderWidth(Sides::Right, 1.0),
            "border-b" => Decl::BorderWidth(Sides::Bottom, 1.0),
            "border-l" => Decl::BorderWidth(Sides::Left, 1.0),
            "rounded" => Decl::Radius(0.25 * REM),
            "shadow" => Decl::Shadow(shadow("DEFAULT").flatten()),
            "ring" => Decl::RingWidth(3.0),
            "outline-none" => Decl::RingWidth(0.0),
            "cursor-default" => Decl::Cursor(Cursor::Default),
            "cursor-pointer" => Decl::Cursor(Cursor::Pointer),
            "cursor-text" => Decl::Cursor(Cursor::Text),
            "cursor-not-allowed" => Decl::Cursor(Cursor::NotAllowed),
            "cursor-wait" => Decl::Cursor(Cursor::Wait),
            "pointer-events-none" => Decl::PointerEvents(false),
            "pointer-events-auto" => Decl::PointerEvents(true),
            "object-fill" => Decl::ObjectFit(ObjectFit::Fill),
            "object-cover" => Decl::ObjectFit(ObjectFit::Cover),
            "object-contain" => Decl::ObjectFit(ObjectFit::Contain),
            "aspect-auto" => Decl::Aspect(None),
            "aspect-square" => Decl::Aspect(Some(1.0)),
            "aspect-video" => Decl::Aspect(Some(16.0 / 9.0)),
            "whitespace-nowrap" => Decl::NoWrap(true),
            "whitespace-normal" => Decl::NoWrap(false),
            "font-normal" => Decl::Weight(t.weight_regular),
            "font-medium" => Decl::Weight(t.weight_medium),
            "font-semibold" => Decl::Weight(t.weight_semibold),
            "font-bold" => Decl::Weight(t.weight_bold),
            // By the node's control size; `Regular` keeps the Kai values.
            "h-control" => Decl::HeightBySize(controls.button_height),
            "min-h-control" => Decl::MinHeightBySize(controls.button_height),
            "h-field" => Decl::HeightBySize(controls.field_height),
            "size-check" => Decl::SizeBySize(controls.check_size),
            "h-field-square" => Decl::HeightBySize(controls.field_square_height),
            "rounded-control" => Decl::RadiusBySize(controls.radius),
            "rounded-field" => Decl::RadiusBySize(controls.field_radius),
            "rounded-field-square" => Decl::RadiusBySize(controls.field_square_radius),
            "px-control" => Decl::PaddingXBySize(controls.padding_x),
            "text-control" => Decl::FontBySize {
                size: controls.font_size,
                line: controls.line_height,
            },
            "gap-control" => Decl::Gap(Some(controls.gap), Some(controls.gap)),
            "h-control-compact" => Decl::Height(Length::Px(controls.height_compact)),
            "py-control" => Decl::Padding(Sides::Y, controls.padding_y_md),
            _ => return None,
        })
    }

    /// Classes that expand to several declarations: `ring-focus` and the
    /// macOS text styles (`text-body`, `text-large-title`…).
    fn compound(&self, class: &str) -> Option<Vec<Decl>> {
        if class == "ring-focus" {
            let controls = &self.theme.controls;
            return Some(vec![
                Decl::RingWidth(controls.focus_ring_width),
                Decl::RingOffset(controls.focus_ring_offset),
                Decl::RingColor(self.theme.semantic.focus_ring),
            ]);
        }
        let style = self
            .theme
            .typography
            .styles
            .get(class.strip_prefix("text-")?)?;
        let mut decls = vec![
            Decl::FontSize(style.size, Some(style.line_height)),
            Decl::Weight(style.weight),
        ];
        if style.tracking_em != 0.0 {
            decls.push(Decl::Tracking(style.tracking_em));
        }
        Some(decls)
    }

    /// Tailwind spacing scale: `4` = 4 units, `1.5`, `px`, `[12px]`, `[0.5rem]`.
    fn spacing(&self, value: &str) -> Option<f32> {
        if value == "px" {
            return Some(1.0);
        }
        if let Some(arbitrary) = arbitrary(value) {
            return match parse_css_length(arbitrary)? {
                Length::Px(px) => Some(px),
                _ => None,
            };
        }
        let steps: f32 = value.parse().ok()?;
        (steps >= 0.0).then_some(steps * self.theme.spacing_unit())
    }

    fn length(&self, value: &str) -> Option<Length> {
        match value {
            "auto" => return Some(Length::Auto),
            "full" | "screen" => return Some(Length::Percent(1.0)),
            "fit" | "min" | "max" => return Some(Length::Auto),
            _ => {}
        }
        if let Some(arbitrary) = arbitrary(value) {
            return parse_css_length(arbitrary);
        }
        if let Some((numerator, denominator)) = value.split_once('/') {
            let numerator: f32 = numerator.parse().ok()?;
            let denominator: f32 = denominator.parse().ok()?;
            return (denominator > 0.0).then_some(Length::Percent(numerator / denominator));
        }
        self.spacing(value).map(Length::Px)
    }

    fn height(&self, value: &str) -> Option<Length> {
        match value {
            "control" => Some(Length::Px(self.theme.controls.height_md)),
            "control-compact" => Some(Length::Px(self.theme.controls.height_compact)),
            _ => self.length(value),
        }
    }

    /// `primary`, `muted-foreground/70`, `[#1e73ae]`, `white`, `transparent`.
    fn color(&self, value: &str) -> Option<Color> {
        let (name, alpha) = match value.rsplit_once('/') {
            Some((name, alpha)) if !name.starts_with('[') || name.ends_with(']') => {
                (name, Some(alpha.parse::<f32>().ok()? / 100.0))
            }
            _ => (value, None),
        };
        let color = match arbitrary(name) {
            Some(hex) => parse_hex(hex)?,
            None => self.theme.color(name)?,
        };
        Some(match alpha {
            Some(alpha) => color.with_alpha((f32::from(color.a) * alpha).round() as u8),
            None => color,
        })
    }

    fn border(&self, sides: Sides, value: &str) -> Option<Decl> {
        match border_width(value) {
            Some(width) => Some(Decl::BorderWidth(sides, width)),
            None => self.color(value).map(Decl::BorderColor),
        }
    }

    fn text(&self, value: &str) -> Option<Decl> {
        let t = &self.theme.typography;
        // Tailwind line heights, scaled with the density-dependent size.
        let size = |size: f32, default_size: f32, default_line: f32| {
            Decl::FontSize(size, Some((default_line * size / default_size).round()))
        };
        Some(match value {
            "xs" => size(t.font_size_xs, 0.75 * REM, REM),
            "sm" => size(t.font_size_sm, 0.875 * REM, 1.25 * REM),
            "base" => size(t.font_size_body, REM, 1.5 * REM),
            "lg" | "page-title" => size(t.font_size_page_title, 1.125 * REM, 1.75 * REM),
            "xl" | "section-title" => size(t.font_size_section_title, 1.25 * REM, 1.75 * REM),
            "2xl" => Decl::FontSize(1.5 * REM, Some(2.0 * REM)),
            "3xl" => Decl::FontSize(1.875 * REM, Some(2.25 * REM)),
            _ => {
                if let Some(arbitrary) = arbitrary(value)
                    && !arbitrary.starts_with('#')
                {
                    return match parse_css_length(arbitrary)? {
                        Length::Px(px) => Some(Decl::FontSize(px, None)),
                        _ => None,
                    };
                }
                Decl::TextColor(self.color(value)?)
            }
        })
    }
}

/// Split `px-4` into (`px`, `4`), keeping multi-word prefixes such as
/// `min-w`, `gap-x`, `ring-offset` or `border-t` together.
fn split_value(class: &str) -> Option<(&str, &str)> {
    const PREFIXES: [&str; 20] = [
        "min-w",
        "min-h",
        "max-w",
        "max-h",
        "gap-x",
        "gap-y",
        "inset-x",
        "inset-y",
        "grid-cols",
        "ring-offset",
        "border-x",
        "border-y",
        "border-t",
        "border-r",
        "border-b",
        "border-l",
        "placeholder",
        "rounded",
        "shadow",
        "opacity",
    ];
    for prefix in PREFIXES {
        if let Some(value) = class
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('-'))
        {
            return Some((prefix, value));
        }
    }
    if let Some(value) = class.strip_prefix('[') {
        return Some(("", value));
    }
    let index = class.find('-')?;
    Some((&class[..index], &class[index + 1..]))
}

fn sides(suffix: &str) -> Option<Sides> {
    Some(match suffix {
        "" => Sides::All,
        "x" => Sides::X,
        "y" => Sides::Y,
        "t" => Sides::Top,
        "r" => Sides::Right,
        "b" => Sides::Bottom,
        "l" => Sides::Left,
        _ => return None,
    })
}

fn arbitrary(value: &str) -> Option<&str> {
    value.strip_prefix('[')?.strip_suffix(']')
}

fn parse_css_length(value: &str) -> Option<Length> {
    if let Some(px) = value.strip_suffix("px") {
        return px.parse().ok().map(Length::Px);
    }
    if let Some(rem) = value.strip_suffix("rem") {
        return rem.parse::<f32>().ok().map(|rem| Length::Px(rem * REM));
    }
    if let Some(percent) = value.strip_suffix('%') {
        return percent
            .parse::<f32>()
            .ok()
            .map(|percent| Length::Percent(percent / 100.0));
    }
    value.parse().ok().map(Length::Px)
}

fn parse_hex(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    let parsed = u32::from_str_radix(hex, 16).ok()?;
    match hex.len() {
        6 => Some(Color::hex(parsed)),
        8 => Some(Color::hex(parsed >> 8).with_alpha(parsed as u8)),
        _ => None,
    }
}

fn border_width(value: &str) -> Option<f32> {
    if let Some(arbitrary) = arbitrary(value) {
        return match parse_css_length(arbitrary)? {
            Length::Px(px) => Some(px),
            _ => None,
        };
    }
    match value {
        "" => Some(1.0),
        "0" | "2" | "4" | "8" => value.parse().ok(),
        _ => None,
    }
}

fn radius(value: &str) -> Option<f32> {
    Some(match value {
        "none" => 0.0,
        "sm" => 0.125 * REM,
        "md" => 0.375 * REM,
        "lg" => 0.5 * REM,
        "xl" => 0.75 * REM,
        "2xl" => REM,
        "3xl" => 1.5 * REM,
        "full" => 9999.0,
        _ => match parse_css_length(arbitrary(value)?)? {
            Length::Px(px) => px,
            _ => return None,
        },
    })
}

fn shadow(value: &str) -> Option<Option<Shadow>> {
    let black = |alpha: f32| Color::rgba(0, 0, 0, (alpha * 255.0).round() as u8);
    let shadow = |offset_y: f32, blur: f32, spread: f32, alpha: f32| {
        Some(Some(Shadow {
            offset_y,
            blur,
            spread,
            color: black(alpha),
        }))
    };
    match value {
        "none" => Some(None),
        "sm" => shadow(1.0, 2.0, 0.0, 0.05),
        "DEFAULT" => shadow(1.0, 3.0, 0.0, 0.1),
        "md" => shadow(4.0, 6.0, -1.0, 0.1),
        "lg" => shadow(10.0, 15.0, -3.0, 0.1),
        "xl" => shadow(20.0, 25.0, -5.0, 0.1),
        // Push button bezel: 0.5 pt drop (11 §6.2) [C].
        "bezel" => shadow(0.5, 0.5, 0.0, 0.2),
        _ => None,
    }
}

fn leading(value: &str) -> Option<LineHeight> {
    Some(match value {
        "none" => LineHeight::Relative(1.0),
        "tight" => LineHeight::Relative(1.25),
        "snug" => LineHeight::Relative(1.375),
        "normal" => LineHeight::Relative(1.5),
        "relaxed" => LineHeight::Relative(1.625),
        "loose" => LineHeight::Relative(2.0),
        _ => match arbitrary(value) {
            Some(arbitrary) => match parse_css_length(arbitrary)? {
                Length::Px(px) if arbitrary.ends_with("px") || arbitrary.ends_with("rem") => {
                    LineHeight::Px(px)
                }
                Length::Px(factor) => LineHeight::Relative(factor),
                _ => return None,
            },
            None => LineHeight::Px(value.parse::<f32>().ok()? * 0.25 * REM),
        },
    })
}

fn max_width(value: &str) -> Option<Length> {
    let rem = match value {
        "none" => return Some(Length::Auto),
        "xs" => 20.0,
        "sm" => 24.0,
        "md" => 28.0,
        "lg" => 32.0,
        "xl" => 36.0,
        "2xl" => 42.0,
        "3xl" => 48.0,
        "4xl" => 56.0,
        "5xl" => 64.0,
        "6xl" => 72.0,
        "7xl" => 80.0,
        _ => return None,
    };
    Some(Length::Px(rem * REM))
}

#[cfg(test)]
mod tests {
    use cosmic_text::Weight;

    use super::{ClassList, State};
    use crate::element::ControlSize;
    use crate::render::Color;
    use crate::style::{
        Align, Cursor, Display, Edges, FlexDirection, Justify, Length, LineHeight, Overflow,
        Position,
    };
    use crate::theme::{AppKitGeneration, Appearance, Density, Theme, ThemeKind};

    fn appkit(appearance: Appearance) -> Theme {
        Theme {
            kind: ThemeKind::AppKit(AppKitGeneration::Aqua11),
            appearance,
            ..Theme::kai(Density::Comfortable)
        }
    }

    #[test]
    fn window_inactive_beats_selected_and_active() {
        let theme = Theme::kai(Density::Comfortable);
        let list = parse(
            "window-inactive:bg-control bg-primary selected:bg-control-accent \
             active:bg-error",
        );
        let state = State {
            selected: true,
            active: true,
            ..State::default()
        };
        assert_eq!(list.resolve(state).background, Some(theme.colors.error));
        let inactive = State {
            window_inactive: true,
            ..state
        };
        assert_eq!(
            list.resolve(inactive).background,
            Some(theme.semantic.control)
        );
        let selected = State {
            selected: true,
            ..State::default()
        };
        assert_eq!(
            list.resolve(selected).background,
            Some(theme.semantic.control_accent)
        );
    }

    #[test]
    fn disabled_beats_window_inactive() {
        let theme = Theme::kai(Density::Comfortable);
        let list = parse("disabled:bg-error window-inactive:bg-control");
        let state = State {
            window_inactive: true,
            disabled: true,
            ..State::default()
        };
        assert_eq!(list.resolve(state).background, Some(theme.colors.error));
    }

    fn sized(size: ControlSize) -> State {
        State {
            control_size: size,
            ..State::default()
        }
    }

    #[test]
    fn glass_variant_and_field_radii_follow_the_generation() {
        let classes = "bg-control glass:bg-fill-2 rounded-field";
        let square = "rounded-field-square";
        let regular = sized(ControlSize::Regular);
        for (generation, fill, radius, square_radius) in [
            (AppKitGeneration::Aqua11, "control", 5.0, 0.0),
            (AppKitGeneration::Glass26, "fill-2", 6.0, 6.0),
            (AppKitGeneration::Glass27, "fill-2", 6.0, 6.0),
        ] {
            let theme = Theme::appkit_static(generation, Appearance::Light);
            let list = ClassList::parse(classes, &theme);
            assert!(list.unknown.is_empty(), "{:?}", list.unknown);
            let style = list.resolve(regular);
            assert_eq!(style.background, theme.color(fill), "{generation:?}");
            assert_eq!(style.radius, radius, "{generation:?}");
            let style = ClassList::parse(square, &theme).resolve(regular);
            assert_eq!(style.radius, square_radius, "{generation:?}");
        }
        let kai = Theme::kai(Density::Comfortable);
        assert_eq!(
            ClassList::parse(classes, &kai).resolve(regular).background,
            kai.color("control")
        );
    }

    #[test]
    fn control_metrics_follow_control_size() {
        let mut theme = Theme::kai(Density::Comfortable);
        theme.controls.button_height = [13.0, 16.0, 20.0, 28.0, 36.0];
        theme.controls.radius = [3.0, 4.0, 5.0, 6.0, 7.0];
        theme.controls.font_size = [9.0, 11.0, 13.0, 13.0, 15.0];
        let list = ClassList::parse("h-control rounded-control px-control text-control", &theme);
        assert!(list.unknown.is_empty(), "{:?}", list.unknown);
        let heights: Vec<Length> = ControlSize::ALL
            .iter()
            .map(|size| list.resolve(sized(*size)).height)
            .collect();
        assert_eq!(
            heights,
            [13.0, 16.0, 20.0, 28.0, 36.0].map(Length::Px).to_vec()
        );
        let mini = list.resolve(sized(ControlSize::Mini));
        assert_eq!(mini.radius, 3.0);
        assert_eq!(mini.text.font_size, Some(9.0));
        assert_eq!(mini.padding.left, theme.controls.padding_x[0]);
        assert_eq!(
            ClassList::parse("min-h-control", &theme)
                .resolve(sized(ControlSize::Large))
                .min_height,
            Length::Px(28.0)
        );
    }

    #[test]
    fn h_control_unchanged_for_kai_regular() {
        for density in [Density::Comfortable, Density::Compact] {
            let theme = Theme::kai(density);
            let style = ClassList::parse("h-control min-h-control text-control", &theme)
                .resolve(State::default());
            assert_eq!(style.height, Length::Px(theme.controls.height_md));
            assert_eq!(style.min_height, Length::Px(theme.controls.height_md));
            let sm = ClassList::parse("text-sm leading-5", &theme).resolve(State::default());
            assert_eq!(style.text.font_size, sm.text.font_size);
            assert_eq!(style.text.line_height, sm.text.line_height);
        }
    }

    #[test]
    fn ring_focus_reads_theme() {
        let kai = Theme::kai(Density::Comfortable);
        let ring = parse("ring-focus").resolve(State::default()).ring;
        assert_eq!((ring.width, ring.offset), (2.0, 2.0));
        assert_eq!(ring.color, Some(kai.semantic.focus_ring));
        let appkit = Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light);
        let list = ClassList::parse("ring-0 keyboard-focus:ring-focus gap-control", &appkit);
        let focused = list.resolve(State {
            keyboard_focus: true,
            ..State::default()
        });
        assert_eq!((focused.ring.width, focused.ring.offset), (3.0, 0.0));
        assert_eq!((focused.gap_x, focused.gap_y), (8.0, 8.0));
        assert_eq!(list.resolve(State::default()).ring.width, 0.0);
    }

    #[test]
    fn text_styles_parse() {
        let appkit = Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light);
        for name in [
            "large-title",
            "title1",
            "title2",
            "title3",
            "headline",
            "body",
            "callout",
            "subheadline",
            "footnote",
            "caption1",
            "caption2",
        ] {
            let class = format!("text-{name}");
            for theme in [Theme::kai(Density::Comfortable), appkit] {
                let list = ClassList::parse(&class, &theme);
                assert!(list.unknown.is_empty(), "{class}");
                let style = list.resolve(State::default());
                let token = theme.typography.styles.get(name).unwrap();
                assert_eq!(style.text.font_size, Some(token.size));
                assert_eq!(
                    style.text.line_height,
                    Some(LineHeight::Px(token.line_height))
                );
                assert_eq!(style.text.weight, Some(token.weight));
            }
        }
        let headline = ClassList::parse("text-headline", &appkit).resolve(State::default());
        assert_eq!(headline.text.weight, Some(Weight(700)));
        assert_eq!(headline.text.letter_spacing, Some(-0.08 / 13.0));
        assert_eq!(
            parse("text-body")
                .resolve(State::default())
                .text
                .letter_spacing,
            None
        );
        assert_eq!(
            parse("tracking-[-0.01em]")
                .resolve(State::default())
                .text
                .letter_spacing,
            Some(-0.01)
        );
        assert_eq!(
            ClassList::parse("text-title4 tracking-[2px]", &appkit).unknown,
            vec!["text-title4", "tracking-[2px]"]
        );
    }

    #[test]
    fn group_can_carry_a_theme_variant() {
        let kai = Theme::kai(Density::Comfortable);
        let appkit = Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light);
        assert!(ClassList::parse("kai:group", &kai).group);
        assert!(!ClassList::parse("kai:group", &appkit).group);
        assert!(ClassList::parse("appkit:group", &appkit).unknown.is_empty());
        let bezel = ClassList::parse("shadow-bezel", &appkit).resolve(State::default());
        assert_eq!(bezel.shadow.map(|s| (s.offset_y, s.blur)), Some((0.5, 0.5)));
    }

    #[test]
    fn focus_visible_is_its_own_state() {
        let list = parse("ring-0 focus:ring-1 focus-visible:ring-2 keyboard-focus:ring-3");
        let pointer = State {
            focus: true,
            ..State::default()
        };
        assert_eq!(list.resolve(pointer).ring.width, 1.0);
        let visible = State {
            focus_visible: true,
            ..pointer
        };
        assert_eq!(list.resolve(visible).ring.width, 2.0);
        let keyboard = State {
            keyboard_focus: true,
            ..visible
        };
        assert_eq!(list.resolve(keyboard).ring.width, 3.0);
    }

    #[test]
    fn mixed_and_keyboard_focus_variants() {
        let theme = Theme::kai(Density::Comfortable);
        let list = parse("ring-0 focus:ring-1 keyboard-focus:ring-3 indeterminate:bg-error");
        let pointer_focus = State {
            focus: true,
            ..State::default()
        };
        assert_eq!(list.resolve(pointer_focus).ring.width, 1.0);
        let keyboard = State {
            keyboard_focus: true,
            ..pointer_focus
        };
        assert_eq!(list.resolve(keyboard).ring.width, 3.0);
        let mixed = State {
            mixed: true,
            ..State::default()
        };
        assert_eq!(list.resolve(mixed).background, Some(theme.colors.error));
        assert_eq!(list.resolve(State::default()).background, None);
    }

    #[test]
    fn control_size_variants_pick_one() {
        use crate::element::ControlSize;

        let list = parse("h-4 control-small:h-3 control-large:h-7 hover:h-8");
        let height = |control_size: ControlSize, hover: bool| {
            list.resolve(State {
                control_size,
                hover,
                ..State::default()
            })
            .height
        };
        assert_eq!(height(ControlSize::Regular, false), Length::Px(16.0));
        assert_eq!(height(ControlSize::Small, false), Length::Px(12.0));
        assert_eq!(height(ControlSize::Large, false), Length::Px(28.0));
        assert_eq!(height(ControlSize::Mini, false), Length::Px(16.0));
        assert_eq!(height(ControlSize::Large, true), Length::Px(32.0));
        assert!(!list.is_static());
    }

    #[test]
    fn theme_variants_filter_at_parse_time() {
        let kai = Theme::kai(Density::Comfortable);
        let classes = "kai:bg-primary appkit:bg-error";
        let list = ClassList::parse(classes, &kai);
        assert!(list.unknown.is_empty(), "{:?}", list.unknown);
        assert_eq!(
            list.resolve(State::default()).background,
            Some(kai.colors.primary)
        );
        let list = ClassList::parse(classes, &appkit(Appearance::Light));
        assert!(list.unknown.is_empty(), "{:?}", list.unknown);
        assert_eq!(
            list.resolve(State::default()).background,
            Some(kai.colors.error)
        );
        // Dropped declarations do not make a list interaction-dependent.
        assert!(ClassList::parse("appkit:hover:bg-error", &kai).is_static());
        let hovered = State {
            hover: true,
            ..State::default()
        };
        let list = ClassList::parse("kai:hover:bg-error", &kai);
        assert_eq!(list.resolve(hovered).background, Some(kai.colors.error));
    }

    #[test]
    fn system_preference_variants() {
        let kai = Theme::kai(Density::Comfortable);
        let classes = "bg-primary contrast-more:bg-error transparency-reduce:border-error \
                       motion-reduce:text-error";
        let plain = ClassList::parse(classes, &kai);
        assert!(plain.unknown.is_empty(), "{:?}", plain.unknown);
        let plain = plain.resolve(State::default());
        assert_eq!(plain.background, Some(kai.colors.primary));
        assert_eq!(plain.border_color, None);
        assert_eq!(plain.text.color, None);
        let mut prefs = kai;
        prefs.prefs.increase_contrast = true;
        prefs.prefs.reduce_transparency = true;
        prefs.prefs.reduce_motion = true;
        let style = ClassList::parse(classes, &prefs).resolve(State::default());
        assert_eq!(style.background, Some(kai.colors.error));
        assert_eq!(style.border_color, Some(kai.colors.error));
        assert_eq!(style.text.color, Some(kai.colors.error));
    }

    #[test]
    fn stacked_theme_variants() {
        let classes = "appkit:dark:text-label";
        let color = |theme: &Theme| {
            let list = ClassList::parse(classes, theme);
            assert!(list.unknown.is_empty(), "{:?}", list.unknown);
            list.resolve(State::default()).text.color
        };
        let kai = Theme::kai(Density::Comfortable);
        let dark_kai = Theme {
            appearance: Appearance::Dark,
            ..kai
        };
        assert_eq!(color(&kai), None);
        assert_eq!(color(&dark_kai), None);
        assert_eq!(color(&appkit(Appearance::Light)), None);
        assert_eq!(color(&appkit(Appearance::Dark)), Some(kai.semantic.label));
        let light = ClassList::parse("light:bg-window dark:bg-black", &kai);
        assert_eq!(
            light.resolve(State::default()).background,
            Some(kai.semantic.window_background)
        );
    }

    #[test]
    fn theme_variant_after_state_is_unknown() {
        let kai = Theme::kai(Density::Comfortable);
        let list = ClassList::parse("hover:appkit:bg-primary", &kai);
        assert_eq!(list.unknown, vec!["hover:appkit:bg-primary".to_owned()]);
        // A typo behind a theme variant that does not hold is still reported.
        let list = ClassList::parse("appkit:bg-nope", &kai);
        assert_eq!(list.unknown, vec!["appkit:bg-nope".to_owned()]);
    }

    fn parse(classes: &str) -> ClassList {
        let list = ClassList::parse(classes, &Theme::kai(Density::Comfortable));
        assert!(list.unknown.is_empty(), "unknown: {:?}", list.unknown);
        list
    }

    #[test]
    fn layout_utilities_map_to_css_box_properties() {
        let style = parse(
            "flex flex-col items-center justify-between gap-4 px-6 py-1.5 mt-2 -mb-1 mx-auto \
             w-full max-w-2xl min-h-[180px] h-1/2 overflow-y-auto absolute inset-0 z-20 shrink-0",
        )
        .resolve(State::default());
        assert_eq!(style.display, Display::Flex);
        assert_eq!(style.flex_direction, FlexDirection::Column);
        assert_eq!(style.align_items, Some(Align::Center));
        assert_eq!(style.justify_content, Some(Justify::Between));
        assert_eq!((style.gap_x, style.gap_y), (16.0, 16.0));
        assert_eq!(style.padding.left, 24.0);
        assert_eq!(style.padding.top, 6.0);
        assert_eq!(style.margin.top, Length::Px(8.0));
        assert_eq!(style.margin.bottom, Length::Px(-4.0));
        assert_eq!(style.margin.left, Length::Auto);
        assert_eq!(style.width, Length::Percent(1.0));
        assert_eq!(style.max_width, Length::Px(42.0 * 16.0));
        assert_eq!(style.min_height, Length::Px(180.0));
        assert_eq!(style.height, Length::Percent(0.5));
        assert_eq!(style.overflow_y, Overflow::Auto);
        assert_eq!(style.overflow_x, Overflow::Hidden, "clipped, not scrolled");
        let both = parse("overflow-x-auto overflow-y-auto").resolve(State::default());
        assert_eq!(
            (both.overflow_x, both.overflow_y),
            (Overflow::Auto, Overflow::Auto)
        );
        let x = parse("overflow-x-auto").resolve(State::default());
        assert_eq!(
            (x.overflow_x, x.overflow_y),
            (Overflow::Auto, Overflow::Hidden)
        );
        let auto = parse("overflow-auto").resolve(State::default());
        assert_eq!(
            (auto.overflow_x, auto.overflow_y),
            (Overflow::Auto, Overflow::Auto)
        );
        assert_eq!(style.position, Position::Absolute);
        assert_eq!(style.inset, Edges::all(Length::Px(0.0)));
        assert_eq!(style.z_index, 20);
        assert_eq!(style.flex_shrink, 0.0);
    }

    #[test]
    fn paint_and_text_utilities_resolve_theme_tokens() {
        let theme = Theme::kai(Density::Comfortable);
        let style = parse(
            "bg-primary border border-border/70 rounded-md shadow-sm ring-2 ring-primary/40 \
             ring-offset-2 opacity-50 text-sm leading-5 font-medium text-white cursor-pointer \
             placeholder-muted-foreground whitespace-nowrap bg-[#00deef]",
        )
        .resolve(State::default());
        assert_eq!(style.background, Some(Color::hex(0x00deef)));
        assert_eq!(style.border_width, Edges::all(1.0));
        assert_eq!(
            style.border_color,
            Some(theme.colors.border.with_alpha(179))
        );
        assert_eq!(style.radius, 6.0);
        assert_eq!(style.shadow.unwrap().blur, 2.0);
        assert_eq!((style.ring.width, style.ring.offset), (2.0, 2.0));
        assert_eq!(style.ring.color, Some(theme.colors.primary.with_alpha(102)));
        assert_eq!(style.opacity, 0.5);
        assert_eq!(style.text.font_size, Some(14.0));
        assert_eq!(style.text.line_height, Some(LineHeight::Px(20.0)));
        assert_eq!(style.text.weight, Some(Weight(500)));
        assert_eq!(style.text.color, Some(Color::hex(0xffffff)));
        assert_eq!(style.text.nowrap, Some(true));
        assert_eq!(style.cursor, Some(Cursor::Pointer));
        assert_eq!(style.placeholder_color, Some(theme.colors.muted_foreground));
        let stroke = |class: &str| parse(class).resolve(State::default()).text.stroke_width;
        assert_eq!(stroke("stroke-1"), Some(1.0));
        assert_eq!(stroke("stroke-[2.25]"), Some(2.25));
    }

    #[test]
    fn drag_variant_needs_a_file_drag_and_loses_to_disabled() {
        let list = parse("border-0 drag:border-2 disabled:border");
        let width = |file_drag, disabled| {
            list.resolve(State {
                file_drag,
                disabled,
                ..State::default()
            })
            .border_width
            .top
        };
        assert_eq!(width(false, false), 0.0);
        assert_eq!(width(true, false), 2.0);
        assert_eq!(width(true, true), 1.0);
    }

    #[test]
    fn object_fit_classes() {
        use crate::style::ObjectFit;
        let fit = |classes| parse(classes).resolve(State::default()).object_fit;
        assert_eq!(fit(""), ObjectFit::Fill);
        assert_eq!(fit("object-cover"), ObjectFit::Cover);
        assert_eq!(fit("object-contain"), ObjectFit::Contain);
        assert_eq!(fit("object-cover object-fill"), ObjectFit::Fill);
        let aspect = |classes| parse(classes).resolve(State::default()).aspect_ratio;
        assert_eq!(aspect("aspect-video"), Some(16.0 / 9.0));
        assert_eq!(aspect("aspect-square aspect-auto"), None);
    }

    #[test]
    fn leading_wins_over_text_size_regardless_of_order() {
        let style = parse("leading-tight text-xl").resolve(State::default());
        assert_eq!(style.text.font_size, Some(20.0));
        assert_eq!(style.text.line_height, Some(LineHeight::Relative(1.25)));
    }

    #[test]
    fn variants_apply_in_css_priority_and_skip_hover_when_disabled() {
        let list = parse(
            "bg-primary active:bg-active hover:bg-accent focus:ring-2 disabled:opacity-50 \
             group-hover:text-primary focus-within:border-primary",
        );
        let theme = Theme::kai(Density::Comfortable);
        let base = list.resolve(State::default());
        assert_eq!(base.background, Some(theme.colors.primary));
        assert_eq!(base.ring.width, 0.0);

        let hovered = list.resolve(State {
            hover: true,
            ..State::default()
        });
        assert_eq!(hovered.background, Some(theme.colors.accent));
        let pressed = list.resolve(State {
            hover: true,
            active: true,
            focus: true,
            ..State::default()
        });
        assert_eq!(pressed.background, Some(theme.colors.active));
        assert_eq!(pressed.ring.width, 2.0);

        let disabled = list.resolve(State {
            hover: true,
            active: true,
            disabled: true,
            ..State::default()
        });
        assert_eq!(disabled.background, Some(theme.colors.primary));
        assert_eq!(disabled.opacity, 0.5);

        let group = list.resolve(State {
            group_hover: true,
            focus_within: true,
            ..State::default()
        });
        assert_eq!(group.text.color, Some(theme.colors.primary));
        assert_eq!(group.border_color, Some(theme.colors.primary));
        assert!(!list.is_static());
        assert!(parse("p-4").is_static());
    }

    #[test]
    fn spacing_scales_with_density_but_arbitrary_values_do_not() {
        let compact = Theme::kai(Density::Compact);
        let style =
            ClassList::parse("p-4 gap-[10px] h-control", &compact).resolve(State::default());
        assert_eq!(style.padding.top, compact.space.space_4);
        assert_eq!(style.gap_x, 10.0);
        assert_eq!(style.height, Length::Px(compact.controls.height_md));
    }

    #[test]
    fn breakpoints_follow_the_window_width_wider_ones_win() {
        let list = parse("grid-cols-1 lg:grid-cols-4 sm:grid-cols-2 md:grid-cols-3");
        let columns = |viewport_width| {
            list.resolve(State {
                viewport_width,
                ..State::default()
            })
            .grid_columns
        };
        assert_eq!(columns(0), 1);
        assert_eq!(columns(639), 1);
        assert_eq!(columns(640), 2);
        assert_eq!(columns(768), 3);
        assert_eq!(columns(1023), 3);
        assert_eq!(columns(1024), 4);
        // Below state variants: `hover:` still beats `sm:`.
        let list = parse("p-1 sm:p-2 hover:p-3");
        let state = State {
            viewport_width: 800,
            hover: true,
            ..State::default()
        };
        let p3 = parse("p-3").resolve(State::default()).padding.left;
        assert_eq!(list.resolve(state).padding.left, p3);
    }

    #[test]
    fn label_variants_need_an_appkit_theme_with_that_experiment() {
        use crate::theme::FieldLabels;
        let holds = |class: &str, theme: &Theme| {
            ClassList::parse(&format!("{class}:hidden"), theme)
                .resolve(State::default())
                .display
                == Display::None
        };
        // `near` is the default of every AppKit theme, strict mix too.
        let strict = Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light);
        assert_eq!(strict.field_labels, FieldLabels::Near);
        assert_eq!(FieldLabels::default(), FieldLabels::Near);
        assert!(holds("label-near", &strict));
        let appkit = strict.with_field_labels(FieldLabels::AppKit);
        let kai = Theme::kai(Density::Comfortable);
        for (class, labels) in [
            ("label-top", FieldLabels::Top),
            ("label-near", FieldLabels::Near),
            ("label-tab", FieldLabels::Tab),
        ] {
            assert!(!holds(class, &appkit), "{class}");
            assert!(holds(class, &appkit.with_field_labels(labels)), "{class}");
            assert!(!holds(class, &kai.with_field_labels(labels)), "{class}");
            for other in [FieldLabels::Top, FieldLabels::Near, FieldLabels::Tab] {
                if other != labels {
                    assert!(!holds(class, &appkit.with_field_labels(other)), "{class}");
                }
            }
        }
    }

    #[test]
    fn unknown_classes_and_variants_are_reported() {
        let list = ClassList::parse(
            "p-4 bg-nope xs:p-2 lol border-3 group",
            &Theme::kai(Density::Comfortable),
        );
        assert_eq!(list.unknown, vec!["bg-nope", "xs:p-2", "lol", "border-3"]);
        assert!(list.group);
    }
}
