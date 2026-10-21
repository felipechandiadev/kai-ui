//! Layout of the element tree over Taffy, in logical window coordinates.
//!
//! The root element is the window: it always gets exactly the viewport size,
//! whatever its classes say, and everything else lives inside it. Each
//! element's classes are resolved to a [`Style`] for its interaction state
//! (hover, focus, active, disabled, group) and translated to Taffy, so
//! block, flex and grid behave as in CSS. Elements with `overflow-hidden` or
//! `overflow-auto` clip their descendants to their padding box;
//! `overflow-y-auto` and `overflow-x-auto` elements (`overflow-auto` is both)
//! also scroll along that axis with their own offset ([`ScrollOffsets`]).
//!
//! Every call lays the whole tree out from scratch; there is no cache or
//! incremental invalidation besides the parsed-class cache ([`StyleCache`]).

use std::collections::HashMap;

use taffy::prelude::{AvailableSpace, NodeId, Size as TaffySize, TaffyTree};

use crate::element::{
    ControlSize, ControlState, Element, KeyEquivalent, Kind, Motion, Role, Rows, WidgetId,
};
use crate::geo::Camera;
use crate::geometry::{Insets, Point, Rect, Size};
use crate::icons::Icon;
use crate::maps::{MapSpec, Marker};
use crate::media::LoadedImage;
use crate::render::TextId;
use crate::render::text::TextStyle;
use crate::style::{ComputedText, Display, Length, Overflow, Style};
use crate::theme::Theme;
use crate::tw::{ClassList, State};

/// Default width of an input without an explicit width (`min-width: 180px`
/// in the web `textfield.css`).
const INPUT_INTRINSIC_WIDTH: f32 = 180.0;

pub trait TextMeasure {
    /// Size of `text` laid out at `max_width` (`None` = unconstrained),
    /// keeping the shaped buffer under `id` ready to draw.
    fn measure(&mut self, id: TextId, text: &str, style: TextStyle, max_width: Option<f32>)
    -> Size;
}

/// `NSWindow.isKeyWindow` / `isMainWindow`. With a single window, key ==
/// main == the app is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowActivity {
    pub key: bool,
    pub main: bool,
}

impl Default for WindowActivity {
    /// Active: tests and captures render a focused window.
    fn default() -> Self {
        Self {
            key: true,
            main: true,
        }
    }
}

impl WindowActivity {
    pub const ACTIVE: Self = Self {
        key: true,
        main: true,
    };
    pub const INACTIVE: Self = Self {
        key: false,
        main: false,
    };

    /// AppKit dims accents only when the window is neither key nor main.
    pub fn is_inactive(self) -> bool {
        !self.key && !self.main
    }
}

/// Pointer, keyboard and window state that selects class variants.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Interaction {
    /// Hit-test path under the pointer, root first.
    pub hovered: Vec<WidgetId>,
    pub focused: Option<WidgetId>,
    pub pressed: Option<WidgetId>,
    pub window: WindowActivity,
    /// The focus came from the keyboard or assistive technology
    /// (`UiModel::focus_visible`).
    pub focus_visible: bool,
    /// Files are being dragged over the window: drop zones
    /// ([`Element::on_drop_files`]) take their `drag:` classes.
    pub file_drag: bool,
}

/// Scroll offset of each scrolling element, horizontal (`x`) and vertical.
pub type ScrollOffsets = HashMap<WidgetId, Point>;

/// A scroll direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

/// Parsed class strings, reused across frames. Empties itself when called
/// with a theme different from the one it parsed with.
#[derive(Default)]
pub struct StyleCache {
    lists: HashMap<String, ClassList>,
    /// Theme the cached lists were parsed with. Colors and `kai:`/`appkit:`/
    /// `dark:`/`light:` variants are resolved at parse time, so a different
    /// theme empties the cache.
    theme: Option<Theme>,
}

impl StyleCache {
    pub fn get(&mut self, classes: &str, theme: &Theme) -> &ClassList {
        if self.theme.as_ref() != Some(theme) {
            self.lists.clear();
            self.theme = Some(*theme);
        }
        if !self.lists.contains_key(classes) {
            self.lists
                .insert(classes.to_owned(), ClassList::parse(classes, theme));
        }
        &self.lists[classes]
    }

    pub fn clear(&mut self) {
        self.lists.clear();
    }

    /// Unknown classes seen so far, for diagnostics.
    pub fn unknown(&self) -> Vec<&str> {
        let mut unknown: Vec<&str> = self
            .lists
            .values()
            .flat_map(|list| list.unknown.iter().map(String::as_str))
            .collect();
        unknown.sort_unstable();
        unknown.dedup();
        unknown
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Box,
    Text(String),
    Icon(Icon),
    Input {
        placeholder: String,
        masked: bool,
        read_only: bool,
        /// Multiline inputs, with their value: they are as tall as its
        /// wrapped lines, within the rows.
        lines: Option<(Rows, String)>,
    },
    /// The decoded image, once the session has it.
    Image(Option<LoadedImage>),
    /// A map with its camera's view.
    Map(Box<MapSpec>),
}

/// Where a scrolling element is: vertical `offset` within `0..=max`, and
/// horizontal `offset_x` within `0..=max_x` (both 0 on an axis it does not
/// scroll).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollState {
    pub offset: f32,
    pub max: f32,
    pub offset_x: f32,
    pub max_x: f32,
}

