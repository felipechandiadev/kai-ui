//! CSS-like computed style of an element.
//!
//! [`Style`] is what the Tailwind parser (`tw`) produces and what layout and
//! paint consume. Box properties follow CSS semantics with `box-sizing:
//! border-box` (Tailwind's preflight). Text properties are optional and
//! inherited from the parent when unset, like CSS inherited properties.

use cosmic_text::Weight;

use crate::render::Color;

/// A CSS length. Percentages are fractions (`0.5` = 50%).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    pub const fn all(value: T) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

impl Edges<f32> {
    pub fn is_uniform(&self) -> bool {
        self.top == self.right && self.top == self.bottom && self.top == self.left
    }

    pub fn is_zero(&self) -> bool {
        self.is_uniform() && self.top == 0.0
    }
}

/// How an image fills its box (CSS `object-fit`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ObjectFit {
    /// Stretched to the box (`object-fill`, the CSS default).
    #[default]
    Fill,
    /// Covers the box, cropped around its center (`object-cover`).
    Cover,
    /// Whole, centered, as large as fits (`object-contain`).
    Contain,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Display {
    #[default]
    Block,
    Flex,
    Grid,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
    Baseline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}

/// `overflow` on both axes. `Auto` scrolls vertically.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Auto,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Default,
    Pointer,
    Text,
    NotAllowed,
    /// `cursor-wait`: over a busy control.
    Wait,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    /// Multiple of the font size (`leading-tight` = 1.25).
    Relative(f32),
    Px(f32),
}

/// `box-shadow` with zero spread; `blur` is the CSS blur radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}

/// Tailwind `ring`: a solid band outside the border box, after `offset`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ring {
    pub width: f32,
    pub offset: f32,
    pub color: Option<Color>,
}

/// Inherited text properties; `None` inherits.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextProps {
    pub color: Option<Color>,
    pub font_size: Option<f32>,
    pub line_height: Option<LineHeight>,
    pub weight: Option<Weight>,
    pub nowrap: Option<bool>,
    /// Icon stroke width (`stroke-*`), in units of the 24×24 view box.
    pub stroke_width: Option<f32>,
    /// Letter spacing in em (`tracking-*`, macOS text styles).
    pub letter_spacing: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub display: Display,
    pub flex_direction: FlexDirection,
    pub flex_wrap: bool,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Length,
    pub align_items: Option<Align>,
    pub align_self: Option<Align>,
    pub justify_content: Option<Justify>,
    /// `inline-*` display: shrinks to content inside flex/grid parents.
    pub inline: bool,
    pub gap_x: f32,
    pub gap_y: f32,
    pub grid_columns: u16,
    pub padding: Edges<f32>,
    pub margin: Edges<Length>,
    pub border_width: Edges<f32>,
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub min_height: Length,
    pub max_width: Length,
    pub max_height: Length,
    /// Width ÷ height (`aspect-video`), used when one side is not set.
    pub aspect_ratio: Option<f32>,
    pub overflow: Overflow,
    pub position: Position,
    pub inset: Edges<Length>,
    pub z_index: i32,

    pub background: Option<Color>,
    /// `bg-current`: the background is the text color (CSS
    /// `currentColor`); layout fills `background` once it knows it.
    pub background_current: bool,
    pub border_color: Option<Color>,
    pub radius: f32,
    pub shadow: Option<Shadow>,
    pub ring: Ring,
    pub opacity: f32,
    pub cursor: Option<Cursor>,
    pub pointer_events: bool,
    pub placeholder_color: Option<Color>,
    pub object_fit: ObjectFit,

    pub text: TextProps,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            display: Display::Block,
            flex_direction: FlexDirection::Row,
            flex_wrap: false,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Length::Auto,
            align_items: None,
            align_self: None,
            justify_content: None,
            inline: false,
            gap_x: 0.0,
            gap_y: 0.0,
            grid_columns: 0,
            padding: Edges::all(0.0),
            margin: Edges::all(Length::Px(0.0)),
            border_width: Edges::all(0.0),
            width: Length::Auto,
            height: Length::Auto,
            min_width: Length::Auto,
            min_height: Length::Auto,
            max_width: Length::Auto,
            max_height: Length::Auto,
            aspect_ratio: None,
            overflow: Overflow::Visible,
            position: Position::Relative,
            inset: Edges::all(Length::Auto),
            z_index: 0,
            background: None,
            background_current: false,
            border_color: None,
            radius: 0.0,
            shadow: None,
            ring: Ring::default(),
            opacity: 1.0,
            cursor: None,
            pointer_events: true,
            placeholder_color: None,
            object_fit: ObjectFit::Fill,
            text: TextProps::default(),
        }
    }
}

