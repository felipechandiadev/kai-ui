//! Map tiles in the [Mapbox Vector Tile] format, decoded with `mvt-reader`
//! and turned into meshes: one per [`Bucket`] (a style layer's share of
//! the tile), in tile units (`0..EXTENT`, `y` down). Runs on the task
//! pool.
//!
//! [Mapbox Vector Tile]: https://github.com/mapbox/vector-tile-spec

use std::sync::Arc;

use geo_types::{Coord, Geometry, LineString, Polygon};
use mvt_reader::Reader;
use mvt_reader::feature::Value;

use crate::geometry::Point;
use crate::render::Mesh;
use crate::render::geometry::{MeshBuilder, StrokeStyle};

/// Side of a tile in its own units; layers with another extent are scaled
/// to it.
pub const EXTENT: f32 = 4096.0;
/// Tile units per screen pixel when a tile is shown about 512 points wide.
const PIXEL: f32 = EXTENT / 512.0;

/// How a bucket draws its features.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Polygons, filled (even-odd), clipped to the tile.
    Fill,
    /// Lines, stroked (polygons are left out).
    Line,
}

/// What one style layer takes from a tile: the features of source layer
/// `source` whose `kind` is one of `kinds` (any when empty), from data
/// zoom `min_zoom` on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bucket {
    pub source: &'static str,
    pub kinds: &'static [&'static str],
    pub shape: Shape,
    pub min_zoom: u8,
}

