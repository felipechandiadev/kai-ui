//! The floating layer (docs/dev/32): popovers, menus, calendars and
//! combobox lists that hang from an element but draw above the window.
//!
//! An element marked with [`Element::floating`] stays where the view put
//! it (child of the element it hangs from, its *anchor*), so its ids,
//! `test_id`s, Tab order, messages and accessibility stay those of that
//! place. Layout lifts it out of the flow: it is sized on its own against
//! the window (as a CSS `position: fixed` box), placed next to the anchor's
//! laid-out border box ([`position`]: preferred [`Side`], [`Align`], flip
//! and shift to stay in the window), and neither the anchor's `overflow-*`
//! clips nor its scroll containers hold it. It paints and takes the pointer
//! above every root child with the `z-*` of the one that contains it (its
//! level: the app's content, the sidebar next to it), whatever the `z-*`
//! of its siblings deeper down; root children with a higher `z-*` (an app
//! dialog, the session's tooltip, pop-up lists, palette and confirmation)
//! stay above it.
//!
//! The session dismisses it: Escape (or ⌘.) and a press outside its anchor
//! send its [`Floating::on_dismiss`]. [`OverlayFocus`] says where the focus
//! goes while it is open.
//!
//! ```ignore
//! div().class("flex flex-col").children([
//!     button("Opciones").on_click(Msg::Toggle),
//!     div()
//!         .class("w-56 p-1 rounded-md border bg-background shadow-lg")
//!         .role(Role::Menu)
//!         .children(items)
//!         .floating(Floating::below().contain_focus().on_dismiss(Msg::Close)),
//! ])
//! ```

use std::rc::Rc;

use crate::element::Element;
use crate::geometry::{Point, Rect, Size};
use crate::responder::Action;

/// Default room left between the anchor and the floating element.
pub const GAP: f32 = 4.0;
/// Default room kept between the floating element and the window edges.
pub const MARGIN: f32 = 8.0;

/// Side of the anchor a floating element goes on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    #[default]
    Below,
    Above,
    Right,
    Left,
}

impl Side {
    /// The side across the anchor.
    pub fn opposite(self) -> Self {
        match self {
            Side::Below => Side::Above,
            Side::Above => Side::Below,
            Side::Right => Side::Left,
            Side::Left => Side::Right,
        }
    }

    /// Below or above: the element is placed down and slides across.
    pub fn vertical(self) -> bool {
        matches!(self, Side::Below | Side::Above)
    }
}

/// How a floating element lines up with its anchor along the side: its
/// start edge with the anchor's start (left, or top on the sides), both
/// centers, or both end edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

/// Where the focus goes while a floating element is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverlayFocus {
    /// It stays where it is, usually in the anchor's field (a combobox
    /// list, a hover card). Tab goes on in document order, through the
    /// floating element right after its anchor.
    #[default]
    Anchor,
    /// It moves to the first control inside when it appears, Tab stays
    /// inside, and it goes back where it was when it leaves (a menu, a
    /// calendar). The app's shortcuts are off meanwhile, as with the
    /// session's pop-up lists. A press outside only dismisses it.
    Contain,
}

/// Where a floating element goes, for [`position`]. After layout, the
/// layout node's `floating` carries the side it ended up on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub side: Side,
    pub align: Align,
    /// Points between the anchor and the element.
    pub gap: f32,
    /// Points kept clear of the window edges.
    pub margin: f32,
    /// Go to the opposite side when it does not fit on `side` and fits
    /// there (or has more room there).
    pub flip: bool,
    /// Slide along the side to stay in the window; when it fits on no
    /// side, also across, over the anchor.
    pub shift: bool,
    /// At least as wide (or tall, on the sides) as the anchor.
    pub match_width: bool,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            side: Side::Below,
            align: Align::Start,
            gap: GAP,
            margin: MARGIN,
            flip: true,
            shift: true,
            match_width: false,
        }
    }
}

