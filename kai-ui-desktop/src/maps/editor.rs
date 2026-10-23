//! Editing the outline of one polygon on a map: drag a vertex, click the
//! middle of an edge to add one, delete one with the keyboard, undo.
//!
//! The draft belongs to the app, like the text of an input: it keeps a
//! [`PolygonDraft`], hands it to
//! [`Element::polygon_editor`](crate::element::Element::polygon_editor)
//! and applies the [`PolygonEdit`]s it gets back. The shape it edits
//! changes only when the app takes the draft, on [`PolygonEdit::Finish`].
//!
//! ```ignore
//! map(&self.vista)
//!     .polygon_editor(&self.borrador, SECTOR, Msg::Edicion)
//! // update:
//! Msg::Edicion(PolygonEdit::Finish) if self.borrador.is_valid() => self.guardar(),
//! Msg::Edicion(PolygonEdit::Cancel) => self.borrador = None,
//! Msg::Edicion(edit) => { self.borrador.apply(edit); }
//! ```

use std::rc::Rc;

use geojson::{Geometry, GeometryValue, Position};

use super::layers::MapLayer;
use super::{Anchor, Marker};
use crate::color::Color;
use crate::element::{Element, Kind, Role};
use crate::geo::{LatLon, LatLonBounds, WorldPoint, project, unproject};
use crate::responder::Action;

/// Fewest vertices a polygon keeps.
pub const MIN_VERTICES: usize = 3;
/// The edges that cross, drawn over the draft.
const CROSSING: Color = Color::hex(0xd64545);
const DRAFT_FILL: f32 = 0.12;
const DRAFT_STROKE: f32 = 2.0;
const CROSSING_STROKE: f32 = 3.0;
/// Undo steps a draft remembers.
const HISTORY: usize = 100;
/// Mean Earth radius of the spherical area formula, in meters.
const EARTH_RADIUS: f64 = 6_371_008.8;

/// A change to a [`PolygonDraft`], from the editor or the app.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PolygonEdit {
    /// Put vertex `index` at `to`; `first` opens an undo step (the first
    /// move of a drag, or a key press).
    Move {
        index: usize,
        to: LatLon,
        first: bool,
    },
    /// A new vertex at `at` after vertex `after`.
    Insert {
        after: usize,
        at: LatLon,
    },
    /// Drop vertex `index`, keeping at least [`MIN_VERTICES`].
    Remove {
        index: usize,
    },
    Undo,
    /// Return, or the app's own button: take the draft if it is valid.
    Finish,
    /// Escape: drop the draft.
    Cancel,
}

/// Edits compare by value; the editor only sends finite places.
impl Eq for PolygonEdit {}

/// The outer ring of a polygon being edited, without repeating its first
/// point, and the rings before each undoable step.
#[derive(Clone, Debug, PartialEq)]
pub struct PolygonDraft {
    ring: Vec<LatLon>,
    history: Vec<Vec<LatLon>>,
}

impl PolygonDraft {
    /// A draft of `points` (closed or not): at least three, all finite.
    pub fn from_ring(points: impl IntoIterator<Item = LatLon>) -> Result<Self, String> {
        let mut ring: Vec<LatLon> = points.into_iter().collect();
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        if let Some(bad) = ring
            .iter()
            .find(|at| !at.lat.is_finite() || !at.lon.is_finite())
        {
            return Err(format!("posición inválida: {bad:?}"));
        }
        if ring.len() < MIN_VERTICES {
            return Err(format!(
                "un polígono necesita al menos {MIN_VERTICES} vértices"
            ));
        }
        Ok(Self {
            ring,
            history: Vec::new(),
        })
    }

