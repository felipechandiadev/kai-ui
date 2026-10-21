//! Map labels: names of the app's layers (at their centroid, in their
//! color), and from the basemap names of towns and neighbourhoods
//! (`places`) and of streets (`roads`, horizontal, over the middle of
//! their longest stretch).
//!
//! A collision grid decides which fit, layers first, then the most
//! important basemap names. It runs when
//! the map is still and its view or tiles changed; during a gesture the
//! placed labels keep their spot on the map and move with it. New labels
//! fade in over [`FADE`]; those placed before stay as they are.

use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cosmic_text::Weight;

use super::MapSpec;
use super::layers::MapLayer;
use super::mvt::EXTENT;
use super::style::Palette;
use super::tiles::TileFrame;
use crate::element::WidgetId;
use crate::geo::{Camera, WorldPoint, project};
use crate::geometry::{Point, Rect, Size};
use crate::layout::TextMeasure;
use crate::render::text::TextStyle;
use crate::render::{Color, DrawList, TextId};
use crate::theme::Theme;

/// How long a new label takes to appear.
pub const FADE: Duration = Duration::from_millis(150);
/// Side of a collision grid cell, in points.
const CELL: f32 = 64.0;
/// Free space kept around every label.
const GAP: f32 = 6.0;
/// Width of the halo around label text.
const HALO: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LabelKind {
    /// The name of a [`MapLayer`].
    Area,
    /// A city, town or village.
    Locality,
    Neighbourhood,
    Street,
}

impl LabelKind {
    pub fn style(self) -> TextStyle {
        let (font_size, line_height, weight) = match self {
            Self::Area => (12.0, 15.0, Weight::SEMIBOLD),
            Self::Locality => (15.0, 18.0, Weight::SEMIBOLD),
            Self::Neighbourhood => (11.0, 14.0, Weight::MEDIUM),
            Self::Street => (11.0, 14.0, Weight::NORMAL),
        };
        TextStyle {
            font_size,
            line_height,
            weight,
            wrap: false,
            letter_spacing: 0.0,
        }
    }
}

/// A label to draw this frame, centered on `at`.
#[derive(Clone, Debug, PartialEq)]
pub struct MapLabel {
    pub text: TextId,
    pub kind: LabelKind,
    pub name: Arc<str>,
    pub at: WorldPoint,
    pub size: Size,
    /// 0 to 1 while fading in.
    pub opacity: f32,
    /// Text color of a layer's name; basemap names take the palette's.
    pub color: Option<Color>,
}

/// Labels compare by value and never hold NaN.
impl Eq for MapLabel {}

/// A name the tiles or layers offer, with its priority: lower `class`
/// first, then lower `rank`.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub kind: LabelKind,
    pub name: Arc<str>,
    pub at: WorldPoint,
    pub class: u8,
    pub rank: f32,
    pub color: Option<Color>,
}

/// The names of `layers`, bottom to top: they go before any basemap name.
fn layer_candidates(layers: &[MapLayer]) -> Vec<Candidate> {
    layers
        .iter()
        .enumerate()
        .filter_map(|(index, layer)| {
            Some(Candidate {
                kind: LabelKind::Area,
                name: Arc::from(layer.label.as_deref()?),
                at: project(layer.centroid()),
                class: 0,
                rank: index as f32,
                color: layer.ink(),
            })
        })
        .collect()
}