impl ScrollState {
    pub fn offset_on(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.offset_x,
            Axis::Y => self.offset,
        }
    }

    pub fn max_on(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.max_x,
            Axis::Y => self.max,
        }
    }

    /// The offsets, as kept in [`ScrollOffsets`].
    pub fn offsets(&self) -> Point {
        Point {
            x: self.offset_x,
            y: self.offset,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutNode {
    pub id: WidgetId,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub content: Content,
    pub role: Option<Role>,
    pub label: Option<String>,
    /// See [`Element::test_id`].
    pub test_id: Option<String>,
    pub description: Option<String>,
    pub disabled: bool,
    /// Effective control size (own or inherited).
    pub control_size: ControlSize,
    pub control_state: Option<ControlState>,
    pub is_default: bool,
    pub key_equivalent: Option<KeyEquivalent>,
    pub required: bool,
    /// Input focused by a click on this node (see
    /// [`Element::delegates_focus`]).
    pub focus_delegate: Option<WidgetId>,
    /// Clickable or editable (see [`Element::is_control`]).
    pub control: bool,
    /// Pop-up button: whether its list is open (see [`Element::options`]).
    pub expanded: Option<bool>,
    /// See [`Element::context_menu`].
    pub context_menu: bool,
    /// See [`Element::row_count`].
    pub row_count: Option<usize>,
    /// See [`Element::row_index`].
    pub row_index: Option<usize>,
    pub style: Style,
    pub text: ComputedText,
    /// Product of the opacities of this node and its ancestors.
    pub opacity: f32,
    /// Hidden by `display: none` on itself or an ancestor.
    pub hidden: bool,
    pub pointer_events: bool,
    /// Border box, window coordinates, ancestor scroll applied.
    pub bounds: Rect,
    pub padding_box: Rect,
    pub content_box: Rect,
    /// Clip inherited from overflow ancestors; `None` = the whole window.
    pub clip: Option<Rect>,
    pub scroll: Option<ScrollState>,
    /// Text area: the text box, or the vertically centered input line.
    pub text_box: Option<Rect>,
    /// Placed on a coordinate of the enclosing map.
    pub marker: Option<Marker>,
    /// See [`Element::motion`]; the painter moves it, layout does not.
    pub motion: Option<Motion>,
    /// See [`Element::busy`].
    pub busy: bool,
}

impl LayoutNode {
    /// Visible part of the border box, or `None` when fully clipped.
    pub fn visible_bounds(&self) -> Option<Rect> {
        match self.clip {
            None => Some(self.bounds),
            Some(clip) => self.bounds.intersection(clip),
        }
    }

    fn accepts_point(&self, point: Point) -> bool {
        !self.hidden
            && self.pointer_events
            && self.bounds.contains(point)
            && self.clip.is_none_or(|clip| clip.contains(point))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub viewport: Size,
    /// Pre-order (document order).
    nodes: Vec<LayoutNode>,
    index: HashMap<WidgetId, usize>,
    /// Back-to-front: document order with siblings stably sorted by z-index.
    paint_order: Vec<usize>,
}

pub struct LayoutInput<'a> {
    pub viewport: Size,
    pub theme: &'a Theme,
    pub interaction: &'a Interaction,
    pub scroll: &'a ScrollOffsets,
}

#[derive(Clone, Copy, Debug)]
enum Leaf {
    Text(usize),
    Icon(usize),
    Input(usize),
    Image(usize),
}

pub fn text_style(text: &ComputedText) -> TextStyle {
    TextStyle {
        font_size: text.font_size,
        line_height: text.line_height,
        weight: text.weight,
        wrap: !text.nowrap,
        letter_spacing: text.letter_spacing,
    }
}

struct Builder<'a> {
    theme: &'a Theme,
    interaction: &'a Interaction,
    focus_path: Vec<WidgetId>,
    styles: &'a mut StyleCache,
    /// For the `sm:` … `xl:` breakpoints.
    viewport_width: u32,
    taffy: TaffyTree<Leaf>,
    nodes: Vec<LayoutNode>,
    handles: Vec<Option<NodeId>>,
}

#[derive(Clone, Copy)]
struct Inherited {
    text: ComputedText,
    opacity: f32,
    hidden: bool,
    pointer_events: bool,
    group_hover: bool,
    group_focus_within: bool,
    /// The nearest `group` ancestor is busy (`group-busy:`).
    group_busy: bool,
    control_size: ControlSize,
    /// The parent's explicit `items-*`: auto-width inline children
    /// (`inline-flex` buttons…) follow it instead of aligning to the start.
    parent_align_items: Option<crate::style::Align>,
}

impl Builder<'_> {
    fn build<Msg>(
        &mut self,
        element: &Element<Msg>,
        parent: Option<usize>,
        inherited: Inherited,
    ) -> taffy::TaffyResult<NodeId> {
        let id = element.id;
        let interaction = self.interaction;
        let control_size = element.control.size.unwrap_or(inherited.control_size);
        let window_inactive = interaction.window.is_inactive();
        // Hover is a theme option (on by default, also in the mixed AppKit
        // themes). AppKit does no rollover in inactive windows; browsers
        // (and the Kai theme) keep hover. Disabled controls skip `hover:`
        // in `tw`.
        let hover = interaction.hovered.contains(&id)
            && self.theme.hover_effects
            && !(window_inactive && self.theme.is_appkit());
        let focus_within = self.focus_path.contains(&id);
        let focus = interaction.focused == Some(id);
        let is_text = matches!(element.kind, Kind::Input { .. });
        let state = State {
            hover,
            focus,
            focus_within,
            active: interaction.pressed == Some(id),
            disabled: element.disabled,
            group_hover: inherited.group_hover,
            group_focus_within: inherited.group_focus_within,
            busy: element.busy,
            group_busy: inherited.group_busy,
            // Browser `:focus-visible`: a click focuses a text field
            // visibly, but not a button.
            focus_visible: focus && (is_text || interaction.focus_visible),
            keyboard_focus: focus
                && (is_text
                    || (interaction.focus_visible && self.theme.prefs.full_keyboard_access)),
            window_inactive,
            selected: element.control.state == Some(ControlState::On),
            mixed: element.control.state == Some(ControlState::Mixed),
            control_size,
            viewport_width: self.viewport_width,
            file_drag: interaction.file_drag && element.handlers.on_drop_files.is_some(),
        };
        let classes = self.styles.get(&element.class, self.theme);
        let is_group = classes.group;
        let mut style = classes.resolve(state);
        let mut text = inherited.text.inherit(&style.text);
        if style.background_current {
            style.background = Some(text.color);
        }
        let content = match &element.kind {
            Kind::Div => Content::Box,
            Kind::Text(content) => Content::Text(content.clone()),
            Kind::Icon(icon) => Content::Icon(*icon),
            Kind::Input {
                value,
                placeholder,
                masked,
                read_only,
                rows,
                ..
            } => {
                text.nowrap = rows.is_none();
                Content::Input {
                    placeholder: placeholder.clone(),
                    masked: *masked,
                    read_only: *read_only,
                    lines: rows.map(|rows| (rows, value.clone())),
                }
            }
            Kind::Image { loaded, .. } => Content::Image(loaded.clone()),
            Kind::Map(spec) => Content::Map(spec.clone()),
        };
        let index = self.nodes.len();
        let node = LayoutNode {
            id,
            parent,
            children: Vec::new(),
            content,
            role: element.role,
            label: element.label.clone(),
            test_id: element.test_id.clone(),
            description: element.description.clone(),
            disabled: element.disabled,
            control_size,
            control_state: element.control.state,
            is_default: element.control.is_default,
            key_equivalent: element.control.key_equivalent,
            required: element.required,
            focus_delegate: element
                .delegates_focus
                .then(|| element.first_input())
                .flatten(),
            control: element.is_control(),
            expanded: element.expanded.or_else(|| {
                element
                    .options
                    .as_ref()
                    .filter(|options| !options.context)
                    .map(|options| options.open)
            }),
            context_menu: element.has_context_menu(),
            row_count: element.row_count,
            row_index: element.row_index,
            opacity: inherited.opacity * style.opacity,
            hidden: inherited.hidden || style.display == Display::None,
            pointer_events: inherited.pointer_events && style.pointer_events,
            style,
            text,
            bounds: Rect::default(),
            padding_box: Rect::default(),
            content_box: Rect::default(),
            clip: None,
            scroll: None,
            text_box: None,
            marker: element.marker,
            motion: element.motion,
            busy: element.busy,
        };
        let child_inherited = Inherited {
            text,
            opacity: node.opacity,
            hidden: node.hidden,
            pointer_events: node.pointer_events,
            group_hover: if is_group {
                hover
            } else {
                inherited.group_hover
            },
            group_focus_within: if is_group {
                focus_within
            } else {
                inherited.group_focus_within
            },
            group_busy: if is_group {
                element.busy
            } else {
                inherited.group_busy
            },
            control_size,
            parent_align_items: node.style.align_items,
        };
        let mut taffy_style = node.style.to_taffy();
        // `Style::to_taffy` aligns auto-width inline elements to the start
        // so they keep their content width; an explicit `items-*` on the
        // parent wins, as in CSS.
        if node.style.align_self.is_none() && inherited.parent_align_items.is_some() {
            taffy_style.align_self = None;
        }
        // A loaded image keeps its aspect ratio, and without a size of its
        // own takes its natural one (an `<img>` is not stretched).
        if let Content::Image(Some(image)) = &node.content {
            let (width, height) = (image.width.max(1) as f32, image.height.max(1) as f32);
            taffy_style.aspect_ratio.get_or_insert(width / height);
            if node.style.width == Length::Auto && node.style.height == Length::Auto {
                taffy_style.size = TaffySize {
                    width: taffy::style_helpers::length(width),
                    height: taffy::style_helpers::length(height),
                };
            }
        }
        self.nodes.push(node);
        self.handles.push(None);
        if let Some(parent) = parent {
            self.nodes[parent].children.push(index);
        }

        let handle = match element.kind {
            Kind::Div | Kind::Map(_) => {
                let mut children = Vec::with_capacity(element.children.len());
                for child in &element.children {
                    children.push(self.build(child, Some(index), child_inherited)?);
                }
                self.taffy.new_with_children(taffy_style, &children)?
            }
            Kind::Text(_) => self
                .taffy
                .new_leaf_with_context(taffy_style, Leaf::Text(index))?,
            Kind::Icon(_) => self
                .taffy
                .new_leaf_with_context(taffy_style, Leaf::Icon(index))?,
            Kind::Input { .. } => self
                .taffy
                .new_leaf_with_context(taffy_style, Leaf::Input(index))?,
            Kind::Image { .. } => self
                .taffy
                .new_leaf_with_context(taffy_style, Leaf::Image(index))?,
        };
        self.handles[index] = Some(handle);
        Ok(handle)
    }
}