    /// A draft of a GeoJSON `Polygon` without holes.
    pub fn from_geometry(geometry: &Geometry) -> Result<Self, String> {
        let GeometryValue::Polygon { coordinates } = &geometry.value else {
            return Err(format!(
                "se esperaba un polígono: {}",
                geometry.value.type_name()
            ));
        };
        let [outer] = coordinates.as_slice() else {
            return Err("se esperaba un polígono de un solo anillo, sin huecos".to_owned());
        };
        let points = outer
            .iter()
            .map(|position| match position.as_slice() {
                [lon, lat, ..] => Ok(LatLon::new(*lat, *lon)),
                other => Err(format!("posición inválida: {other:?}")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_ring(points)
    }

    pub fn ring(&self) -> &[LatLon] {
        &self.ring
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Apply `edit`; `true` if the ring changed. Edits naming a vertex
    /// that is not there, removals below [`MIN_VERTICES`], `Finish` and
    /// `Cancel` change nothing.
    pub fn apply(&mut self, edit: PolygonEdit) -> bool {
        match edit {
            PolygonEdit::Move { index, to, first } => {
                if index >= self.ring.len() || !to.lat.is_finite() || !to.lon.is_finite() {
                    return false;
                }
                if first {
                    self.remember();
                }
                if self.ring[index] == to {
                    return false;
                }
                self.ring[index] = to;
                true
            }
            PolygonEdit::Insert { after, at } => {
                if after >= self.ring.len() || !at.lat.is_finite() || !at.lon.is_finite() {
                    return false;
                }
                self.remember();
                self.ring.insert(after + 1, at);
                true
            }
            PolygonEdit::Remove { index } => {
                if index >= self.ring.len() || self.ring.len() <= MIN_VERTICES {
                    return false;
                }
                self.remember();
                self.ring.remove(index);
                true
            }
            PolygonEdit::Undo => match self.history.pop() {
                Some(ring) => {
                    self.ring = ring;
                    true
                }
                None => false,
            },
            PolygonEdit::Finish | PolygonEdit::Cancel => false,
        }
    }

    fn remember(&mut self) {
        if self.history.len() == HISTORY {
            self.history.remove(0);
        }
        self.history.push(self.ring.clone());
    }

    /// The first two edges that cross; edge `i` goes from vertex `i` to
    /// the next one.
    pub fn crossing(&self) -> Option<(usize, usize)> {
        crossing(&self.ring)
    }

    /// Enough vertices and no crossing edges.
    pub fn is_valid(&self) -> bool {
        self.ring.len() >= MIN_VERTICES && self.crossing().is_none()
    }

    /// Area on the Earth's surface, in hectares.
    pub fn area_ha(&self) -> f64 {
        area_m2(&self.ring) / 10_000.0
    }

    /// The middle of edge `index` on the map.
    pub fn midpoint(&self, index: usize) -> LatLon {
        let a = project(self.ring[index % self.ring.len()]);
        let b = project(self.ring[(index + 1) % self.ring.len()]);
        unproject(WorldPoint {
            x: (a.x + b.x) / 2.0,
            y: (a.y + b.y) / 2.0,
        })
    }

    pub fn bounds(&self) -> LatLonBounds {
        LatLonBounds::around(self.ring.iter().copied()).expect("a draft has vertices")
    }

    /// A GeoJSON `Polygon` with the ring closed.
    pub fn to_geometry(&self) -> Geometry {
        let mut ring: Vec<Position> = self
            .ring
            .iter()
            .map(|at| Position::from(vec![at.lon, at.lat]))
            .collect();
        ring.push(ring[0].clone());
        Geometry::new(GeometryValue::Polygon {
            coordinates: vec![ring],
        })
    }
}

/// Which side of `a → b` the point `c` is on (twice the signed area).
fn orient(a: WorldPoint, b: WorldPoint, c: WorldPoint) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// `p`, known to be on the line through `a` and `b`, lies between them.
fn within(a: WorldPoint, b: WorldPoint, p: WorldPoint) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
}

fn segments_meet(p1: WorldPoint, p2: WorldPoint, q1: WorldPoint, q2: WorldPoint) -> bool {
    let d1 = orient(q1, q2, p1);
    let d2 = orient(q1, q2, p2);
    let d3 = orient(p1, p2, q1);
    let d4 = orient(p1, p2, q2);
    if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
        return true;
    }
    (d1 == 0.0 && within(q1, q2, p1))
        || (d2 == 0.0 && within(q1, q2, p2))
        || (d3 == 0.0 && within(p1, p2, q1))
        || (d4 == 0.0 && within(p1, p2, q2))
}

/// The first two non-adjacent edges of the closed `ring` that touch or
/// cross; edge `i` goes from vertex `i` to the next one.
pub fn crossing(ring: &[LatLon]) -> Option<(usize, usize)> {
    let n = ring.len();
    if n < 4 {
        return None;
    }
    let world: Vec<WorldPoint> = ring.iter().map(|at| project(*at)).collect();
    let edge = |i: usize| (world[i], world[(i + 1) % n]);
    for i in 0..n {
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let (p1, p2) = edge(i);
            let (q1, q2) = edge(j);
            if segments_meet(p1, p2, q1, q2) {
                return Some((i, j));
            }
        }
    }
    None
}

/// Area of the closed `ring` on a sphere the size of the Earth, in
/// square meters (the formula of Chamberlain and Duquette, as Leaflet
/// and Turf use it).
pub fn area_m2(ring: &[LatLon]) -> f64 {
    let n = ring.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        sum += (b.lon - a.lon).to_radians()
            * (2.0 + a.lat.to_radians().sin() + b.lat.to_radians().sin());
    }
    (sum * EARTH_RADIUS * EARTH_RADIUS / 2.0).abs()
}