impl Bucket {
    pub const fn fill(source: &'static str, kinds: &'static [&'static str]) -> Self {
        Self {
            source,
            kinds,
            shape: Shape::Fill,
            min_zoom: 0,
        }
    }

    pub const fn line(source: &'static str, kinds: &'static [&'static str]) -> Self {
        Self {
            source,
            kinds,
            shape: Shape::Line,
            min_zoom: 0,
        }
    }

    pub const fn from_zoom(mut self, min_zoom: u8) -> Self {
        self.min_zoom = min_zoom;
        self
    }

    fn takes(&self, kind: &str) -> bool {
        self.kinds.is_empty() || self.kinds.contains(&kind)
    }
}

/// A named point of a tile, for labels: a place, or the middle of the
/// longest stretch of a named street.
#[derive(Clone, Debug, PartialEq)]
pub struct TileLabel {
    pub source: &'static str,
    pub kind: String,
    pub name: String,
    /// In tile units.
    pub at: Point,
    /// Places: lower is more important. Streets: their length in tile
    /// units, negated (longer first).
    pub rank: f32,
}

/// What a tile draws: a mesh per bucket that had features, and labels.
#[derive(Debug, Default)]
pub struct TileMeshes {
    /// Bucket index and its mesh, in bucket order.
    pub meshes: Vec<(usize, Arc<Mesh>)>,
    pub labels: Vec<TileLabel>,
}

impl TileMeshes {
    /// GPU memory of every mesh.
    pub fn byte_size(&self) -> usize {
        self.meshes.iter().map(|(_, mesh)| mesh.byte_size()).sum()
    }

    pub fn mesh(&self, bucket: usize) -> Option<&Arc<Mesh>> {
        self.meshes
            .iter()
            .find(|(index, _)| *index == bucket)
            .map(|(_, mesh)| mesh)
    }
}

fn text(value: Option<&Value>) -> Option<&str> {
    match value? {
        Value::String(text) => Some(text),
        _ => None,
    }
}

fn number(value: Option<&Value>) -> Option<f32> {
    Some(match value? {
        Value::Float(n) => *n,
        Value::Double(n) => *n as f32,
        Value::Int(n) | Value::SInt(n) => *n as f32,
        Value::UInt(n) => *n as f32,
        _ => return None,
    })
}

/// Clip a closed ring to the tile square (Sutherland–Hodgman); edges that
/// run along the border are harmless for even-odd fills.
fn clip_ring(ring: &[Point]) -> Vec<Point> {
    #[derive(Clone, Copy)]
    enum Edge {
        Left,
        Right,
        Top,
        Bottom,
    }
    let inside = |p: Point, edge: Edge| match edge {
        Edge::Left => p.x >= 0.0,
        Edge::Right => p.x <= EXTENT,
        Edge::Top => p.y >= 0.0,
        Edge::Bottom => p.y <= EXTENT,
    };
    let cross = |a: Point, b: Point, edge: Edge| {
        let t = match edge {
            Edge::Left => (0.0 - a.x) / (b.x - a.x),
            Edge::Right => (EXTENT - a.x) / (b.x - a.x),
            Edge::Top => (0.0 - a.y) / (b.y - a.y),
            Edge::Bottom => (EXTENT - a.y) / (b.y - a.y),
        };
        Point {
            x: a.x + (b.x - a.x) * t,
            y: a.y + (b.y - a.y) * t,
        }
    };
    let mut points = ring.to_vec();
    for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
        let Some(&last) = points.last() else {
            break;
        };
        let mut out = Vec::with_capacity(points.len() + 4);
        let mut previous = last;
        for &current in &points {
            match (inside(previous, edge), inside(current, edge)) {
                (true, true) => out.push(current),
                (true, false) => out.push(cross(previous, current, edge)),
                (false, true) => {
                    out.push(cross(previous, current, edge));
                    out.push(current);
                }
                (false, false) => {}
            }
            previous = current;
        }
        points = out;
    }
    points
}

fn points(line: &LineString<f32>, scale: f32) -> Vec<Point> {
    line.0
        .iter()
        .map(|Coord { x, y }| Point {
            x: x * scale,
            y: y * scale,
        })
        .collect()
}

fn polygons(geometry: &Geometry<f32>) -> Vec<&Polygon<f32>> {
    match geometry {
        Geometry::Polygon(polygon) => vec![polygon],
        Geometry::MultiPolygon(many) => many.0.iter().collect(),
        _ => Vec::new(),
    }
}

fn lines(geometry: &Geometry<f32>) -> Vec<&LineString<f32>> {
    match geometry {
        Geometry::LineString(line) => vec![line],
        Geometry::MultiLineString(many) => many.0.iter().collect(),
        _ => Vec::new(),
    }
}

/// The middle of the longest segment run of `line` and its length.
fn line_middle(line: &[Point]) -> Option<(Point, f32)> {
    let length: f32 = line
        .windows(2)
        .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y))
        .sum();
    if length <= 0.0 {
        return None;
    }
    let mut left = length / 2.0;
    for pair in line.windows(2) {
        let step = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
        if step >= left && step > 0.0 {
            let t = left / step;
            return Some((
                Point {
                    x: pair[0].x + (pair[1].x - pair[0].x) * t,
                    y: pair[0].y + (pair[1].y - pair[0].y) * t,
                },
                length,
            ));
        }
        left -= step;
    }
    None
}

fn inside_tile(p: Point) -> bool {
    (0.0..EXTENT).contains(&p.x) && (0.0..EXTENT).contains(&p.y)
}

/// Labels of the `places` layer (points) and of named `roads` (lines),
/// only those that fall inside the tile (its buffer repeats neighbors').
fn labels(
    source: &'static str,
    features: &[mvt_reader::feature::Feature],
    scale: f32,
) -> Vec<TileLabel> {
    let mut out = Vec::new();
    for feature in features {
        let Some(properties) = &feature.properties else {
            continue;
        };
        let Some(name) = text(properties.get("name")) else {
            continue;
        };
        let kind = text(properties.get("kind")).unwrap_or_default().to_owned();
        let point = match &feature.geometry {
            Geometry::Point(point) => Some(*point),
            Geometry::MultiPoint(many) => many.0.first().copied(),
            _ => None,
        };
        let found = match (&feature.geometry, source) {
            (_, "places") => point.map(|point| {
                (
                    Point {
                        x: point.x() * scale,
                        y: point.y() * scale,
                    },
                    number(properties.get("min_zoom")).unwrap_or(99.0),
                )
            }),
            (geometry, "roads") => lines(geometry)
                .into_iter()
                .filter_map(|line| line_middle(&points(line, scale)))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(at, length)| (at, -length)),
            _ => None,
        };
        if let Some((at, rank)) = found.filter(|(at, _)| inside_tile(*at)) {
            out.push(TileLabel {
                source,
                kind,
                name: name.to_owned(),
                at,
                rank,
            });
        }
    }
    out
}

