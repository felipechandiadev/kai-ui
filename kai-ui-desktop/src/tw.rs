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
//! active, disabled. `hover:` and `active:` do not apply to disabled
//! elements.

use cosmic_text::Weight;

use crate::render::Color;
use crate::style::{
    Align, Cursor, Display, Edges, FlexDirection, Justify, Length, LineHeight, Overflow, Position,
    Shadow, Style,
};
use crate::theme::{REM, Theme};

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Base,
    GroupHover,
    GroupFocusWithin,
    Hover,
    FocusWithin,
    Focus,
    Active,
    Disabled,
}

const VARIANT_ORDER: [Variant; 8] = [
    Variant::Base,
    Variant::GroupHover,
    Variant::GroupFocusWithin,
    Variant::Hover,
    Variant::FocusWithin,
    Variant::Focus,
    Variant::Active,
    Variant::Disabled,
];

impl Variant {
    fn parse(prefix: &str) -> Option<Self> {
        Some(match prefix {
            "hover" => Self::Hover,
            "focus" | "focus-visible" => Self::Focus,
            "focus-within" => Self::FocusWithin,
            "active" => Self::Active,
            "disabled" => Self::Disabled,
            "group-hover" => Self::GroupHover,
            "group-focus-within" => Self::GroupFocusWithin,
            _ => return None,
        })
    }

    fn applies(self, state: State) -> bool {
        match self {
            Self::Base => true,
            Self::GroupHover => state.group_hover,
            Self::GroupFocusWithin => state.group_focus_within,
            Self::Hover => state.hover && !state.disabled,
            Self::FocusWithin => state.focus_within,
            Self::Focus => state.focus,
            Self::Active => state.active && !state.disabled,
            Self::Disabled => state.disabled,
        }
    }
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
    Overflow(Overflow),
    Position(Position),
    Z(i32),
    GridColumns(u16),
    Background(Color),
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
    Placeholder(Color),
    TextColor(Color),
    FontSize(f32, Option<f32>),
    Leading(LineHeight),
    Weight(Weight),
    NoWrap(bool),
    Stroke(f32),
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
            if class == "group" {
                list.group = true;
                continue;
            }
            let (variant, utility) = match class.split_once(':') {
                Some((prefix, utility)) => match Variant::parse(prefix) {
                    Some(variant) => (variant, utility),
                    None => {
                        list.unknown.push(class.to_owned());
                        continue;
                    }
                },
                None => (Variant::Base, class),
            };
            let parser = Parser { theme };
            match parser.utility(utility) {
                Some(decls) => list
                    .decls
                    .extend(decls.into_iter().map(|decl| (variant, decl))),
                None => list.unknown.push(class.to_owned()),
            }
        }
        list
    }

    pub fn resolve(&self, state: State) -> Style {
        let mut style = Style::default();
        for variant in VARIANT_ORDER {
            if !variant.applies(state) {
                continue;
            }
            let decls = self.decls.iter().filter(|(v, _)| *v == variant);
            for (_, decl) in decls.clone() {
                if !matches!(decl, Decl::Leading(_)) {
                    apply(&mut style, *decl);
                }
            }
            for (_, decl) in decls {
                if matches!(decl, Decl::Leading(_)) {
                    apply(&mut style, *decl);
                }
            }
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

fn apply(style: &mut Style, decl: Decl) {
    match decl {
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
        Decl::Overflow(overflow) => style.overflow = overflow,
        Decl::Position(position) => style.position = position,
        Decl::Z(z) => style.z_index = z,
        Decl::GridColumns(columns) => style.grid_columns = columns,
        Decl::Background(color) => style.background = Some(color),
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
        if class == "truncate" {
            return Some(vec![Decl::NoWrap(true), Decl::Overflow(Overflow::Hidden)]);
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
            "flex" | "grow" | "shrink" => return None,
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
            "overflow-visible" => Decl::Overflow(Overflow::Visible),
            "overflow-hidden" | "overflow-clip" => Decl::Overflow(Overflow::Hidden),
            "overflow-auto" | "overflow-y-auto" | "overflow-scroll" | "overflow-y-scroll" => {
                Decl::Overflow(Overflow::Auto)
            }
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
            "pointer-events-none" => Decl::PointerEvents(false),
            "pointer-events-auto" => Decl::PointerEvents(true),
            "whitespace-nowrap" => Decl::NoWrap(true),
            "whitespace-normal" => Decl::NoWrap(false),
            "font-normal" => Decl::Weight(t.weight_regular),
            "font-medium" => Decl::Weight(t.weight_medium),
            "font-semibold" => Decl::Weight(t.weight_semibold),
            "font-bold" => Decl::Weight(t.weight_bold),
            "h-control" => Decl::Height(Length::Px(controls.height_md)),
            "h-control-compact" => Decl::Height(Length::Px(controls.height_compact)),
            "min-h-control" => Decl::MinHeight(Length::Px(controls.height_md)),
            "py-control" => Decl::Padding(Sides::Y, controls.padding_y_md),
            _ => return None,
        })
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
    use crate::render::Color;
    use crate::style::{
        Align, Cursor, Display, Edges, FlexDirection, Justify, Length, LineHeight, Overflow,
        Position,
    };
    use crate::theme::{Density, Theme};

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
        assert_eq!(style.overflow, Overflow::Auto);
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
    fn unknown_classes_and_variants_are_reported() {
        let list = ClassList::parse(
            "p-4 bg-nope sm:p-2 lol border-3 group",
            &Theme::kai(Density::Comfortable),
        );
        assert_eq!(list.unknown, vec!["bg-nope", "sm:p-2", "lol", "border-3"]);
        assert!(list.group);
    }
}