/// The names of the drawn tiles worth showing at `zoom`, most important
/// first, in a fixed order.
pub fn candidates(tiles: &TileFrame, zoom: f64) -> Vec<Candidate> {
    let mut out = Vec::new();
    for placed in tiles.tiles.iter().filter(|placed| placed.clip.is_none()) {
        let (corner, side) = placed.tile.world_origin();
        for label in &placed.meshes.labels {
            let (kind, class) = match (label.source, label.kind.as_str()) {
                ("places", "locality" | "city" | "town" | "village") => (LabelKind::Locality, 1),
                ("places", "neighbourhood" | "suburb" | "quarter" | "macrohood" | "microhood") => {
                    (LabelKind::Neighbourhood, 2)
                }
                ("roads", "highway" | "major_road") if zoom >= 14.0 => (LabelKind::Street, 3),
                ("roads", "minor_road") if zoom >= 15.0 => (LabelKind::Street, 4),
                ("roads", "other") if zoom >= 16.0 => (LabelKind::Street, 5),
                _ => continue,
            };
            // Protomaps zooms count 512-point tiles: one less than ours.
            if label.source == "places" && f64::from(label.rank) + 1.0 > zoom {
                continue;
            }
            out.push(Candidate {
                kind,
                name: Arc::from(label.name.as_str()),
                at: WorldPoint {
                    x: corner.x + f64::from(label.at.x / EXTENT) * side,
                    y: corner.y + f64::from(label.at.y / EXTENT) * side,
                },
                class,
                rank: label.rank,
                color: None,
            });
        }
    }
    out.sort_by(|a, b| {
        a.class
            .cmp(&b.class)
            .then(a.rank.total_cmp(&b.rank))
            .then_with(|| a.name.cmp(&b.name))
            .then(a.at.x.total_cmp(&b.at.x))
            .then(a.at.y.total_cmp(&b.at.y))
    });
    out
}

/// Boxes already taken, bucketed in `CELL`-point cells.
pub struct Grid {
    bounds: Rect,
    columns: usize,
    rows: usize,
    cells: Vec<Vec<Rect>>,
}

impl Grid {
    pub fn new(bounds: Rect) -> Self {
        let columns = (bounds.width / CELL).ceil().max(1.0) as usize;
        let rows = (bounds.height / CELL).ceil().max(1.0) as usize;
        Self {
            bounds,
            columns,
            rows,
            cells: vec![Vec::new(); columns * rows],
        }
    }

    fn cells_of(&self, rect: Rect) -> impl Iterator<Item = usize> + use<> {
        let index = |value: f32, origin: f32, count: usize| {
            (((value - origin) / CELL).floor().max(0.0) as usize).min(count - 1)
        };
        let (left, right) = (
            index(rect.x, self.bounds.x, self.columns),
            index(rect.x + rect.width, self.bounds.x, self.columns),
        );
        let (top, bottom) = (
            index(rect.y, self.bounds.y, self.rows),
            index(rect.y + rect.height, self.bounds.y, self.rows),
        );
        let columns = self.columns;
        (top..=bottom).flat_map(move |row| (left..=right).map(move |column| row * columns + column))
    }

    /// Whether `rect` lies inside the bounds and clear of every box taken.
    pub fn fits(&self, rect: Rect) -> bool {
        let b = self.bounds;
        let inside = rect.x >= b.x
            && rect.y >= b.y
            && rect.x + rect.width <= b.x + b.width
            && rect.y + rect.height <= b.y + b.height;
        inside
            && self.cells_of(rect).all(|cell| {
                self.cells[cell]
                    .iter()
                    .all(|taken| taken.intersection(rect).is_none())
            })
    }

    pub fn take(&mut self, rect: Rect) {
        for cell in self.cells_of(rect) {
            self.cells[cell].push(rect);
        }
    }
}

fn key(kind: LabelKind, name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    name.hash(&mut hasher);
    hasher.finish()
}

#[derive(Clone, Debug)]
struct Placed {
    key: u64,
    kind: LabelKind,
    name: Arc<str>,
    at: WorldPoint,
    size: Size,
    since: Instant,
    color: Option<Color>,
}

/// The labels of one map.
#[derive(Debug, Default)]
pub struct Labels {
    signature: Option<u64>,
    placed: Vec<Placed>,
}