/// Text properties after inheritance, in logical points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComputedText {
    pub color: Color,
    pub font_size: f32,
    pub line_height: f32,
    pub weight: Weight,
    pub nowrap: bool,
    pub stroke_width: f32,
    /// Letter spacing in em.
    pub letter_spacing: f32,
}

impl ComputedText {
    /// Apply an element's own text properties over the inherited ones.
    pub fn inherit(self, own: &TextProps) -> Self {
        let font_size = own.font_size.unwrap_or(self.font_size);
        let line_height = match own.line_height {
            Some(LineHeight::Px(px)) => px,
            Some(LineHeight::Relative(factor)) => (font_size * factor).round(),
            None if own.font_size.is_some() => (font_size * 1.5).round(),
            None => self.line_height,
        };
        Self {
            color: own.color.unwrap_or(self.color),
            font_size,
            line_height,
            weight: own.weight.unwrap_or(self.weight),
            nowrap: own.nowrap.unwrap_or(self.nowrap),
            stroke_width: own.stroke_width.unwrap_or(self.stroke_width),
            letter_spacing: own.letter_spacing.unwrap_or(self.letter_spacing),
        }
    }
}

impl Style {
    /// Translate the box model to Taffy.
    pub fn to_taffy(&self) -> taffy::Style {
        use taffy::prelude::{
            Dimension, LengthPercentage, LengthPercentageAuto, Rect, Size, TaffyAuto, auto, length,
            percent,
        };

        let dimension = |value: Length| -> Dimension {
            match value {
                Length::Auto => auto(),
                Length::Px(px) => length(px),
                Length::Percent(fraction) => percent(fraction),
            }
        };
        let lpa = |value: Length| -> LengthPercentageAuto {
            match value {
                Length::Auto => LengthPercentageAuto::AUTO,
                Length::Px(px) => length(px),
                Length::Percent(fraction) => percent(fraction),
            }
        };
        let lp = |value: f32| -> LengthPercentage { length(value) };

        let mut style = taffy::Style {
            display: match self.display {
                Display::Block => taffy::Display::Block,
                Display::Flex => taffy::Display::Flex,
                Display::Grid => taffy::Display::Grid,
                Display::None => taffy::Display::None,
            },
            position: match self.position {
                Position::Relative => taffy::Position::Relative,
                Position::Absolute => taffy::Position::Absolute,
            },
            inset: Rect {
                left: lpa(self.inset.left),
                right: lpa(self.inset.right),
                top: lpa(self.inset.top),
                bottom: lpa(self.inset.bottom),
            },
            size: Size {
                width: dimension(self.width),
                height: dimension(self.height),
            },
            min_size: Size {
                width: lpa(self.min_width),
                height: lpa(self.min_height),
            },
            max_size: Size {
                width: lpa(self.max_width),
                height: lpa(self.max_height),
            },
            aspect_ratio: self.aspect_ratio,
            margin: Rect {
                left: lpa(self.margin.left),
                right: lpa(self.margin.right),
                top: lpa(self.margin.top),
                bottom: lpa(self.margin.bottom),
            },
            padding: Rect {
                left: lp(self.padding.left),
                right: lp(self.padding.right),
                top: lp(self.padding.top),
                bottom: lp(self.padding.bottom),
            },
            border: Rect {
                left: lp(self.border_width.left),
                right: lp(self.border_width.right),
                top: lp(self.border_width.top),
                bottom: lp(self.border_width.bottom),
            },
            gap: Size {
                width: lp(self.gap_x),
                height: lp(self.gap_y),
            },
            flex_direction: match self.flex_direction {
                FlexDirection::Row => taffy::FlexDirection::Row,
                FlexDirection::Column => taffy::FlexDirection::Column,
            },
            flex_wrap: if self.flex_wrap {
                taffy::FlexWrap::Wrap
            } else {
                taffy::FlexWrap::NoWrap
            },
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: dimension(self.flex_basis),
            align_items: self
                .align_items
                .map(align_items)
                .unwrap_or(taffy::AlignItems::NORMAL),
            align_self: self
                .align_self
                .or((self.inline && self.width == Length::Auto).then_some(Align::Start))
                .map(align_items),
            justify_content: self
                .justify_content
                .map(justify)
                .unwrap_or(taffy::JustifyContent::NORMAL),
            ..taffy::Style::default()
        };
        let overflow = match self.overflow {
            Overflow::Visible => taffy::Overflow::Visible,
            Overflow::Hidden => taffy::Overflow::Hidden,
            Overflow::Auto => taffy::Overflow::Scroll,
        };
        style.overflow = taffy::Point {
            x: overflow,
            y: overflow,
        };
        style.scrollbar_width = 0.0;
        if self.display == Display::Grid && self.grid_columns > 0 {
            style.grid_template_columns = vec![taffy::prelude::repeat(
                self.grid_columns,
                vec![taffy::prelude::flex(1.0)],
            )];
        }
        style
    }
}