fn measure_leaf(
    leaf: Option<Leaf>,
    known: TaffySize<Option<f32>>,
    available: TaffySize<AvailableSpace>,
    nodes: &[LayoutNode],
    text: &mut dyn TextMeasure,
) -> TaffySize<f32> {
    let measured = match leaf {
        None => Size::default(),
        Some(Leaf::Text(index)) => {
            let node = &nodes[index];
            let Content::Text(content) = &node.content else {
                return TaffySize::ZERO;
            };
            let max_width = if node.text.nowrap {
                None
            } else {
                known.width.or(match available.width {
                    AvailableSpace::Definite(width) => Some(width),
                    AvailableSpace::MinContent => Some(0.0),
                    AvailableSpace::MaxContent => None,
                })
            };
            text.measure(
                TextId::content(node.id),
                content,
                text_style(&node.text),
                max_width,
            )
        }
        Some(Leaf::Icon(index)) => {
            let side = nodes[index].text.font_size;
            Size {
                width: side,
                height: side,
            }
        }
        Some(Leaf::Input(index)) => {
            let node = &nodes[index];
            let available_width = match available.width {
                AvailableSpace::Definite(width) => width,
                _ => INPUT_INTRINSIC_WIDTH,
            };
            let width = INPUT_INTRINSIC_WIDTH.min(available_width);
            let line_height = node.text.line_height;
            let height = match &node.content {
                Content::Input {
                    lines: Some((rows, value)),
                    ..
                } => {
                    let wrapped = text.measure(
                        TextId::content(node.id),
                        value,
                        text_style(&node.text),
                        Some(known.width.unwrap_or(width)),
                    );
                    let count = (wrapped.height / line_height).round().max(1.0);
                    count.clamp(f32::from(rows.min), f32::from(rows.max)) * line_height
                }
                _ => line_height,
            };
            Size { width, height }
        }
        Some(Leaf::Image(index)) => image_size(&nodes[index].content, known),
    };
    // Rounded up so pixel-snapped layout never trims the last glyph.
    TaffySize {
        width: known.width.unwrap_or(measured.width.ceil()),
        height: known.height.unwrap_or(measured.height.ceil()),
    }
}

/// Natural size of an image in points (one pixel each), or the side its
/// aspect ratio gives when the other one is known; empty until loaded.
fn image_size(content: &Content, known: TaffySize<Option<f32>>) -> Size {
    let Content::Image(Some(image)) = content else {
        return Size::default();
    };
    let (width, height) = (image.width as f32, image.height as f32);
    match (known.width, known.height) {
        (Some(known_width), None) => Size {
            width: known_width,
            height: known_width * height / width,
        },
        (None, Some(known_height)) => Size {
            width: known_height * width / height,
            height: known_height,
        },
        _ => Size { width, height },
    }
}

fn clip_to(clip: Option<Rect>, rect: Rect) -> Option<Rect> {
    let empty = Rect {
        width: 0.0,
        height: 0.0,
        ..rect
    };
    Some(match clip {
        None => rect,
        Some(clip) => clip.intersection(rect).unwrap_or(empty),
    })
}

fn insets(rect: taffy::Rect<f32>) -> Insets {
    Insets {
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        left: rect.left,
    }
}