/// An element in the floating layer: where it goes, where the focus goes
/// and what dismisses it ([`Element::floating`]).
#[derive(Clone, Debug)]
pub struct Floating<Msg> {
    pub placement: Placement,
    pub focus: OverlayFocus,
    /// Sent on Escape or ⌘. while the focus is inside it or in its anchor,
    /// and on a press outside its anchor. Without it, it stays until the
    /// app removes it.
    pub on_dismiss: Option<Msg>,
}

impl<Msg> Floating<Msg> {
    /// On `side` of the anchor, aligned at the start, flipping and shifting
    /// to stay in the window; the focus stays put.
    pub fn new(side: Side) -> Self {
        Self {
            placement: Placement {
                side,
                ..Placement::default()
            },
            focus: OverlayFocus::Anchor,
            on_dismiss: None,
        }
    }

    pub fn below() -> Self {
        Self::new(Side::Below)
    }

    pub fn above() -> Self {
        Self::new(Side::Above)
    }

    pub fn right() -> Self {
        Self::new(Side::Right)
    }

    pub fn left() -> Self {
        Self::new(Side::Left)
    }

    pub fn align(mut self, align: Align) -> Self {
        self.placement.align = align;
        self
    }

    /// Points between the anchor and the element ([`GAP`] by default).
    pub fn gap(mut self, gap: f32) -> Self {
        self.placement.gap = gap;
        self
    }

    /// Points kept clear of the window edges ([`MARGIN`] by default).
    pub fn margin(mut self, margin: f32) -> Self {
        self.placement.margin = margin;
        self
    }

    /// Stay on the chosen side even when it does not fit.
    pub fn no_flip(mut self) -> Self {
        self.placement.flip = false;
        self
    }

    /// Do not slide to stay in the window.
    pub fn no_shift(mut self) -> Self {
        self.placement.shift = false;
        self
    }

    /// At least as wide as the anchor (a combobox list under its field).
    pub fn match_width(mut self) -> Self {
        self.placement.match_width = true;
        self
    }

    pub fn focus(mut self, focus: OverlayFocus) -> Self {
        self.focus = focus;
        self
    }

    /// [`OverlayFocus::Contain`]: a menu or a calendar.
    pub fn contain_focus(self) -> Self {
        self.focus(OverlayFocus::Contain)
    }

    /// See [`Floating::on_dismiss`](field@Floating::on_dismiss).
    pub fn on_dismiss(mut self, msg: Msg) -> Self {
        self.on_dismiss = Some(msg);
        self
    }

    pub(crate) fn map_with<To>(self, f: &Rc<dyn Fn(Msg) -> To>) -> Floating<To> {
        Floating {
            placement: self.placement,
            focus: self.focus,
            on_dismiss: self.on_dismiss.map(|msg| f(msg)),
        }
    }
}

impl<Msg: Clone + 'static> Element<Msg> {
    /// Draw this element in the floating layer, next to its parent (see
    /// the [module docs](self)). Give it a role ([`Role::Menu`],
    /// [`Role::Dialog`], [`Role::List`]…) and a name; give the control that
    /// opens it [`Element::expanded`].
    ///
    /// [`Role::Menu`]: crate::element::Role::Menu
    /// [`Role::Dialog`]: crate::element::Role::Dialog
    /// [`Role::List`]: crate::element::Role::List
    pub fn floating(mut self, floating: Floating<Msg>) -> Self {
        if let Some(on_dismiss) = floating.on_dismiss.clone() {
            // Escape from inside goes up the responder chain: a control
            // that takes it (a search field with text) still goes first.
            let inner = self.handlers.on_action.take();
            self.handlers.on_action = Some(Rc::new(move |action| {
                inner
                    .as_ref()
                    .and_then(|handler| handler(action))
                    .or_else(|| (action == Action::CancelOperation).then(|| on_dismiss.clone()))
            }));
        }
        self.floating = Some(Box::new(floating));
        self
    }
}