fn align_items(value: Align) -> taffy::AlignItems {
    match value {
        Align::Start => taffy::AlignItems::START,
        Align::Center => taffy::AlignItems::CENTER,
        Align::End => taffy::AlignItems::END,
        Align::Stretch => taffy::AlignItems::STRETCH,
        Align::Baseline => taffy::AlignItems::BASELINE,
    }
}

fn justify(value: Justify) -> taffy::JustifyContent {
    match value {
        Justify::Start => taffy::JustifyContent::START,
        Justify::Center => taffy::JustifyContent::CENTER,
        Justify::End => taffy::JustifyContent::END,
        Justify::Between => taffy::JustifyContent::SPACE_BETWEEN,
        Justify::Around => taffy::JustifyContent::SPACE_AROUND,
        Justify::Evenly => taffy::JustifyContent::SPACE_EVENLY,
    }
}

#[cfg(test)]
mod tests {
    use cosmic_text::Weight;

    use super::{Align, ComputedText, Display, Edges, LineHeight, Overflow, Style, TextProps};
    use crate::render::Color;

    const BASE: ComputedText = ComputedText {
        color: Color::rgb(0, 0, 0),
        font_size: 16.0,
        line_height: 24.0,
        weight: Weight(400),
        nowrap: false,
        stroke_width: 2.0,
        letter_spacing: 0.0,
    };

    #[test]
    fn text_inherits_unset_properties_and_derives_line_height() {
        let sized = BASE.inherit(&TextProps {
            font_size: Some(14.0),
            ..TextProps::default()
        });
        assert_eq!(sized.line_height, 21.0);
        assert_eq!(sized.color, BASE.color);

        let leading = BASE.inherit(&TextProps {
            font_size: Some(14.0),
            line_height: Some(LineHeight::Px(20.0)),
            weight: Some(Weight(500)),
            ..TextProps::default()
        });
        assert_eq!((leading.line_height, leading.weight), (20.0, Weight(500)));
        assert_eq!(BASE.inherit(&TextProps::default()), BASE);
    }

    #[test]
    fn style_maps_to_taffy_box_model() {
        let style = Style {
            display: Display::Flex,
            padding: Edges::all(8.0),
            border_width: Edges::all(1.0),
            overflow: Overflow::Auto,
            inline: true,
            ..Style::default()
        };
        let taffy_style = style.to_taffy();
        assert_eq!(taffy_style.display, taffy::Display::Flex);
        assert_eq!(taffy_style.overflow.y, taffy::Overflow::Scroll);
        assert_eq!(taffy_style.scrollbar_width, 0.0);
        assert_eq!(taffy_style.align_self, Some(taffy::AlignItems::START));
        assert_eq!(
            Style {
                align_self: Some(Align::Center),
                ..style
            }
            .to_taffy()
            .align_self,
            Some(taffy::AlignItems::CENTER)
        );
    }
}