fn signature(camera: &Camera, tiles: &TileFrame, layers: &[Candidate]) -> u64 {
    let mut hasher = DefaultHasher::new();
    camera.view.center.lat.to_bits().hash(&mut hasher);
    camera.view.center.lon.to_bits().hash(&mut hasher);
    camera.view.zoom.to_bits().hash(&mut hasher);
    camera.size.width.to_bits().hash(&mut hasher);
    camera.size.height.to_bits().hash(&mut hasher);
    for placed in &tiles.tiles {
        placed.tile.hash(&mut hasher);
        placed.clip.hash(&mut hasher);
        (Arc::as_ptr(&placed.meshes) as usize).hash(&mut hasher);
    }
    for candidate in layers {
        candidate.name.hash(&mut hasher);
        candidate.at.x.to_bits().hash(&mut hasher);
        candidate.at.y.to_bits().hash(&mut hasher);
        candidate.color.hash(&mut hasher);
    }
    hasher.finish()
}

impl Labels {
    /// Place the labels of map `map` again if it is `still` and its view,
    /// tiles or layers changed. `measure` shapes their text (and keeps it
    /// ready to draw).
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        map: WidgetId,
        camera: Camera,
        tiles: &TileFrame,
        layers: &[MapLayer],
        still: bool,
        now: Instant,
        measure: &mut dyn TextMeasure,
    ) {
        if !still {
            return;
        }
        let mut wanted = layer_candidates(layers);
        let signature = signature(&camera, tiles, &wanted);
        if self.signature == Some(signature) {
            return;
        }
        self.signature = Some(signature);
        let mut grid = Grid::new(Rect {
            x: 0.0,
            y: 0.0,
            width: camera.size.width,
            height: camera.size.height,
        });
        let mut streets: HashSet<Arc<str>> = HashSet::new();
        let mut placed = Vec::new();
        wanted.extend(candidates(tiles, camera.view.zoom));
        for candidate in wanted {
            if candidate.kind == LabelKind::Street && streets.contains(&candidate.name) {
                continue;
            }
            let key = key(candidate.kind, &candidate.name);
            let size = measure.measure(
                TextId::map_label(map, key),
                &candidate.name,
                candidate.kind.style(),
                None,
            );
            let (x, y) = camera.world_to_local(candidate.at);
            let rect = Rect {
                x: x as f32 - size.width / 2.0 - GAP,
                y: y as f32 - size.height / 2.0 - GAP,
                width: size.width + 2.0 * GAP,
                height: size.height + 2.0 * GAP,
            };
            if !grid.fits(rect) {
                continue;
            }
            grid.take(rect);
            if candidate.kind == LabelKind::Street {
                streets.insert(Arc::clone(&candidate.name));
            }
            let since = self
                .placed
                .iter()
                .find(|old| old.key == key)
                .map_or(now, |old| old.since);
            placed.push(Placed {
                key,
                kind: candidate.kind,
                name: candidate.name,
                at: candidate.at,
                size,
                since,
                color: candidate.color,
            });
        }
        self.placed = placed;
    }

    /// The labels to draw at `now`, and whether some is still fading in.
    pub fn frame(&self, map: WidgetId, now: Instant) -> (Vec<MapLabel>, bool) {
        let mut fading = false;
        let labels = self
            .placed
            .iter()
            .map(|placed| {
                let shown = now.saturating_duration_since(placed.since);
                let opacity = (shown.as_secs_f32() / FADE.as_secs_f32()).min(1.0);
                fading |= opacity < 1.0;
                MapLabel {
                    text: TextId::map_label(map, placed.key),
                    kind: placed.kind,
                    name: Arc::clone(&placed.name),
                    at: placed.at,
                    size: placed.size,
                    opacity,
                    color: placed.color,
                }
            })
            .collect();
        (labels, fading)
    }
}

/// Where `label` goes on a map showing `camera` over `area`: the top left
/// corner of its text, on whole points so glyphs stay crisp.
pub fn origin(label: &MapLabel, camera: &Camera, area: Rect) -> Point {
    let (x, y) = camera.world_to_local(label.at);
    Point {
        x: (f64::from(area.x) + x - f64::from(label.size.width) / 2.0).round() as f32,
        y: (f64::from(area.y) + y - f64::from(label.size.height) / 2.0).round() as f32,
    }
}