impl Layout {
    /// Lay out `root` (with ids assigned) for the given viewport.
    pub fn compute<Msg>(
        root: &Element<Msg>,
        input: &LayoutInput<'_>,
        styles: &mut StyleCache,
        text: &mut dyn TextMeasure,
    ) -> Self {
        let focus_path = input
            .interaction
            .focused
            .and_then(|id| root.path_to(id))
            .unwrap_or_default();
        let mut builder = Builder {
            theme: input.theme,
            interaction: input.interaction,
            focus_path,
            styles,
            viewport_width: input.viewport.width.max(0.0) as u32,
            taffy: TaffyTree::new(),
            nodes: Vec::new(),
            handles: Vec::new(),
        };
        let inherited = Inherited {
            text: input.theme.base_text(),
            opacity: 1.0,
            hidden: false,
            pointer_events: true,
            group_hover: false,
            group_focus_within: false,
            group_busy: false,
            control_size: input.theme.control_size,
            parent_align_items: None,
        };
        let root_handle = builder
            .build(root, None, inherited)
            .expect("element styles are valid Taffy input");
        let Builder {
            mut taffy,
            mut nodes,
            handles,
            ..
        } = builder;

        let viewport = input.viewport;
        let mut root_style = taffy.style(root_handle).cloned().unwrap_or_default();
        root_style.size = TaffySize {
            width: taffy::prelude::length(viewport.width),
            height: taffy::prelude::length(viewport.height),
        };
        root_style.position = taffy::Position::Relative;
        root_style.margin = taffy::Rect::zero();
        taffy
            .set_style(root_handle, root_style)
            .expect("root style is valid");
        if nodes[0].style.display == Display::None {
            nodes[0].hidden = false;
            nodes[0].style.display = Display::Block;
        }

        {
            let nodes_ref = &nodes;
            taffy
                .compute_layout_with_measure(
                    root_handle,
                    TaffySize {
                        width: AvailableSpace::Definite(viewport.width),
                        height: AvailableSpace::Definite(viewport.height),
                    },
                    |inputs, _node, context, style| {
                        let leaf = context.map(|leaf| *leaf);
                        taffy::compute_leaf_layout(
                            inputs,
                            style,
                            |_, _| 0.0,
                            |known, available| {
                                measure_leaf(leaf, known, available, nodes_ref, text)
                            },
                        )
                    },
                )
                .expect("Taffy layout of a valid tree");
        }

        place(
            &mut nodes,
            &handles,
            &taffy,
            0,
            Point::default(),
            None,
            input.scroll,
            text,
        );
        place_markers(&mut nodes);

        let mut paint_order = Vec::with_capacity(nodes.len());
        push_paint_order(&nodes, 0, &mut paint_order);
        let index = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id, index))
            .collect();
        Self {
            viewport,
            nodes,
            index,
            paint_order,
        }
    }

    pub fn nodes(&self) -> &[LayoutNode] {
        &self.nodes
    }

    pub fn root(&self) -> Option<&LayoutNode> {
        self.nodes.first()
    }

    pub fn node(&self, id: WidgetId) -> Option<&LayoutNode> {
        self.index.get(&id).map(|index| &self.nodes[*index])
    }

    pub fn get(&self, index: usize) -> &LayoutNode {
        &self.nodes[index]
    }

    /// Every visible map, with its padding box (the area it draws in).
    pub(crate) fn maps_mut(&mut self) -> impl Iterator<Item = (WidgetId, Rect, &mut MapSpec)> {
        self.nodes
            .iter_mut()
            .filter(|node| !node.hidden)
            .filter_map(|node| match &mut node.content {
                Content::Map(spec) => Some((node.id, node.padding_box, &mut **spec)),
                _ => None,
            })
    }

    pub fn bounds(&self, id: WidgetId) -> Option<Rect> {
        self.node(id).map(|node| node.bounds)
    }

    pub fn text_box(&self, id: WidgetId) -> Option<Rect> {
        self.node(id).and_then(|node| node.text_box)
    }

    /// Node indices back-to-front.
    pub fn paint_order(&self) -> &[usize] {
        &self.paint_order
    }

    /// Path (root first) to the topmost node under `point`, honoring
    /// clipping, z-order, `display: none` and `pointer-events-none`.
    pub fn hit_path(&self, point: Point) -> Vec<WidgetId> {
        let Some(mut index) = self
            .paint_order
            .iter()
            .rev()
            .copied()
            .find(|index| self.nodes[*index].accepts_point(point))
        else {
            return Vec::new();
        };
        let mut path = vec![self.nodes[index].id];
        while let Some(parent) = self.nodes[index].parent {
            path.push(self.nodes[parent].id);
            index = parent;
        }
        path.reverse();
        path
    }

    /// Ancestors of `id` (nearest first), excluding itself.
    pub fn ancestors(&self, id: WidgetId) -> impl Iterator<Item = &LayoutNode> {
        let mut parent = self
            .index
            .get(&id)
            .and_then(|index| self.nodes[*index].parent);
        std::iter::from_fn(move || {
            let node = &self.nodes[parent?];
            parent = node.parent;
            Some(node)
        })
    }

    /// `point` relative to the node's text box origin.
    pub fn text_local_position(&self, id: WidgetId, point: Point) -> Option<Point> {
        self.text_box(id).map(|text_box| Point {
            x: point.x - text_box.x,
            y: point.y - text_box.y,
        })
    }

    /// Adjust the offsets of the scroll containers around `id`, innermost
    /// first, so its border box plus `margin` becomes visible, across and
    /// down. Returns whether any offset changed.
    pub fn reveal(&self, id: WidgetId, margin: f32, offsets: &mut ScrollOffsets) -> bool {
        let Some(node) = self.node(id) else {
            return false;
        };
        let mut x = (
            node.bounds.x - margin,
            node.bounds.x + node.bounds.width + margin,
        );
        let mut y = (
            node.bounds.y - margin,
            node.bounds.y + node.bounds.height + margin,
        );
        let mut changed = false;
        for ancestor in self.ancestors(id) {
            let Some(scroll) = ancestor.scroll else {
                continue;
            };
            let view = ancestor.padding_box;
            let next = Point {
                x: reveal_span(&mut x, (view.x, view.width), scroll.offset_x, scroll.max_x),
                y: reveal_span(&mut y, (view.y, view.height), scroll.offset, scroll.max),
            };
            if next != scroll.offsets() {
                offsets.insert(ancestor.id, next);
                changed = true;
            }
        }
        changed
    }

    /// Innermost scroll container on `path` (root first) that can move by
    /// `delta` along `axis` (positive = toward the end of the content).
    pub fn scroll_target(
        &self,
        path: &[WidgetId],
        axis: Axis,
        delta: f32,
    ) -> Option<(WidgetId, ScrollState)> {
        path.iter().rev().find_map(|id| {
            let scroll = self.node(*id)?.scroll?;
            let offset = scroll.offset_on(axis);
            let can_move = if delta > 0.0 {
                offset < scroll.max_on(axis)
            } else {
                offset > 0.0
            };
            can_move.then_some((*id, scroll))
        })
    }
}

