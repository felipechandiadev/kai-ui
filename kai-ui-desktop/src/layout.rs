//! Layout of the element tree over Taffy, in logical window coordinates.
//!
//! The root element is the window: it always gets exactly the viewport size,
//! whatever its classes say, and everything else lives inside it. Each
//! element's classes are resolved to a [`Style`] for its interaction state
//! (hover, focus, active, disabled, group) and translated to Taffy, so
//! block, flex and grid behave as in CSS. Elements with `overflow-hidden` or
//! `overflow-auto` clip their descendants to their padding box; `overflow-auto`
//! elements also scroll vertically with their own offset ([`ScrollOffsets`]).
//!
//! Every call lays the whole tree out from scratch; there is no cache or
//! incremental invalidation besides the parsed-class cache ([`StyleCache`]).

use std::collections::HashMap;

use taffy::prelude::{AvailableSpace, NodeId, Size as TaffySize, TaffyTree};

use crate::element::{Element, Kind, Role, WidgetId};
use crate::geometry::{Insets, Point, Rect, Size};
use crate::icons::Icon;
use crate::render::TextId;
use crate::render::text::TextStyle;
use crate::style::{ComputedText, Display, Overflow, Style};
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

/// Pointer and keyboard state that selects class variants.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Interaction {
    /// Hit-test path under the pointer, root first.
    pub hovered: Vec<WidgetId>,
    pub focused: Option<WidgetId>,
    pub pressed: Option<WidgetId>,
}

/// Vertical scroll offset of each `overflow-auto` element.
pub type ScrollOffsets = HashMap<WidgetId, f32>;

/// Parsed class strings, reused across frames. Clear it when the theme
/// changes.
#[derive(Default)]
pub struct StyleCache {
    lists: HashMap<String, ClassList>,
}