/// Top-left corner of a `size` box hanging from `anchor` in a `viewport`
/// window, and the side it ends up on.
///
/// It goes on `placement.side` when it fits there; with `flip`, on the
/// opposite side when it fits there instead, or else on whichever has more
/// room. With `shift` it then slides along the side to stay `margin` inside
/// the window and, when it fits on neither side, across too (over the
/// anchor) rather than leaving the window.
pub fn position(anchor: Rect, size: Size, viewport: Size, placement: Placement) -> (Point, Side) {
    let Placement {
        side,
        align,
        gap,
        margin,
        flip,
        shift,
        ..
    } = placement;
    let room = |side: Side| match side {
        Side::Below => viewport.height - margin - (anchor.y + anchor.height + gap),
        Side::Above => anchor.y - gap - margin,
        Side::Right => viewport.width - margin - (anchor.x + anchor.width + gap),
        Side::Left => anchor.x - gap - margin,
    };
    let need = |side: Side| {
        if side.vertical() {
            size.height
        } else {
            size.width
        }
    };
    let side = if !flip || need(side) <= room(side) {
        side
    } else if need(side) <= room(side.opposite()) || room(side.opposite()) > room(side) {
        side.opposite()
    } else {
        side
    };
    let main = match side {
        Side::Below => anchor.y + anchor.height + gap,
        Side::Above => anchor.y - gap - size.height,
        Side::Right => anchor.x + anchor.width + gap,
        Side::Left => anchor.x - gap - size.width,
    };
    let (start, length, own) = if side.vertical() {
        (anchor.x, anchor.width, size.width)
    } else {
        (anchor.y, anchor.height, size.height)
    };
    let cross = match align {
        Align::Start => start,
        Align::Center => start + (length - own) / 2.0,
        Align::End => start + length - own,
    };
    let keep_in = |at: f32, own: f32, window: f32| {
        // Too big for the window: from its start edge.
        at.min(window - margin - own).max(margin)
    };
    let (x, y) = if side.vertical() {
        (cross, main)
    } else {
        (main, cross)
    };
    let (x, y) = if !shift {
        (x, y)
    } else if side.vertical() {
        let y = if need(side) > room(side) {
            keep_in(y, size.height, viewport.height)
        } else {
            y
        };
        (keep_in(x, size.width, viewport.width), y)
    } else {
        let x = if need(side) > room(side) {
            keep_in(x, size.width, viewport.width)
        } else {
            x
        };
        (x, keep_in(y, size.height, viewport.height))
    };
    (Point { x, y }, side)
}

#[cfg(test)]
mod tests {
    use super::{Align, Floating, Placement, Side, position};
    use crate::element::{Element, WidgetId, div, text};
    use crate::geometry::{Point, Rect, Size};
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Axis, Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::testing::{By, Screen};
    use crate::theme::{Density, Theme};

    const WINDOW: Size = Size {
        width: 400.0,
        height: 300.0,
    };
    const PANEL: Size = Size {
        width: 100.0,
        height: 80.0,
    };

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn at(side: Side) -> Placement {
        Placement {
            side,
            ..Placement::default()
        }
    }

    #[test]
    fn goes_on_the_preferred_side_when_it_fits() {
        let anchor = rect(50.0, 50.0, 80.0, 20.0);
        let placed = position(anchor, PANEL, WINDOW, at(Side::Below));
        assert_eq!(placed, (Point { x: 50.0, y: 74.0 }, Side::Below));
        let placed = position(anchor, PANEL, WINDOW, at(Side::Right));
        assert_eq!(placed, (Point { x: 134.0, y: 50.0 }, Side::Right));
    }