/// Background, border or ring class of `color`.
fn color_class(prefix: &str, color: Color) -> String {
    let Color { r, g, b, .. } = color;
    format!("{prefix}-[#{r:02x}{g:02x}{b:02x}]")
}

impl<Msg: Clone + 'static> Element<Msg> {
    /// Maps only: edit `draft` in `color`. The map draws it emphasized
    /// over its layers (the edges that cross in red) with a handle on
    /// each vertex (part `vertex.<n>`, from 0) and a smaller one in the
    /// middle of each edge (part `midpoint.<n>`) that adds a vertex there.
    /// Dragging a vertex moves it; with a vertex focused, the arrows move
    /// it a point (ten with ⇧), Delete removes it and Return finishes;
    /// Escape cancels and ⌘Z undoes anywhere in the map. Every change
    /// comes back through `on_edit` for the app to [`PolygonDraft::apply`].
    pub fn polygon_editor(
        mut self,
        draft: &PolygonDraft,
        color: Color,
        on_edit: impl Fn(PolygonEdit) -> Msg + 'static,
    ) -> Self {
        let on_edit: Rc<dyn Fn(PolygonEdit) -> Msg> = Rc::new(on_edit);
        let ring = draft.ring();
        if let Kind::Map(spec) = &mut self.kind {
            if let Ok(layer) = MapLayer::ring(ring) {
                spec.layers.push(
                    layer
                        .fill(color, DRAFT_FILL)
                        .stroke(color, DRAFT_STROKE)
                        .selected(true),
                );
            }
            if let Some((a, b)) = draft.crossing() {
                for edge in [a, b] {
                    let ends = [ring[edge], ring[(edge + 1) % ring.len()]];
                    if let Ok(line) = MapLayer::line(&ends) {
                        spec.layers.push(line.stroke(CROSSING, CROSSING_STROKE));
                    }
                }
            }
        }
        let count = ring.len();
        let midpoints = (0..count).map(|index| {
            let mut handle = Element::new(Kind::Div)
                .key(format!("midpoint.{index}"))
                .part(format!("midpoint.{index}"))
                .label(format!(
                    "Agregar vértice entre {} y {}",
                    index + 1,
                    (index + 1) % count + 1
                ))
                .role(Role::Button)
                .focus_on_press(false)
                .class(&format!(
                    "absolute size-2.5 rounded-full border bg-background/80 opacity-70 \
                     hover:opacity-100 cursor-pointer focus-visible:ring-2 \
                     focus-visible:ring-primary/40 {}",
                    color_class("border", color)
                ))
                .on_click(on_edit(PolygonEdit::Insert {
                    after: index,
                    at: draft.midpoint(index),
                }));
            handle.marker = Some(Marker {
                at: draft.midpoint(index),
                anchor: Anchor::Center,
            });
            handle
        });
        let vertices = ring.iter().enumerate().map(|(index, at)| {
            let mut handle = Element::new(Kind::Div)
                .key(format!("vertex.{index}"))
                .part(format!("vertex.{index}"))
                .label(format!("Vértice {} de {count}", index + 1))
                .role(Role::Button)
                .class(&format!(
                    "absolute size-3 rounded-full border-2 bg-background cursor-pointer \
                     focus-visible:ring-2 focus-visible:ring-primary/40 {}",
                    color_class("border", color)
                ));
            handle.marker = Some(Marker {
                at: *at,
                anchor: Anchor::Center,
            });
            handle.map_vertex = Some(index);
            handle
        });
        let mut children: Vec<Element<Msg>> = midpoints.chain(vertices).collect();
        self.children.append(&mut children);
        let inner = self.handlers.on_action.take();
        let edit = Rc::clone(&on_edit);
        self.handlers.on_action = Some(Rc::new(move |action| {
            inner
                .as_ref()
                .and_then(|handler| handler(action))
                .or_else(|| {
                    match action {
                        Action::CancelOperation => Some(PolygonEdit::Cancel),
                        Action::Undo => Some(PolygonEdit::Undo),
                        _ => None,
                    }
                    .map(|change| edit(change))
                })
        }));
        self.handlers.on_edit = Some(on_edit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> PolygonDraft {
        PolygonDraft::from_ring([
            LatLon::new(-36.15, -71.83),
            LatLon::new(-36.15, -71.82),
            LatLon::new(-36.14, -71.82),
            LatLon::new(-36.14, -71.83),
        ])
        .unwrap()
    }

    #[test]
    fn edits_move_insert_and_remove_down_to_three_vertices() {
        let mut draft = square();
        let to = LatLon::new(-36.155, -71.835);
        assert!(draft.apply(PolygonEdit::Move {
            index: 0,
            to,
            first: true
        }));
        assert_eq!(draft.ring()[0], to);
        let middle = draft.midpoint(1);
        assert!(draft.apply(PolygonEdit::Insert {
            after: 1,
            at: middle
        }));
        assert_eq!(draft.ring().len(), 5);
        assert_eq!(draft.ring()[2], middle);
        assert!(draft.apply(PolygonEdit::Remove { index: 2 }));
        assert!(draft.apply(PolygonEdit::Remove { index: 3 }));
        assert_eq!(draft.ring().len(), 3);
        assert!(
            !draft.apply(PolygonEdit::Remove { index: 0 }),
            "three is the least"
        );
        assert!(!draft.apply(PolygonEdit::Remove { index: 7 }));
        assert!(!draft.apply(PolygonEdit::Move {
            index: 9,
            to,
            first: true
        }));
        assert!(!draft.apply(PolygonEdit::Finish));
        assert!(!draft.apply(PolygonEdit::Cancel));
    }

    #[test]
    fn undo_takes_back_a_whole_drag_at_a_time() {
        let mut draft = square();
        let start = draft.ring().to_vec();
        assert!(!draft.can_undo());
        for (step, first) in [(0.001, true), (0.002, false), (0.003, false)] {
            draft.apply(PolygonEdit::Move {
                index: 2,
                to: LatLon::new(-36.14 + step, -71.82),
                first,
            });
        }
        let dragged = draft.ring().to_vec();
        draft.apply(PolygonEdit::Remove { index: 0 });
        assert!(draft.apply(PolygonEdit::Undo));
        assert_eq!(draft.ring(), dragged.as_slice());
        assert!(draft.apply(PolygonEdit::Undo));
        assert_eq!(draft.ring(), start.as_slice(), "one step for the drag");
        assert!(!draft.apply(PolygonEdit::Undo));
    }

    #[test]
    fn a_bow_tie_crosses_and_is_not_valid() {
        let mut draft = square();
        assert_eq!(draft.crossing(), None);
        assert!(draft.is_valid());
        let (a, b) = (draft.ring()[1], draft.ring()[2]);
        draft.apply(PolygonEdit::Move {
            index: 1,
            to: b,
            first: true,
        });
        draft.apply(PolygonEdit::Move {
            index: 2,
            to: a,
            first: true,
        });
        assert_eq!(draft.crossing(), Some((0, 2)));
        assert!(!draft.is_valid());
        let triangle = PolygonDraft::from_ring(square().ring()[..3].to_vec()).unwrap();
        assert!(triangle.is_valid(), "a triangle never crosses itself");
    }

    #[test]
    fn the_area_of_a_sector_is_about_192_hectares() {
        let (lat, lon) = (-36.1415, -71.8265);
        let sector = PolygonDraft::from_ring([
            LatLon::new(lat - 0.006, lon - 0.008),
            LatLon::new(lat - 0.006, lon + 0.008),
            LatLon::new(lat + 0.006, lon + 0.008),
            LatLon::new(lat + 0.006, lon - 0.008),
        ])
        .unwrap();
        let width = 0.016_f64.to_radians() * EARTH_RADIUS * lat.to_radians().cos();
        let height = 0.012_f64.to_radians() * EARTH_RADIUS;
        let expected = width * height / 10_000.0;
        assert!(
            (sector.area_ha() - expected).abs() < expected * 0.001,
            "{} vs {expected}",
            sector.area_ha()
        );
        assert!((expected - 192.0).abs() < 2.0, "{expected}");
    }

    #[test]
    fn drafts_go_to_and_from_geojson() {
        let draft = square();
        let geometry = draft.to_geometry();
        let GeometryValue::Polygon { coordinates } = &geometry.value else {
            panic!("{geometry:?}");
        };
        assert_eq!(coordinates[0].len(), 5, "closed");
        assert_eq!(coordinates[0][0], coordinates[0][4]);
        assert_eq!(PolygonDraft::from_geometry(&geometry).unwrap(), draft);
        let donut = Geometry::new(GeometryValue::Polygon {
            coordinates: vec![coordinates[0].clone(), coordinates[0].clone()],
        });
        assert!(PolygonDraft::from_geometry(&donut).is_err());
        let point = Geometry::new(GeometryValue::Point {
            coordinates: Position::from(vec![0.0, 0.0]),
        });
        assert!(PolygonDraft::from_geometry(&point).is_err());
        assert!(PolygonDraft::from_ring([LatLon::new(0.0, 0.0), LatLon::new(1.0, 1.0)]).is_err());
        assert!(
            PolygonDraft::from_ring([
                LatLon::new(0.0, 0.0),
                LatLon::new(1.0, f64::INFINITY),
                LatLon::new(1.0, 0.0)
            ])
            .is_err()
        );
    }
}