/// The offset along one axis that shows `span` (start, end) in a view
/// (start, length) scrolled by `offset` within `0..=max`, moving it the
/// least (a span that covers the view stays: a wide table row keeps the
/// horizontal scroll); `span` becomes its visible part afterwards, for the
/// next container out.
fn reveal_span(span: &mut (f32, f32), view: (f32, f32), offset: f32, max: f32) -> f32 {
    let (start, end) = *span;
    let (view_start, view_length) = view;
    let view_end = view_start + view_length;
    let delta = if start < view_start && end < view_end {
        start - view_start
    } else if end > view_end && start > view_start {
        (end - view_end).min(start - view_start)
    } else {
        0.0
    };
    let next = (offset + delta).clamp(0.0, max);
    let moved = next - offset;
    *span = ((start - moved).max(view_start), (end - moved).min(view_end));
    next
}

#[allow(clippy::too_many_arguments)]
fn place(
    nodes: &mut [LayoutNode],
    handles: &[Option<NodeId>],
    taffy: &TaffyTree<Leaf>,
    index: usize,
    parent_origin: Point,
    clip: Option<Rect>,
    offsets: &ScrollOffsets,
    text: &mut dyn TextMeasure,
) {
    let Some(layout) = handles[index].and_then(|handle| taffy.layout(handle).ok()) else {
        return;
    };
    let bounds = Rect {
        x: parent_origin.x + layout.location.x,
        y: parent_origin.y + layout.location.y,
        width: layout.size.width,
        height: layout.size.height,
    };
    let padding_box = bounds.inset(insets(layout.border));
    let content_box = padding_box.inset(insets(layout.padding));
    let node = &mut nodes[index];
    node.bounds = bounds;
    node.padding_box = padding_box;
    node.content_box = content_box;
    node.clip = clip;
    let (scrolls_x, scrolls_y) = (
        node.style.overflow_x == Overflow::Auto,
        node.style.overflow_y == Overflow::Auto,
    );
    node.scroll = (scrolls_x || scrolls_y).then(|| {
        let saved = offsets.get(&node.id).copied().unwrap_or_default();
        let max = if scrolls_y {
            layout.scroll_height()
        } else {
            0.0
        };
        let max_x = if scrolls_x {
            layout.scroll_width()
        } else {
            0.0
        };
        ScrollState {
            offset: saved.y.clamp(0.0, max),
            max,
            offset_x: saved.x.clamp(0.0, max_x),
            max_x,
        }
    });
    if !node.hidden {
        match &node.content {
            Content::Box | Content::Icon(_) | Content::Image(_) | Content::Map(_) => {}
            Content::Text(content) => {
                // Re-wrap at the final width so the buffer is ready to draw.
                if !node.text.nowrap {
                    text.measure(
                        TextId::content(node.id),
                        content,
                        text_style(&node.text),
                        Some(content_box.width),
                    );
                }
                node.text_box = Some(content_box);
            }
            Content::Input {
                placeholder, lines, ..
            } => {
                let line_height = node.text.line_height;
                text.measure(
                    TextId::placeholder(node.id),
                    placeholder,
                    text_style(&node.text),
                    lines.as_ref().map(|_| content_box.width),
                );
                // One line is centered; several fill the box from the top.
                node.text_box = Some(if lines.is_some() {
                    Rect {
                        width: content_box.width.max(0.0),
                        height: content_box.height.max(line_height),
                        ..content_box
                    }
                } else {
                    Rect {
                        x: content_box.x,
                        y: content_box.y + ((content_box.height - line_height) / 2.0).max(0.0),
                        width: content_box.width.max(0.0),
                        height: line_height,
                    }
                });
            }
        }
    }
    let child_clip = if node.style.overflow_x == Overflow::Visible
        && node.style.overflow_y == Overflow::Visible
    {
        clip
    } else {
        clip_to(clip, padding_box)
    };
    let scrolled = node
        .scroll
        .map(|scroll| scroll.offsets())
        .unwrap_or_default();
    let origin = Point {
        x: bounds.x - scrolled.x,
        y: bounds.y - scrolled.y,
    };
    let children = node.children.clone();
    for child in children {
        place(
            nodes, handles, taffy, child, origin, child_clip, offsets, text,
        );
    }
}

/// Move every marker (and its subtree) so its anchor point lands on the
/// projection of its coordinate in the nearest enclosing map.
fn place_markers(nodes: &mut [LayoutNode]) {
    for index in 0..nodes.len() {
        let Some(marker) = nodes[index].marker else {
            continue;
        };
        let mut parent = nodes[index].parent;
        let map = loop {
            let Some(at) = parent else {
                break None;
            };
            if let Content::Map(spec) = &nodes[at].content {
                break Some((spec.view, nodes[at].padding_box));
            }
            parent = nodes[at].parent;
        };
        let Some((view, area)) = map else {
            continue;
        };
        let camera = Camera::new(
            view,
            Size {
                width: area.width,
                height: area.height,
            },
        );
        let at = camera.to_local(marker.at);
        let bounds = nodes[index].bounds;
        let (anchor_x, anchor_y) = marker.anchor.offset(bounds.width, bounds.height);
        let delta = Point {
            x: area.x + at.x - anchor_x - bounds.x,
            y: area.y + at.y - anchor_y - bounds.y,
        };
        let clip = nodes[index].clip;
        shift_subtree(nodes, index, delta, clip);
    }
}

/// Translate a node and its descendants by `delta`; clips made inside the
/// subtree (different from `outer`) move with it.
fn shift_subtree(nodes: &mut [LayoutNode], index: usize, delta: Point, outer: Option<Rect>) {
    let node = &mut nodes[index];
    node.bounds = node.bounds.translated(delta);
    node.padding_box = node.padding_box.translated(delta);
    node.content_box = node.content_box.translated(delta);
    node.text_box = node.text_box.map(|rect| rect.translated(delta));
    if node.clip != outer {
        node.clip = node.clip.map(|rect| rect.translated(delta));
    }
    for child in node.children.clone() {
        shift_subtree(nodes, child, delta, outer);
    }
}

fn push_paint_order(nodes: &[LayoutNode], index: usize, order: &mut Vec<usize>) {
    let node = &nodes[index];
    if node.hidden {
        return;
    }
    order.push(index);
    let mut children = node.children.clone();
    children.sort_by_key(|child| nodes[*child].style.z_index);
    for child in children {
        push_paint_order(nodes, child, order);
    }
}