    #[test]
    fn flips_to_the_opposite_side_when_it_does_not_fit() {
        let near_bottom = rect(50.0, 250.0, 80.0, 20.0);
        let placed = position(near_bottom, PANEL, WINDOW, at(Side::Below));
        assert_eq!(placed, (Point { x: 50.0, y: 166.0 }, Side::Above));
        let near_top = rect(50.0, 10.0, 80.0, 20.0);
        let placed = position(near_top, PANEL, WINDOW, at(Side::Above));
        assert_eq!(placed, (Point { x: 50.0, y: 34.0 }, Side::Below));
        let near_right = rect(330.0, 100.0, 40.0, 20.0);
        let placed = position(near_right, PANEL, WINDOW, at(Side::Right));
        assert_eq!(placed, (Point { x: 226.0, y: 100.0 }, Side::Left));
        let near_left = rect(20.0, 100.0, 40.0, 20.0);
        let placed = position(near_left, PANEL, WINDOW, at(Side::Left));
        assert_eq!(placed, (Point { x: 64.0, y: 100.0 }, Side::Right));
    }

    #[test]
    fn fits_on_neither_side_takes_the_roomier_one_and_shifts_into_the_window() {
        let short = Size {
            width: 400.0,
            height: 150.0,
        };
        let tall = Size {
            width: 100.0,
            height: 120.0,
        };
        // 78 points below, 28 above: below, slid up over the anchor.
        let anchor = rect(50.0, 40.0, 80.0, 20.0);
        let placed = position(anchor, tall, short, at(Side::Below));
        assert_eq!(placed, (Point { x: 50.0, y: 22.0 }, Side::Below));
        // Taller than the window: from the top margin.
        let huge = Size {
            width: 100.0,
            height: 500.0,
        };
        let placed = position(anchor, huge, short, at(Side::Below));
        assert_eq!(placed, (Point { x: 50.0, y: 8.0 }, Side::Below));
    }

    #[test]
    fn shifts_along_the_side_to_stay_in_the_window() {
        let near_right = rect(360.0, 50.0, 30.0, 20.0);
        let placed = position(near_right, PANEL, WINDOW, at(Side::Below));
        assert_eq!(placed, (Point { x: 292.0, y: 74.0 }, Side::Below));
        let near_left = rect(2.0, 50.0, 30.0, 20.0);
        let placed = position(near_left, PANEL, WINDOW, at(Side::Below));
        assert_eq!(placed.0.x, 8.0);
        let near_bottom = rect(50.0, 270.0, 40.0, 20.0);
        let placed = position(near_bottom, PANEL, WINDOW, at(Side::Right));
        assert_eq!(placed, (Point { x: 94.0, y: 212.0 }, Side::Right));
    }

    #[test]
    fn aligns_on_the_cross_axis() {
        let anchor = rect(100.0, 50.0, 80.0, 20.0);
        let aligned = |align| {
            let placement = Placement {
                align,
                ..Placement::default()
            };
            position(anchor, PANEL, WINDOW, placement).0
        };
        assert_eq!(aligned(Align::Start).x, 100.0);
        assert_eq!(aligned(Align::Center).x, 90.0);
        assert_eq!(aligned(Align::End).x, 80.0);
        let beside = Placement {
            side: Side::Right,
            align: Align::Center,
            ..Placement::default()
        };
        assert_eq!(position(anchor, PANEL, WINDOW, beside).0.y, 20.0);
    }

    #[test]
    fn no_flip_and_no_shift_keep_the_requested_spot() {
        let near_bottom = rect(360.0, 250.0, 30.0, 20.0);
        let floating = Floating::<()>::below().no_flip();
        let placed = position(near_bottom, PANEL, WINDOW, floating.placement);
        assert_eq!(placed, (Point { x: 292.0, y: 212.0 }, Side::Below));
        let floating = Floating::<()>::below().no_flip().no_shift();
        let placed = position(near_bottom, PANEL, WINDOW, floating.placement);
        assert_eq!(placed, (Point { x: 360.0, y: 274.0 }, Side::Below));
        let floating = Floating::<()>::below().gap(0.0).margin(0.0);
        let placed = position(
            rect(0.0, 0.0, 10.0, 10.0),
            PANEL,
            WINDOW,
            floating.placement,
        );
        assert_eq!(placed, (Point { x: 0.0, y: 10.0 }, Side::Below));
    }