/// Decode tile `bytes` (already decompressed) of data zoom `z` and build
/// the meshes of `buckets` and the labels.
pub fn tessellate(bytes: Vec<u8>, z: u8, buckets: &[Bucket]) -> Result<TileMeshes, String> {
    let reader = Reader::new(bytes).map_err(|error| format!("MVT inválido: {error}"))?;
    let layers = reader
        .get_layer_metadata()
        .map_err(|error| format!("MVT inválido: {error}"))?;
    let mut builders: Vec<Option<MeshBuilder>> = buckets.iter().map(|_| None).collect();
    let mut out = TileMeshes::default();
    for layer in &layers {
        let wanted: Vec<usize> = buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| bucket.source == layer.name && z >= bucket.min_zoom)
            .map(|(index, _)| index)
            .collect();
        let label_source = ["places", "roads"]
            .into_iter()
            .find(|source| *source == layer.name);
        if wanted.is_empty() && label_source.is_none() {
            continue;
        }
        let features = reader
            .get_features(layer.layer_index)
            .map_err(|error| format!("MVT inválido en {}: {error}", layer.name))?;
        let scale = if layer.extent > 0 {
            EXTENT / layer.extent as f32
        } else {
            1.0
        };
        if let Some(source) = label_source {
            out.labels.extend(labels(source, &features, scale));
        }
        for feature in &features {
            let kind = feature
                .properties
                .as_ref()
                .and_then(|properties| text(properties.get("kind")))
                .unwrap_or_default();
            for &index in &wanted {
                let bucket = &buckets[index];
                if !bucket.takes(kind) {
                    continue;
                }
                let builder = builders[index].get_or_insert_with(|| MeshBuilder::new(PIXEL));
                match bucket.shape {
                    Shape::Fill => {
                        for polygon in polygons(&feature.geometry) {
                            let rings: Vec<Vec<Point>> = std::iter::once(polygon.exterior())
                                .chain(polygon.interiors())
                                .map(|ring| clip_ring(&points(ring, scale)))
                                .filter(|ring| ring.len() >= 3)
                                .collect();
                            if !rings.is_empty() {
                                builder.fill(&rings);
                            }
                        }
                    }
                    Shape::Line => {
                        for line in lines(&feature.geometry) {
                            builder.stroke(&points(line, scale), false, StrokeStyle::ROUND);
                        }
                    }
                }
            }
        }
    }
    out.meshes = builders
        .into_iter()
        .enumerate()
        .filter_map(|(index, builder)| {
            let builder = builder?;
            (!builder.is_empty()).then(|| (index, Arc::new(builder.build())))
        })
        .collect();
    Ok(out)
}

/// A minimal MVT encoder for tests.
#[cfg(test)]
pub(crate) mod encode {
    /// Geometry of a test feature, in tile units.
    pub enum Geom {
        Point(i32, i32),
        Line(Vec<(i32, i32)>),
        /// Rings; the outer one with positive area (clockwise on screen).
        Polygon(Vec<Vec<(i32, i32)>>),
    }

    pub struct Feature {
        pub kind: &'static str,
        pub name: Option<&'static str>,
        pub geom: Geom,
    }