/// Test helpers, public so the demo binary, examples and apps can lay out
/// trees without fonts.
pub mod testing {
    use super::TextMeasure;
    use crate::geometry::Size;
    use crate::render::TextId;
    use crate::render::text::TextStyle;

    /// Deterministic measurement: every character is half the font size wide,
    /// wrapping breaks at any character and `\n` starts a line.
    #[derive(Default)]
    pub struct FixedMeasure;

    impl TextMeasure for FixedMeasure {
        fn measure(
            &mut self,
            _id: TextId,
            text: &str,
            style: TextStyle,
            max_width: Option<f32>,
        ) -> Size {
            let char_width = style.font_size * (0.5 + style.letter_spacing);
            let mut size = Size::default();
            for paragraph in text.split('\n') {
                let full = paragraph.chars().count() as f32 * char_width;
                let (width, lines) = match max_width.filter(|_| style.wrap) {
                    Some(max) if full > max => {
                        let per_line = (max / char_width).floor().max(1.0);
                        let lines = (paragraph.chars().count() as f32 / per_line).ceil();
                        (per_line * char_width, lines)
                    }
                    _ => (full, 1.0),
                };
                size.width = size.width.max(width);
                size.height += lines * style.line_height;
            }
            size
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::FixedMeasure;
    use super::{Axis, Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::element::sample::{self, Msg};
    use crate::element::{Element, WidgetId, div, text};
    use crate::geometry::{Point, Rect, Size};
    use crate::render::Color;
    use crate::theme::{AppKitGeneration, Density, Theme, ThemeKind};
    use crate::tw::State;

    #[test]
    fn keyboard_focus_requires_fka_for_buttons() {
        use crate::element::input;

        let probe = "ring-0 keyboard-focus:ring-3";
        let root = tree(div().children([
            div().class(probe).on_click(Msg::Clicked),
            input("", "Campo").class(probe),
        ]));
        let (button, field) = (root.children[0].id, root.children[1].id);
        let kai = Theme::kai(Density::Comfortable);
        let mut no_fka = kai;
        no_fka.prefs.full_keyboard_access = false;
        let keyboard_focus = |theme: &Theme, focused: WidgetId, focus_visible: bool| {
            let layout = Layout::compute(
                &root,
                &LayoutInput {
                    viewport: WINDOW,
                    theme,
                    interaction: &Interaction {
                        focused: Some(focused),
                        focus_visible,
                        ..Interaction::default()
                    },
                    scroll: &ScrollOffsets::new(),
                },
                &mut StyleCache::default(),
                &mut FixedMeasure,
            );
            layout.node(focused).unwrap().style.ring.width == 3.0
        };
        assert!(keyboard_focus(&kai, button, true));
        assert!(!keyboard_focus(&kai, button, false), "pointer focus");
        assert!(!keyboard_focus(&no_fka, button, true));
        assert!(keyboard_focus(&no_fka, field, true));
        assert!(keyboard_focus(&no_fka, field, false));
    }

    #[test]
    fn hover_is_on_by_default_off_by_option_inactive_window_or_disabled() {
        use super::WindowActivity;
        use crate::theme::Appearance;

        let root = tree(
            div()
                .child(div().class("bg-background hover:bg-hover"))
                .child(div().class("bg-background hover:bg-hover").disabled(true)),
        );
        let (card, disabled) = (root.children[0].id, root.children[1].id);
        let kai = Theme::kai(Density::Comfortable);
        let appkit = Theme {
            kind: ThemeKind::AppKit(AppKitGeneration::Aqua11),
            appearance: Appearance::Light,
            ..kai
        };
        let background_of = |node, theme: &Theme, window: WindowActivity| {
            Layout::compute(
                &root,
                &LayoutInput {
                    viewport: WINDOW,
                    theme,
                    interaction: &Interaction {
                        hovered: vec![root.id, card, disabled],
                        window,
                        ..Interaction::default()
                    },
                    scroll: &ScrollOffsets::new(),
                },
                &mut StyleCache::default(),
                &mut FixedMeasure,
            )
            .node(node)
            .unwrap()
            .style
            .background
        };
        let background = |theme: &Theme, window| background_of(card, theme, window);
        let (hover, base) = (Some(kai.colors.hover), Some(kai.colors.background));
        // Mixed AppKit theme: hover on by default, except in an inactive window.
        assert!(appkit.hover_effects && kai.hover_effects);
        assert_eq!(background(&appkit, WindowActivity::ACTIVE), hover);
        assert_eq!(background(&appkit, WindowActivity::INACTIVE), base);
        assert_eq!(
            background(&kai, WindowActivity::INACTIVE),
            hover,
            "web keeps hover"
        );
        // Strict AppKit (or a Kai screen without hover).
        for theme in [appkit.with_hover(false), kai.with_hover(false)] {
            assert_eq!(background(&theme, WindowActivity::ACTIVE), base);
        }
        // Disabled controls never hover.
        for theme in [&appkit, &kai] {
            assert_eq!(background_of(disabled, theme, WindowActivity::ACTIVE), base);
        }
    }

    #[test]
    fn window_activity_defaults_to_active() {
        use super::WindowActivity;
        assert_eq!(Interaction::default().window, WindowActivity::ACTIVE);
        assert!(!WindowActivity::default().is_inactive());
        assert!(WindowActivity::INACTIVE.is_inactive());
        // A key panel over an inactive main window is not dimmed.
        assert!(
            !WindowActivity {
                key: true,
                main: false
            }
            .is_inactive()
        );
    }

    #[test]
    fn style_cache_clears_on_theme_change() {
        let kai = Theme::kai(Density::Comfortable);
        let mut other = Theme {
            kind: ThemeKind::AppKit(AppKitGeneration::Aqua11),
            ..kai
        };
        other.semantic.label = Color::hex(0x123456);
        let mut styles = StyleCache::default();
        let background = |styles: &mut StyleCache, theme: &Theme, classes: &str| -> Option<Color> {
            styles
                .get(classes, theme)
                .resolve(State::default())
                .background
        };
        assert_eq!(
            background(&mut styles, &kai, "bg-label"),
            Some(kai.semantic.label)
        );
        assert_eq!(
            background(&mut styles, &other, "bg-label"),
            Some(Color::hex(0x123456))
        );
        let variants = "kai:bg-primary appkit:bg-error";
        assert_eq!(
            background(&mut styles, &other, variants),
            Some(kai.colors.error)
        );
        assert_eq!(
            background(&mut styles, &kai, variants),
            Some(kai.colors.primary)
        );
        assert!(styles.unknown().is_empty());
    }

    const WINDOW: Size = Size {
        width: 800.0,
        height: 600.0,
    };

    fn layout_with(
        root: &Element<Msg>,
        viewport: Size,
        interaction: &Interaction,
        scroll: &ScrollOffsets,
    ) -> Layout {
        let theme = Theme::kai(Density::Comfortable);
        Layout::compute(
            root,
            &LayoutInput {
                viewport,
                theme: &theme,
                interaction,
                scroll,
            },
            &mut StyleCache::default(),
            &mut FixedMeasure,
        )
    }

    fn layout(root: &Element<Msg>, viewport: Size) -> Layout {
        layout_with(
            root,
            viewport,
            &Interaction::default(),
            &ScrollOffsets::new(),
        )
    }

    fn tree(mut root: Element<Msg>) -> Element<Msg> {
        root.assign_ids();
        root
    }

    /// Auto-width inline children (buttons) keep their content width and
    /// sit at the start, unless the parent sets `items-*`: then they follow
    /// it like any flex item (`self-*` still wins).
    #[test]
    fn inline_children_follow_an_explicit_items_class() {
        let row = |items: &str, child: &str| {
            let root = tree(div().class("w-full h-full").child(
                div().class(&format!("flex {items} h-20 w-80")).children([
                    div().class("w-10 h-20"),
                    div().class(child).child(text("Ok")),
                ]),
            ));
            let layout = layout(&root, WINDOW);
            let row = layout.bounds(root.children[0].id).unwrap();
            let button = layout.bounds(root.children[0].children[1].id).unwrap();
            (button.y - row.y, button.height, button.width)
        };
        let inline = "inline-flex h-8 px-2";
        let (top, height, width) = row("", inline);
        assert_eq!((top, height), (0.0, 32.0));
        assert!(width < 100.0);
        assert_eq!(row("items-start", inline).0, 0.0);
        assert_eq!(row("items-center", inline).0, 24.0);
        assert_eq!(row("items-end", inline).0, 48.0);
        assert_eq!(row("items-end", &format!("{inline} self-start")).0, 0.0);
        // Stretch is honored too (no fixed height).
        assert_eq!(row("items-stretch", "inline-flex px-2").1, 80.0);
        // Columns: centered horizontally, still content-wide.
        let root = tree(
            div().class("w-full h-full").child(
                div()
                    .class("flex flex-col items-center w-80")
                    .child(div().class(inline).child(text("Ok"))),
            ),
        );
        let layout = layout(&root, WINDOW);
        let column = layout.bounds(root.children[0].id).unwrap();
        let button = layout.bounds(root.children[0].children[0].id).unwrap();
        assert!(button.width < 100.0);
        assert_eq!(
            button.x - column.x,
            ((column.width - button.width) / 2.0).round()
        );
    }

    #[test]
    fn root_always_fills_the_window_even_without_size_classes() {
        let root = tree(div().class("w-10 h-10 m-8").child(text("hola")));
        let layout = layout(&root, WINDOW);
        assert_eq!(
            layout.root().unwrap().bounds,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0
            }
        );
        let sample = sample::tree();
        let filled = layout_with(
            &sample,
            WINDOW,
            &Interaction::default(),
            &ScrollOffsets::new(),
        );
        assert_eq!(filled.root().unwrap().bounds.width, 800.0);
        assert_eq!(filled.root().unwrap().bounds.height, 600.0);
    }

    #[test]
    fn flex_column_places_children_with_padding_gap_and_inline_buttons() {
        let sample = sample::tree();
        let layout = layout(&sample, WINDOW);
        let heading = layout.bounds(sample::heading()).unwrap();
        assert_eq!((heading.x, heading.y), (16.0, 16.0));
        assert_eq!(heading.width, 800.0 - 32.0, "block children stretch");
        let description = layout.bounds(sample::description()).unwrap();
        assert_eq!(description.y, heading.y + heading.height + 16.0);

        let button = layout.bounds(sample::button()).unwrap();
        assert_eq!(button.height, 36.0, "h-control");
        let label = layout.bounds(sample::button_label()).unwrap();
        assert_eq!(
            button.width,
            label.width + 32.0,
            "inline-flex shrinks to px-4 + label"
        );
        let label_box = layout.text_box(sample::button_label()).unwrap();
        assert_eq!(label_box.y - button.y, (36.0 - label_box.height) / 2.0);

        let field = layout.node(sample::field()).unwrap();
        assert_eq!(field.bounds.width, 800.0 - 32.0, "w-full");
        let line = field.text_box.unwrap();
        assert_eq!(line.x, field.bounds.x + 1.0 + 12.0, "border + px-3");
        assert_eq!(line.height, 24.0);
        assert_eq!(line.y, field.bounds.y + (36.0 - 24.0) / 2.0);
    }

    #[test]
    fn overflow_auto_scrolls_children_and_clips_to_padding_box() {
        let sample = sample::tree();
        let short = Size {
            width: 420.0,
            height: 120.0,
        };
        let layout = layout(&sample, short);
        let root = layout.root().unwrap();
        let scroll = root.scroll.unwrap();
        assert!(scroll.max > 0.0);
        assert_eq!(scroll.offset, 0.0);
        let field = layout.node(sample::field()).unwrap();
        assert_eq!(field.clip, Some(root.padding_box));
        assert!(field.visible_bounds().is_none());

        let mut offsets = ScrollOffsets::new();
        offsets.insert(
            sample::root(),
            Point {
                x: 500.0,
                y: 10_000.0,
            },
        );
        let scrolled = layout_with(&sample, short, &Interaction::default(), &offsets);
        assert_eq!(
            scrolled.root().unwrap().scroll.unwrap().offset_x,
            0.0,
            "overflow-y-auto does not scroll across"
        );
        let root = scrolled.root().unwrap();
        assert_eq!(root.scroll.unwrap().offset, scroll.max, "offset is clamped");
        let moved = scrolled.bounds(sample::field()).unwrap();
        assert_eq!(moved.y, field.bounds.y - scroll.max);
        assert!(
            scrolled
                .node(sample::field())
                .unwrap()
                .visible_bounds()
                .is_some()
        );
    }

    #[test]
    fn borders_of_unclipped_children_are_never_cut() {
        let root = tree(
            div()
                .class("p-0")
                .child(div().class("border-2 border-primary w-20 h-20")),
        );
        let layout = layout(&root, WINDOW);
        let child = layout.get(1);
        assert_eq!(child.clip, None);
        assert_eq!((child.bounds.x, child.bounds.y), (0.0, 0.0));
        assert_eq!(child.padding_box.x, 2.0);
    }

    #[test]
    fn hit_path_respects_z_index_clip_hidden_and_pointer_events() {
        let root = tree(div().class("relative").children([
            div().class("absolute inset-0 z-10 pointer-events-none"),
            div().class("absolute top-0 left-0 w-20 h-20 z-20"),
            div().class("absolute top-0 left-0 w-20 h-20"),
            div().class("hidden w-full h-full"),
        ]));
        let layout = layout(&root, WINDOW);
        let ids: Vec<WidgetId> = root.children.iter().map(|child| child.id).collect();
        let point = Point { x: 10.0, y: 10.0 };
        assert_eq!(layout.hit_path(point), vec![root.id, ids[1]]);
        let order: Vec<WidgetId> = layout
            .paint_order()
            .iter()
            .map(|index| layout.get(*index).id)
            .collect();
        assert_eq!(order, vec![root.id, ids[2], ids[0], ids[1]]);
        assert_eq!(layout.hit_path(Point { x: 500.0, y: 500.0 }), vec![root.id]);
    }

    #[test]
    fn interaction_state_selects_variants_and_group_state_reaches_descendants() {
        let root = tree(
            div().child(
                div()
                    .class("group bg-background hover:bg-hover")
                    .child(text("x").class("text-foreground group-hover:text-primary")),
            ),
        );
        let theme = Theme::kai(Density::Comfortable);
        let card = root.children[0].id;
        let label = root.children[0].children[0].id;
        let idle = layout(&root, WINDOW);
        assert_eq!(
            idle.node(card).unwrap().style.background,
            Some(theme.colors.background)
        );
        let hovered = layout_with(
            &root,
            WINDOW,
            &Interaction {
                hovered: vec![root.id, card],
                ..Interaction::default()
            },
            &ScrollOffsets::new(),
        );
        assert_eq!(
            hovered.node(card).unwrap().style.background,
            Some(theme.colors.hover)
        );
        assert_eq!(
            hovered.node(label).unwrap().text.color,
            theme.colors.primary
        );
    }

    #[test]
    fn reveal_scrolls_the_container_to_show_the_focused_node() {
        let sample = sample::tree();
        let short = Size {
            width: 420.0,
            height: 120.0,
        };
        let layout = layout(&sample, short);
        let mut offsets = ScrollOffsets::new();
        assert!(layout.reveal(sample::field(), 4.0, &mut offsets));
        let revealed = layout_with(&sample, short, &Interaction::default(), &offsets);
        let root = revealed.root().unwrap();
        let field = revealed.bounds(sample::field()).unwrap();
        assert!(field.y + field.height + 4.0 <= root.padding_box.y + root.padding_box.height + 0.5);
        assert!(!revealed.reveal(sample::field(), 4.0, &mut offsets));

        let target = revealed.scroll_target(&[sample::root(), sample::field()], Axis::Y, -10.0);
        assert_eq!(target.map(|(id, _)| id), Some(sample::root()));
        assert!(
            layout
                .scroll_target(&[sample::root()], Axis::Y, -10.0)
                .is_none(),
            "already at the top"
        );
        assert!(
            revealed
                .scroll_target(&[sample::root()], Axis::X, 10.0)
                .is_none(),
            "nothing to scroll across"
        );
    }

    /// A 200-point-wide `overflow-x-auto` strip of ten 80-point cells.
    fn strip() -> Element<Msg> {
        tree(
            div().class("p-0").child(
                div()
                    .class("flex w-[200px] h-10 overflow-x-auto")
                    .children((0..10).map(|_| div().class("w-20 h-10 shrink-0"))),
            ),
        )
    }

    #[test]
    fn overflow_x_auto_scrolls_across_clamps_and_clips() {
        let root = strip();
        let strip = root.children[0].id;
        let cells: Vec<WidgetId> = root.children[0].children.iter().map(|c| c.id).collect();
        let layout = layout(&root, WINDOW);
        let scroll = layout.node(strip).unwrap().scroll.unwrap();
        assert_eq!((scroll.max_x, scroll.max), (600.0, 0.0));
        assert_eq!(
            layout
                .node(cells[2])
                .unwrap()
                .visible_bounds()
                .unwrap()
                .width,
            40.0,
            "cut at the strip's edge"
        );
        assert!(layout.node(cells[3]).unwrap().visible_bounds().is_none());

        let mut offsets = ScrollOffsets::new();
        offsets.insert(strip, Point { x: 170.0, y: 50.0 });
        let scrolled = layout_with(&root, WINDOW, &Interaction::default(), &offsets);
        let scroll = scrolled.node(strip).unwrap().scroll.unwrap();
        assert_eq!((scroll.offset_x, scroll.offset), (170.0, 0.0));
        assert_eq!(scrolled.bounds(cells[3]).unwrap().x, 240.0 - 170.0);
        let point = Point { x: 75.0, y: 20.0 };
        assert_eq!(scrolled.hit_path(point).last(), Some(&cells[3]));

        offsets.insert(
            strip,
            Point {
                x: 10_000.0,
                y: 0.0,
            },
        );
        let end = layout_with(&root, WINDOW, &Interaction::default(), &offsets);
        assert_eq!(end.node(strip).unwrap().scroll.unwrap().offset_x, 600.0);
        assert_eq!(
            end.bounds(cells[9]).unwrap().x,
            120.0,
            "the last cell ends at 200"
        );

        assert_eq!(
            layout
                .scroll_target(&[root.id, strip], Axis::X, 10.0)
                .map(|(id, _)| id),
            Some(strip)
        );
        assert!(
            layout
                .scroll_target(&[root.id, strip], Axis::X, -10.0)
                .is_none()
        );
        assert!(
            layout
                .scroll_target(&[root.id, strip], Axis::Y, 10.0)
                .is_none()
        );
    }

    #[test]
    fn reveal_scrolls_across_to_show_a_node() {
        let root = strip();
        let strip = root.children[0].id;
        let last = root.children[0].children[9].id;
        let layout = layout(&root, WINDOW);
        let mut offsets = ScrollOffsets::new();
        assert!(layout.reveal(last, 0.0, &mut offsets));
        assert_eq!(offsets[&strip], Point { x: 600.0, y: 0.0 });
        let first = root.children[0].children[0].id;
        let revealed = layout_with(&root, WINDOW, &Interaction::default(), &offsets);
        assert!(revealed.reveal(first, 0.0, &mut offsets));
        assert_eq!(offsets[&strip], Point { x: 0.0, y: 0.0 });
    }
}
