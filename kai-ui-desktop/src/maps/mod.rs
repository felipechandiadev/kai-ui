//! Interactive maps: the [`map`] element, [`marker`]s placed by latitude
//! and longitude, and the camera that gestures move.
//!
//! ```ignore
//! map(&self.vista)
//!     .on_view_change(Msg::MapaMovido)
//!     .label("Mapa")
//!     .test_id("mapa")
//!     .children([marker(plaza, Anchor::Bottom, icon(Icon::MapPin))])
//! ```
//!
//! The map shows the app's [`MapView`] and lets the person drag, scroll,
//! pinch, double click (⇧ to zoom out), use the arrow keys and `+`/`−`
//! when it has the focus, or its zoom buttons. Those gestures move the
//! session's camera without calling `update`; when one ends the app gets a
//! single `on_view_change`. A new view from the app glides there.

pub mod camera;
pub mod mvt;
pub mod pmtiles;
pub mod tiles;

use crate::element::{Element, Kind, Role, icon, text};
use crate::geo::{LatLon, MapView};
use crate::icons::Icon;
use tiles::{TileFrame, TileSource};

/// Credit for the map data, shown in a corner of every map.
pub const ATTRIBUTION: &str = "© OpenStreetMap contributors · Protomaps";

/// What a [`map`] element shows; the session replaces `view` with its
/// camera's before layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapSpec {
    pub view: MapView,
    /// Glide to a new view from the app instead of jumping.
    pub animate: bool,
    /// The basemap's tiles, if any.
    pub source: Option<TileSource>,
    /// The tiles to draw this frame (the session fills them in).
    pub tiles: TileFrame,
}

impl MapSpec {
    pub fn new(view: MapView) -> Self {
        Self {
            view: view.clamped(),
            animate: true,
            source: None,
            tiles: TileFrame::default(),
        }
    }
}

/// Which point of a marker sits on its coordinate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Anchor {
    /// The middle of its bottom edge (a pin).
    #[default]
    Bottom,
    Center,
    Top,
    TopLeft,
}

impl Anchor {
    /// Offset of the anchor point from the top left corner of a
    /// `width × height` box.
    pub fn offset(self, width: f32, height: f32) -> (f32, f32) {
        match self {
            Self::Bottom => (width / 2.0, height),
            Self::Center => (width / 2.0, height / 2.0),
            Self::Top => (width / 2.0, 0.0),
            Self::TopLeft => (0.0, 0.0),
        }
    }
}

/// A child of a [`map`] pinned to a coordinate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marker {
    pub at: LatLon,
    pub anchor: Anchor,
}

/// The map's zoom buttons, which the session handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapControl {
    ZoomIn,
    ZoomOut,
}

impl MapControl {
    pub fn delta(self) -> f64 {
        match self {
            Self::ZoomIn => 1.0,
            Self::ZoomOut => -1.0,
        }
    }
}

fn zoom_button<Msg>(control: MapControl) -> Element<Msg> {
    let (part, label, glyph) = match control {
        MapControl::ZoomIn => ("zoom_in", "Acercar", Icon::Plus),
        MapControl::ZoomOut => ("zoom_out", "Alejar", Icon::Minus),
    };
    let mut button = Element::new(Kind::Div)
        .part(part)
        .label(label)
        .role(Role::Button)
        .focus_on_press(false)
        .class(
            "flex items-center justify-center size-8 text-foreground hover:bg-hover \
             active:bg-active cursor-pointer focus-visible:ring-2 focus-visible:ring-primary/40",
        )
        .child(icon(glyph).class("size-4"));
    button.map_control = Some(control);
    button
}

/// An interactive map showing `view`. Give it a size (`h-96`, `flex-1`…)
/// and an accessible name; its children are usually [`marker`]s. Parts:
/// `zoom_in`, `zoom_out` and `attribution`.
pub fn map<Msg>(view: &MapView) -> Element<Msg> {
    Element::new(Kind::Map(Box::new(MapSpec::new(*view))))
        .role(Role::Group)
        .parts_root()
        .class(
            "relative overflow-hidden bg-muted focus-visible:ring-2 \
             focus-visible:ring-primary/40",
        )
        .children([
            Element::new(Kind::Div)
                .part("zoom")
                .class(
                    "absolute top-2 right-2 z-10 flex flex-col rounded-md border border-border \
                     bg-background/90 shadow-sm overflow-hidden",
                )
                .children([
                    zoom_button(MapControl::ZoomIn),
                    zoom_button(MapControl::ZoomOut),
                ]),
            Element::new(Kind::Div)
                .part("attribution")
                .class(
                    "absolute right-0 bottom-0 z-10 px-1 bg-background/70 text-xs \
                     text-muted-foreground",
                )
                .child(text(ATTRIBUTION)),
        ])
}

/// `child` pinned to `at` inside a [`map`], with its `anchor` point on
/// the coordinate; it moves with the map.
pub fn marker<Msg>(at: LatLon, anchor: Anchor, child: Element<Msg>) -> Element<Msg> {
    let mut marker = Element::new(Kind::Div).class("absolute").child(child);
    marker.marker = Some(Marker { at, anchor });
    marker
}

impl<Msg> Element<Msg> {
    /// Maps only: `handler` gets the view once a gesture (drag, scroll,
    /// pinch, keys, zoom buttons) ends.
    pub fn on_view_change(mut self, handler: impl Fn(MapView) -> Msg + 'static) -> Self {
        self.handlers.on_view_change = Some(std::rc::Rc::new(handler));
        self
    }

    /// Maps only: jump to a new view from the app instead of gliding.
    pub fn animate_view(mut self, animate: bool) -> Self {
        if let Kind::Map(spec) = &mut self.kind {
            spec.animate = animate;
        }
        self
    }

    /// Maps only: draw the vector tiles of `source` under the markers.
    pub fn basemap(mut self, source: TileSource) -> Self {
        if let Kind::Map(spec) = &mut self.kind {
            spec.source = Some(source);
        }
        self
    }
}

#[cfg(test)]
mod tests;