impl StyleCache {
    pub fn get(&mut self, classes: &str, theme: &Theme) -> &ClassList {
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
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollState {
    pub offset: f32,
    pub max: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutNode {
    pub id: WidgetId,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub content: Content,
    pub role: Option<Role>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub disabled: bool,
    pub is_pressed: bool,
    pub required: bool,
    /// Input focused by a click on this node (see
    /// [`Element::delegates_focus`]).
    pub focus_delegate: Option<WidgetId>,
    /// Clickable or editable (see [`Element::is_control`]).
    pub control: bool,
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
}

pub fn text_style(text: &ComputedText) -> TextStyle {
    TextStyle {
        font_size: text.font_size,
        line_height: text.line_height,
        weight: text.weight,
        wrap: !text.nowrap,
    }
}

struct Builder<'a> {
    theme: &'a Theme,
    interaction: &'a Interaction,
    focus_path: Vec<WidgetId>,
    styles: &'a mut StyleCache,
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
        let hover = interaction.hovered.contains(&id);
        let focus_within = self.focus_path.contains(&id);
        let state = State {
            hover,
            focus: interaction.focused == Some(id),
            focus_within,
            active: interaction.pressed == Some(id),
            disabled: element.disabled,
            group_hover: inherited.group_hover,
            group_focus_within: inherited.group_focus_within,
        };
        let classes = self.styles.get(&element.class, self.theme);
        let is_group = classes.group;
        let style = classes.resolve(state);
        let mut text = inherited.text.inherit(&style.text);
        let content = match &element.kind {
            Kind::Div => Content::Box,
            Kind::Text(content) => Content::Text(content.clone()),
            Kind::Icon(icon) => Content::Icon(*icon),
            Kind::Input {
                placeholder,
                masked,
                read_only,
                ..
            } => {
                text.nowrap = true;
                Content::Input {
                    placeholder: placeholder.clone(),
                    masked: *masked,
                    read_only: *read_only,
                }
            }
        };
        let index = self.nodes.len();
        let node = LayoutNode {
            id,
            parent,
            children: Vec::new(),
            content,
            role: element.role,
            label: element.label.clone(),
            description: element.description.clone(),
            disabled: element.disabled,
            is_pressed: element.is_pressed,
            required: element.required,
            focus_delegate: element
                .delegates_focus
                .then(|| element.first_input())
                .flatten(),
            control: element.is_control(),
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
        };
        let taffy_style = node.style.to_taffy();
        self.nodes.push(node);
        self.handles.push(None);
        if let Some(parent) = parent {
            self.nodes[parent].children.push(index);
        }

        let handle = match element.kind {
            Kind::Div => {
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
            let available_width = match available.width {
                AvailableSpace::Definite(width) => width,
                _ => INPUT_INTRINSIC_WIDTH,
            };
            Size {
                width: INPUT_INTRINSIC_WIDTH.min(available_width),
                height: nodes[index].text.line_height,
            }
        }
    };
    // Rounded up so pixel-snapped layout never trims the last glyph.
    TaffySize {
        width: known.width.unwrap_or(measured.width.ceil()),
        height: known.height.unwrap_or(measured.height.ceil()),
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
    /// first, so its border box plus `margin` becomes visible. Returns
    /// whether any offset changed.
    pub fn reveal(&self, id: WidgetId, margin: f32, offsets: &mut ScrollOffsets) -> bool {
        let Some(node) = self.node(id) else {
            return false;
        };
        let mut top = node.bounds.y - margin;
        let mut bottom = node.bounds.y + node.bounds.height + margin;
        let mut changed = false;
        for ancestor in self.ancestors(id) {
            let Some(scroll) = ancestor.scroll else {
                continue;
            };
            let view = ancestor.padding_box;
            let delta = if top < view.y {
                top - view.y
            } else if bottom > view.y + view.height {
                (bottom - (view.y + view.height)).min(top - view.y)
            } else {
                0.0
            };
            let next = (scroll.offset + delta).clamp(0.0, scroll.max);
            if next != scroll.offset {
                offsets.insert(ancestor.id, next);
                changed = true;
            }
            let moved = next - scroll.offset;
            top = (top - moved).max(view.y);
            bottom = (bottom - moved).min(view.y + view.height);
        }
        changed
    }

    /// Innermost scroll container on `path` (root first) that can move by
    /// `delta` (positive = toward the end of the content).
    pub fn scroll_target(&self, path: &[WidgetId], delta: f32) -> Option<(WidgetId, ScrollState)> {
        path.iter().rev().find_map(|id| {
            let scroll = self.node(*id)?.scroll?;
            let can_move = if delta > 0.0 {
                scroll.offset < scroll.max
            } else {
                scroll.offset > 0.0
            };
            can_move.then_some((*id, scroll))
        })
    }
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
    node.scroll = (node.style.overflow == Overflow::Auto).then(|| {
        let max = layout.scroll_height();
        ScrollState {
            offset: offsets
                .get(&node.id)
                .copied()
                .unwrap_or(0.0)
                .clamp(0.0, max),
            max,
        }
    });
    if !node.hidden {
        match &node.content {
            Content::Box | Content::Icon(_) => {}
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
            Content::Input { placeholder, .. } => {
                let line_height = node.text.line_height;
                text.measure(
                    TextId::placeholder(node.id),
                    placeholder,
                    text_style(&node.text),
                    None,
                );
                node.text_box = Some(Rect {
                    x: content_box.x,
                    y: content_box.y + ((content_box.height - line_height) / 2.0).max(0.0),
                    width: content_box.width.max(0.0),
                    height: line_height,
                });
            }
        }
    }
    let child_clip = if node.style.overflow == Overflow::Visible {
        clip
    } else {
        clip_to(clip, padding_box)
    };
    let scroll_offset = node.scroll.map_or(0.0, |scroll| scroll.offset);
    let origin = Point {
        x: bounds.x,
        y: bounds.y - scroll_offset,
    };
    let children = node.children.clone();
    for child in children {
        place(
            nodes, handles, taffy, child, origin, child_clip, offsets, text,
        );
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

#[cfg(test)]
pub mod testing {
    use super::TextMeasure;
    use crate::geometry::Size;
    use crate::render::TextId;
    use crate::render::text::TextStyle;

    /// Deterministic measurement: every character is half the font size wide
    /// and wrapping breaks at any character.
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
            let char_width = style.font_size / 2.0;
            let full = text.chars().count() as f32 * char_width;
            match max_width.filter(|_| style.wrap) {
                Some(max) if full > max => {
                    let per_line = (max / char_width).floor().max(1.0);
                    let lines = (text.chars().count() as f32 / per_line).ceil();
                    Size {
                        width: per_line * char_width,
                        height: lines * style.line_height,
                    }
                }
                _ => Size {
                    width: full,
                    height: style.line_height,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::FixedMeasure;
    use super::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::element::sample::{self, Msg};
    use crate::element::{Element, WidgetId, div, text};
    use crate::geometry::{Point, Rect, Size};
    use crate::theme::{Density, Theme};

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
        offsets.insert(sample::root(), 10_000.0);
        let scrolled = layout_with(&sample, short, &Interaction::default(), &offsets);
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

        let target = revealed.scroll_target(&[sample::root(), sample::field()], -10.0);
        assert_eq!(target.map(|(id, _)| id), Some(sample::root()));
        assert!(
            layout.scroll_target(&[sample::root()], -10.0).is_none(),
            "already at the top"
        );
    }
}