    fn layout(root: &mut Element<()>, viewport: Size, scroll: &ScrollOffsets) -> Layout {
        root.assign_ids();
        Layout::compute(
            root,
            &LayoutInput {
                viewport,
                theme: &Theme::kai(Density::Comfortable),
                interaction: &Interaction::default(),
                scroll,
            },
            &mut StyleCache::default(),
            &mut FixedMeasure,
        )
    }

    fn bounds(layout: &Layout, test_id: &str) -> Rect {
        Screen::new(layout).get(By::test_id(test_id)).bounds()
    }

    fn id(layout: &Layout, test_id: &str) -> WidgetId {
        Screen::new(layout).get(By::test_id(test_id)).id()
    }

    fn center(rect: Rect) -> Point {
        Point {
            x: rect.x + rect.width / 2.0,
            y: rect.y + rect.height / 2.0,
        }
    }

    fn panel() -> Element<()> {
        div()
            .class("w-32 h-24 bg-background")
            .test_id("panel")
            .floating(Floating::below())
    }

    #[test]
    fn hangs_from_the_anchor_unclipped_and_out_of_its_flow() {
        let anchor = |float: bool| {
            let mut anchor = div().class("w-20").test_id("anchor").child(text("Abrir"));
            if float {
                anchor = anchor.child(panel());
            }
            div().class("p-4").child(
                div()
                    .class("w-40 h-10 overflow-hidden")
                    .test_id("clip")
                    .child(anchor),
            )
        };
        let plain = layout(&mut anchor(false), WINDOW, &ScrollOffsets::new());
        let layout = layout(&mut anchor(true), WINDOW, &ScrollOffsets::new());
        let hanging = bounds(&layout, "anchor");
        assert_eq!(hanging, bounds(&plain, "anchor"), "out of the flow");
        let panel = bounds(&layout, "panel");
        assert_eq!(
            panel,
            rect(16.0, hanging.y + hanging.height + 4.0, 128.0, 96.0)
        );
        let node = layout.node(id(&layout, "panel")).unwrap();
        assert_eq!(node.clip, None, "the anchor's overflow does not clip it");
        assert_eq!(node.floating.unwrap().side, Side::Below);
        // Below the clipping box, the pointer still finds it, through its
        // anchor.
        let point = Point { x: 30.0, y: 120.0 };
        assert!(point.y > bounds(&layout, "clip").y + 40.0);
        let path = layout.hit_path(point);
        assert_eq!(path.last(), Some(&id(&layout, "panel")));
        assert!(path.contains(&id(&layout, "anchor")));
    }

    #[test]
    fn follows_its_anchor_through_scroll_without_scrolling_with_it() {
        let mut root = div().class("p-4").child(
            div()
                .class("h-20 overflow-y-auto")
                .test_id("scroller")
                .child(
                    div()
                        .class("h-96")
                        .child(div().class("mt-10 h-6").test_id("anchor").child(panel())),
                ),
        );
        let unscrolled = layout(&mut root, WINDOW, &ScrollOffsets::new());
        let scroller = id(&unscrolled, "scroller");
        let mut offsets = ScrollOffsets::new();
        offsets.insert(scroller, Point { x: 0.0, y: 30.0 });
        let scrolled = layout(&mut root, WINDOW, &offsets);
        let (before, after) = (bounds(&unscrolled, "anchor"), bounds(&scrolled, "anchor"));
        assert_eq!(after.y, before.y - 30.0);
        assert_eq!(bounds(&scrolled, "panel").y, after.y + after.height + 4.0);
        // Its own content is shown, never by scrolling the anchor's
        // scroller…
        let mut kept = offsets.clone();
        assert!(!scrolled.reveal(id(&scrolled, "panel"), 8.0, &mut kept));
        assert_eq!(kept, offsets);
        // …nor does the wheel over it scroll what is under it.
        let path = scrolled.hit_path(center(bounds(&scrolled, "panel")));
        assert!(
            path.contains(&scroller),
            "the hit path goes through its anchor"
        );
        assert_eq!(scrolled.scroll_target(&path, Axis::Y, 10.0), None);
        assert_eq!(scrolled.scroll_target(&path, Axis::Y, -10.0), None);
    }