    fn varint(out: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            out.push((value as u8) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }

    fn key(out: &mut Vec<u8>, field: u32, wire: u32) {
        varint(out, u64::from((field << 3) | wire));
    }

    fn bytes(out: &mut Vec<u8>, field: u32, data: &[u8]) {
        key(out, field, 2);
        varint(out, data.len() as u64);
        out.extend_from_slice(data);
    }

    fn packed(out: &mut Vec<u8>, field: u32, values: &[u32]) {
        let mut data = Vec::new();
        for value in values {
            varint(&mut data, u64::from(*value));
        }
        bytes(out, field, &data);
    }

    fn zigzag(n: i32) -> u32 {
        ((n << 1) ^ (n >> 31)) as u32
    }

    fn command(id: u32, count: u32) -> u32 {
        (id & 7) | (count << 3)
    }

    fn path(out: &mut Vec<u32>, cursor: &mut (i32, i32), points: &[(i32, i32)], close: bool) {
        let move_to = |out: &mut Vec<u32>, cursor: &mut (i32, i32), p: (i32, i32)| {
            out.push(zigzag(p.0 - cursor.0));
            out.push(zigzag(p.1 - cursor.1));
            *cursor = p;
        };
        out.push(command(1, 1));
        move_to(out, cursor, points[0]);
        out.push(command(2, points.len() as u32 - 1));
        for p in &points[1..] {
            move_to(out, cursor, *p);
        }
        if close {
            out.push(command(7, 1));
        }
    }

    /// A tile with `layers` of features (extent 4096).
    pub fn tile(layers: &[(&str, Vec<Feature>)]) -> Vec<u8> {
        let mut tile = Vec::new();
        for (name, features) in layers {
            let mut layer = Vec::new();
            key(&mut layer, 15, 0);
            varint(&mut layer, 2);
            bytes(&mut layer, 1, name.as_bytes());
            let mut values: Vec<&str> = Vec::new();
            let value_index = |text: &'static str, values: &mut Vec<&str>| {
                values.iter().position(|v| *v == text).unwrap_or_else(|| {
                    values.push(text);
                    values.len() - 1
                }) as u32
            };
            for feature in features {
                let mut tags = vec![0, value_index(feature.kind, &mut values)];
                if let Some(name) = feature.name {
                    tags.extend([1, value_index(name, &mut values)]);
                }
                let mut geometry = Vec::new();
                let mut cursor = (0, 0);
                let kind = match &feature.geom {
                    Geom::Point(x, y) => {
                        geometry.push(command(1, 1));
                        geometry.extend([zigzag(*x), zigzag(*y)]);
                        1
                    }
                    Geom::Line(points) => {
                        path(&mut geometry, &mut cursor, points, false);
                        2
                    }
                    Geom::Polygon(rings) => {
                        for ring in rings {
                            path(&mut geometry, &mut cursor, ring, true);
                        }
                        3
                    }
                };
                let mut encoded = Vec::new();
                packed(&mut encoded, 2, &tags);
                key(&mut encoded, 3, 0);
                varint(&mut encoded, kind);
                packed(&mut encoded, 4, &geometry);
                bytes(&mut layer, 2, &encoded);
            }
            for key_name in ["kind", "name"] {
                bytes(&mut layer, 3, key_name.as_bytes());
            }
            for value in values {
                let mut encoded = Vec::new();
                bytes(&mut encoded, 1, value.as_bytes());
                bytes(&mut layer, 4, &encoded);
            }
            key(&mut layer, 5, 0);
            varint(&mut layer, 4096);
            bytes(&mut tile, 3, &layer);
        }
        tile
    }

    /// A square ring from `(x, y)` with side `side`, positive area.
    pub fn square(x: i32, y: i32, side: i32) -> Vec<(i32, i32)> {
        vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)]
    }
}

#[cfg(test)]
mod tests {
    use super::encode::{Feature, Geom, square, tile};
    use super::*;
    use crate::geo::TileId;
    use crate::maps::pmtiles::PmTiles;

    const BUCKETS: [Bucket; 4] = [
        Bucket::fill("earth", &[]),
        Bucket::fill("water", &["water", "lake"]),
        Bucket::line("roads", &["major_road"]),
        Bucket::line("roads", &["minor_road"]).from_zoom(14),
    ];