/// Paint the labels of `spec` over `area`, each with a halo, clipped to
/// the map.
pub fn paint(list: &mut DrawList, area: Rect, spec: &MapSpec, theme: &Theme) {
    if spec.labels.is_empty() || area.width <= 0.0 || area.height <= 0.0 {
        return;
    }
    let camera = Camera::new(
        spec.view,
        Size {
            width: area.width,
            height: area.height,
        },
    );
    let palette = Palette::new(theme);
    list.push_clip(area);
    for label in &spec.labels {
        let at = origin(label, &camera, area);
        let rect = Rect {
            x: at.x,
            y: at.y,
            width: label.size.width,
            height: label.size.height,
        };
        if rect.intersection(area).is_none() || label.opacity <= 0.0 {
            continue;
        }
        let alpha = (255.0 * label.opacity).round() as u8;
        let halo = palette.halo.with_alpha(alpha);
        for (dx, dy) in [
            (-1.0, -1.0),
            (0.0, -1.0),
            (1.0, -1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (1.0, 1.0),
        ] {
            list.text(
                label.text,
                Point {
                    x: at.x + dx * HALO,
                    y: at.y + dy * HALO,
                },
                halo,
            );
        }
        let color = label.color.unwrap_or(match label.kind {
            LabelKind::Area | LabelKind::Locality => palette.place_text,
            LabelKind::Neighbourhood | LabelKind::Street => palette.minor_text,
        });
        list.text(label.text, at, color.with_alpha(alpha));
    }
    list.pop_clip();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::{LatLon, MapView, TileId, project};
    use crate::layout::testing::FixedMeasure;
    use crate::maps::mvt::{TileLabel, TileMeshes};
    use crate::maps::tiles::PlacedTile;

    const PLAZA: LatLon = LatLon::new(-36.1415, -71.8265);
    const SIZE: Size = Size {
        width: 600.0,
        height: 400.0,
    };
    const MAP: WidgetId = WidgetId(7);

    fn camera(zoom: f64) -> Camera {
        Camera::new(MapView::new(PLAZA, zoom), SIZE)
    }

    fn label(source: &'static str, kind: &str, name: &str, x: f32, y: f32, rank: f32) -> TileLabel {
        TileLabel {
            source,
            kind: kind.to_owned(),
            name: name.to_owned(),
            at: Point { x, y },
            rank,
        }
    }

    /// The tile under the plaza at zoom 15 with `labels`.
    fn frame(labels: Vec<TileLabel>) -> TileFrame {
        let world = project(PLAZA);
        let tile = TileId::new(15, (world.x * 32768.0) as u32, (world.y * 32768.0) as u32);
        TileFrame {
            tiles: vec![PlacedTile {
                tile,
                clip: None,
                meshes: Arc::new(TileMeshes {
                    meshes: Vec::new(),
                    labels,
                }),
            }],
            pending: false,
            revealing: false,
        }
    }

    /// A crowded tile: a town, neighbourhoods and streets on top of each
    /// other, and a street in two pieces.
    fn crowded() -> TileFrame {
        let mut labels = vec![
            label("places", "locality", "Parral", 2048.0, 2048.0, 8.0),
            label("places", "neighbourhood", "Centro", 2060.0, 2060.0, 12.0),
            label(
                "places",
                "neighbourhood",
                "Villa Real",
                2048.0,
                3000.0,
                12.0,
            ),
            label("places", "country", "Chile", 2048.0, 2048.0, 0.0),
            label(
                "roads",
                "major_road",
                "Aníbal Pinto",
                2048.0,
                2100.0,
                -900.0,
            ),
            label(
                "roads",
                "major_road",
                "Aníbal Pinto",
                3500.0,
                2100.0,
                -400.0,
            ),
            label("roads", "minor_road", "Dieciocho", 1000.0, 1000.0, -300.0),
            label("roads", "other", "Pasaje 1", 3000.0, 1000.0, -100.0),
            label("roads", "path", "Sendero", 500.0, 3500.0, -100.0),
        ];
        for i in 0..30 {
            let x = 200.0 + (i % 6) as f32 * 600.0;
            let y = 300.0 + (i / 6) as f32 * 700.0;
            labels.push(label(
                "roads",
                "minor_road",
                &format!("Calle {i}"),
                x,
                y,
                -(i as f32),
            ));
        }
        frame(labels)
    }

    fn placed(zoom: f64, tiles: &TileFrame) -> (Labels, Camera) {
        let camera = camera(zoom);
        let mut labels = Labels::default();
        labels.update(
            MAP,
            camera,
            tiles,
            &[],
            true,
            Instant::now(),
            &mut FixedMeasure,
        );
        (labels, camera)
    }

    fn boxes(labels: &Labels, camera: &Camera, now: Instant) -> Vec<(String, Rect)> {
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: SIZE.width,
            height: SIZE.height,
        };
        labels
            .frame(MAP, now)
            .0
            .iter()
            .map(|label| {
                let at = origin(label, camera, area);
                (
                    label.name.to_string(),
                    Rect {
                        x: at.x,
                        y: at.y,
                        width: label.size.width,
                        height: label.size.height,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn labels_never_overlap_and_stay_inside_the_map() {
        let tiles = crowded();
        let (labels, camera) = placed(15.0, &tiles);
        let shown = boxes(&labels, &camera, Instant::now());
        assert!(shown.len() > 5, "{shown:?}");
        for (i, (name, a)) in shown.iter().enumerate() {
            assert!(a.x >= 0.0 && a.y >= 0.0, "{name} {a:?}");
            assert!(a.x + a.width <= SIZE.width && a.y + a.height <= SIZE.height);
            for (other, b) in &shown[i + 1..] {
                assert!(a.intersection(*b).is_none(), "{name} {a:?} × {other} {b:?}");
            }
        }
        let names: Vec<&str> = shown.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names[0], "Parral", "the town goes first");
        assert!(!names.contains(&"Centro"), "under the town's name");
        assert!(!names.contains(&"Chile") && !names.contains(&"Sendero"));
        assert_eq!(
            names.iter().filter(|name| **name == "Aníbal Pinto").count(),
            1,
            "a street is named once"
        );
    }

    #[test]
    fn placement_is_deterministic_and_follows_the_zoom() {
        let tiles = crowded();
        let now = Instant::now();
        let (a, camera) = placed(15.0, &tiles);
        let (b, _) = placed(15.0, &tiles);
        assert_eq!(boxes(&a, &camera, now), boxes(&b, &camera, now));

        let mut labels = tiles.tiles[0].meshes.labels.clone();
        labels.reverse();
        let (c, _) = placed(15.0, &frame(labels));
        assert_eq!(
            boxes(&a, &camera, now),
            boxes(&c, &camera, now),
            "the tile's order does not matter"
        );

        let (far, camera) = placed(14.0, &tiles);
        let names: Vec<String> = boxes(&far, &camera, now)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert!(names.contains(&"Parral".to_owned()));
        assert!(
            candidates(&tiles, 14.0)
                .iter()
                .any(|candidate| &*candidate.name == "Aníbal Pinto"),
            "major streets from 14"
        );
        assert!(
            names.iter().all(|name| !name.starts_with("Calle")),
            "minor streets from 15: {names:?}"
        );
        let (town, camera) = placed(8.5, &tiles);
        assert!(boxes(&town, &camera, now).is_empty(), "Parral from zoom 9");
        let (town, camera) = placed(9.0, &tiles);
        assert_eq!(boxes(&town, &camera, now).len(), 1);
    }

    #[test]
    fn new_labels_fade_in_and_kept_ones_do_not() {
        let tiles = crowded();
        let start = Instant::now();
        let camera = camera(15.0);
        let mut labels = Labels::default();
        labels.update(MAP, camera, &tiles, &[], true, start, &mut FixedMeasure);
        let (shown, fading) = labels.frame(MAP, start);
        assert!(fading && shown.iter().all(|label| label.opacity == 0.0));
        let (half, _) = labels.frame(MAP, start + FADE / 2);
        assert!((half[0].opacity - 0.5).abs() < 1e-3);
        let (done, fading) = labels.frame(MAP, start + FADE);
        assert!(!fading && done.iter().all(|label| label.opacity == 1.0));

        // A small pan: the labels still in place keep showing.
        let later = start + FADE * 4;
        let moved = Camera::new(camera.panned(-40.0, 0.0), SIZE);
        labels.update(MAP, moved, &tiles, &[], true, later, &mut FixedMeasure);
        let (after, _) = labels.frame(MAP, later);
        let kept: Vec<&MapLabel> = after.iter().filter(|label| label.opacity == 1.0).collect();
        assert!(!kept.is_empty() && kept.iter().any(|label| &*label.name == "Parral"));
    }

    #[test]
    fn labels_wait_for_the_map_to_hold_still() {
        let tiles = crowded();
        let now = Instant::now();
        let camera = camera(15.0);
        let mut labels = Labels::default();
        labels.update(MAP, camera, &tiles, &[], false, now, &mut FixedMeasure);
        assert!(labels.frame(MAP, now).0.is_empty());
        labels.update(MAP, camera, &tiles, &[], true, now, &mut FixedMeasure);
        let before = labels.frame(MAP, now).0;
        assert!(!before.is_empty());
        let moved = Camera::new(camera.panned(-200.0, 0.0), SIZE);
        labels.update(MAP, moved, &tiles, &[], false, now, &mut FixedMeasure);
        assert_eq!(
            labels.frame(MAP, now).0,
            before,
            "they ride along during the gesture"
        );
    }

    #[test]
    fn layer_names_go_first_in_their_color() {
        use geojson::{Geometry, GeometryValue, Position};
        let sector = Color::hex(0xc45c26);
        let square = |d: f64| {
            let (lon, lat) = (PLAZA.lon, PLAZA.lat);
            Geometry::new(GeometryValue::Polygon {
                coordinates: vec![vec![
                    Position::from(vec![lon - d, lat - d]),
                    Position::from(vec![lon + d, lat - d]),
                    Position::from(vec![lon + d, lat + d]),
                    Position::from(vec![lon - d, lat + d]),
                ]],
            })
        };
        let layers = [MapLayer::polygon(&square(0.004))
            .unwrap()
            .stroke(sector, 2.0)
            .label("Plaza de Armas")];
        let tiles = crowded();
        let camera = camera(15.0);
        let now = Instant::now();
        let mut labels = Labels::default();
        labels.update(MAP, camera, &tiles, &layers, true, now, &mut FixedMeasure);
        let (shown, _) = labels.frame(MAP, now);
        assert_eq!(&*shown[0].name, "Plaza de Armas");
        assert_eq!(
            (shown[0].kind, shown[0].color),
            (LabelKind::Area, Some(sector))
        );
        assert!(shown[1..].iter().all(|label| label.color.is_none()));

        let renamed = [layers[0].clone().label("Centro cívico")];
        labels.update(MAP, camera, &tiles, &renamed, true, now, &mut FixedMeasure);
        assert_eq!(&*labels.frame(MAP, now).0[0].name, "Centro cívico");
    }

    #[test]
    fn the_grid_finds_overlaps_across_cells() {
        let mut grid = Grid::new(Rect {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 200.0,
        });
        let wide = Rect {
            x: 10.0,
            y: 10.0,
            width: 200.0,
            height: 20.0,
        };
        assert!(grid.fits(wide));
        grid.take(wide);
        assert!(!grid.fits(Rect {
            x: 180.0,
            y: 25.0,
            width: 40.0,
            height: 20.0,
        }));
        assert!(grid.fits(Rect {
            x: 180.0,
            y: 40.0,
            width: 40.0,
            height: 20.0,
        }));
        assert!(
            !grid.fits(Rect {
                x: 280.0,
                y: 40.0,
                width: 40.0,
                height: 20.0,
            }),
            "outside"
        );
    }
}