    #[test]
    fn paints_and_takes_the_pointer_over_its_layer_whatever_the_z_index() {
        // Both in the app's content (one root child).
        let mut root = div().child(
            div().class("relative w-80 h-80").children([
                div().class("w-20 h-6").test_id("anchor").child(panel()),
                div()
                    .class("absolute top-0 left-0 w-80 h-60 z-40 bg-background")
                    .test_id("cover"),
            ]),
        );
        let layout = layout(&mut root, WINDOW, &ScrollOffsets::new());
        let (panel, cover) = (id(&layout, "panel"), id(&layout, "cover"));
        let path = layout.hit_path(center(bounds(&layout, "panel")));
        assert_eq!(path.last(), Some(&panel));
        assert!(!path.contains(&cover));
        let rank = |id| {
            let index = layout
                .paint_order()
                .iter()
                .position(|index| layout.get(*index).id == id);
            index.expect("painted")
        };
        assert!(rank(panel) > rank(cover));
    }

    #[test]
    fn goes_over_the_root_children_of_its_level() {
        // A sidebar and the content after it, both at the root.
        let mut root = div().class("flex w-full h-full").children([
            div().class("w-20").child(
                div().class("w-20 h-6").test_id("anchor").child(
                    div()
                        .class("w-60 h-24")
                        .test_id("panel")
                        .floating(Floating::below()),
                ),
            ),
            div().class("flex-1 bg-background").test_id("content"),
        ]);
        let layout = layout(&mut root, WINDOW, &ScrollOffsets::new());
        let panel = bounds(&layout, "panel");
        let over_content = Point {
            x: panel.x + panel.width - 10.0,
            y: panel.y + 10.0,
        };
        assert!(over_content.x > bounds(&layout, "content").x);
        let path = layout.hit_path(over_content);
        assert_eq!(path.last(), Some(&id(&layout, "panel")));
    }

    #[test]
    fn stays_under_a_later_top_level_layer() {
        // A root child above the app's content, like an app dialog.
        let mut root = div().class("relative w-full h-full").children([
            div().child(div().class("w-20 h-6").test_id("anchor").child(panel())),
            div()
                .class("absolute inset-0 z-50 bg-background")
                .test_id("layer"),
        ]);
        let layout = layout(&mut root, WINDOW, &ScrollOffsets::new());
        let path = layout.hit_path(center(bounds(&layout, "panel")));
        assert_eq!(path.last(), Some(&id(&layout, "layer")));
        assert!(!path.contains(&id(&layout, "panel")));
    }

    #[test]
    fn flips_above_at_the_bottom_of_the_window_and_matches_the_width() {
        let mut root = div().class("relative w-full h-full").child(
            div()
                .class("absolute left-4 bottom-4 w-60 h-6")
                .test_id("anchor")
                .child(
                    div()
                        .class("h-24")
                        .test_id("panel")
                        .child(text("Uno"))
                        .floating(Floating::below().match_width()),
                ),
        );
        let layout = layout(&mut root, WINDOW, &ScrollOffsets::new());
        let (anchor, panel) = (bounds(&layout, "anchor"), bounds(&layout, "panel"));
        assert_eq!(panel.y + panel.height, anchor.y - 4.0);
        assert_eq!(panel.width, anchor.width);
        let node = layout.node(id(&layout, "panel")).unwrap();
        assert_eq!(node.floating.unwrap().side, Side::Above);
    }
}