    fn sample() -> Vec<u8> {
        tile(&[
            (
                "earth",
                vec![Feature {
                    kind: "earth",
                    name: None,
                    // Spills into the buffer on every side.
                    geom: Geom::Polygon(vec![square(-64, -64, 4096 + 128)]),
                }],
            ),
            (
                "water",
                vec![Feature {
                    kind: "river",
                    name: None,
                    geom: Geom::Polygon(vec![square(100, 100, 50)]),
                }],
            ),
            (
                "roads",
                vec![
                    Feature {
                        kind: "major_road",
                        name: Some("Avenida Aníbal Pinto"),
                        geom: Geom::Line(vec![(0, 2000), (1000, 2000), (3000, 2000)]),
                    },
                    Feature {
                        kind: "minor_road",
                        name: Some("Dieciocho"),
                        geom: Geom::Line(vec![(2000, 0), (2000, 1000)]),
                    },
                ],
            ),
            (
                "places",
                vec![Feature {
                    kind: "locality",
                    name: Some("Parral"),
                    geom: Geom::Point(2048, 2048),
                }],
            ),
        ])
    }

    #[test]
    fn buckets_take_their_layer_kinds_and_zooms() {
        let meshes = tessellate(sample(), 13, &BUCKETS).unwrap();
        let indices: Vec<usize> = meshes.meshes.iter().map(|(index, _)| *index).collect();
        assert_eq!(
            indices,
            [0, 2],
            "a river is not a lake; minor roads from 14"
        );
        let earth = meshes.mesh(0).unwrap().bounds().unwrap();
        assert_eq!(
            (earth.x, earth.y, earth.width, earth.height),
            (0.0, 0.0, EXTENT, EXTENT),
            "fills are clipped to the tile"
        );
        let road = meshes.mesh(2).unwrap();
        assert!(road.vertices().iter().any(|v| v.normal != [0.0, 0.0]));
        assert_eq!(road.bounds().unwrap().y, 2000.0, "strokes keep their path");

        let meshes = tessellate(sample(), 14, &BUCKETS).unwrap();
        assert!(meshes.mesh(3).is_some());
    }

    #[test]
    fn places_and_named_streets_become_labels() {
        let meshes = tessellate(sample(), 14, &BUCKETS).unwrap();
        let names: Vec<(&str, &str, Point)> = meshes
            .labels
            .iter()
            .map(|label| (label.source, label.name.as_str(), label.at))
            .collect();
        assert_eq!(
            names,
            [
                (
                    "roads",
                    "Avenida Aníbal Pinto",
                    Point {
                        x: 1500.0,
                        y: 2000.0
                    }
                ),
                (
                    "roads",
                    "Dieciocho",
                    Point {
                        x: 2000.0,
                        y: 500.0
                    }
                ),
                (
                    "places",
                    "Parral",
                    Point {
                        x: 2048.0,
                        y: 2048.0
                    }
                ),
            ]
        );
    }

    #[test]
    fn rings_clip_to_the_tile_square() {
        let ring = clip_ring(&[
            Point { x: -10.0, y: 10.0 },
            Point { x: 10.0, y: 10.0 },
            Point { x: 10.0, y: 20.0 },
            Point { x: -10.0, y: 20.0 },
        ]);
        assert!(ring.iter().all(|p| p.x >= 0.0));
        assert_eq!(ring.len(), 4);
        assert!(
            clip_ring(&[
                Point { x: -10.0, y: 10.0 },
                Point { x: -5.0, y: 10.0 },
                Point { x: -5.0, y: 20.0 },
            ])
            .is_empty()
        );
    }

    #[test]
    fn a_parral_tile_has_streets_and_buildings() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/maps/parral.pmtiles");
        let archive = PmTiles::open(&path).unwrap();
        let bytes = archive.tile(TileId::new(15, 9846, 19916)).unwrap().unwrap();
        let buckets = [
            Bucket::fill("earth", &[]),
            Bucket::fill("buildings", &[]),
            Bucket::line("roads", &[]),
        ];
        let meshes = tessellate(bytes, 15, &buckets).unwrap();
        assert_eq!(meshes.meshes.len(), 3, "earth, buildings and roads");
        assert!(meshes.byte_size() > 10_000);
        assert!(
            meshes.labels.iter().any(|label| label.source == "roads"),
            "named streets"
        );
    }
}
