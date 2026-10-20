//! Shapes the app draws over the basemap: polygons from GeoJSON (areas of
//! operation, sectors, districts) with a translucent fill, an outline and
//! a name at their centroid.
//!
//! ```ignore
//! map(&self.vista)
//!     .layers(self.areas.iter().map(|area| {
//!         area.layer.clone().fill(SECTOR, 0.12).stroke(SECTOR, 2.0).label(&area.name)
//!     }))
//!     .on_area_click(Msg::AreaElegida)
//! ```
//!
//! A shape is tessellated once, when it is built; clones share its meshes,
//! so a view can rebuild its layers every frame for free.

use std::sync::Arc;

use geojson::{Feature, GeoJson, Geometry, GeometryValue, JsonObject, Position};

use super::MapSpec;
use crate::geo::{Camera, LatLon, LatLonBounds, WorldPoint, project, unproject, world_size};
use crate::geometry::{Point, Rect, Size};
use crate::render::geometry::{Join, MeshBuilder, StrokeStyle};
use crate::render::{Affine2, Color, DrawList, GeometryDraw, Mesh};

/// Mesh units per world unit: a point is 4 units at zoom 14, so `f32`
/// keeps a shape exact from the whole country down to a single street.
const UNITS: f64 = 16_777_216.0;
/// The zoom the meshes are cut for (see [`MeshBuilder::new`]).
const MESH_ZOOM: f64 = 14.0;
/// Hovered and selected areas: fill this many times stronger, outline
/// this much wider (points).
const EMPHASIS_FILL: f32 = 2.0;
const EMPHASIS_STROKE: f32 = 1.0;

/// A polygon in world coordinates: outer ring first, then holes. Rings
/// do not repeat their first point.
type Polygon = Vec<Vec<WorldPoint>>;

#[derive(Debug)]
struct Shape {
    polygons: Vec<Polygon>,
    /// `false` for a line: an open outline, no fill and no inside.
    closed: bool,
    /// Mesh coordinates count from here.
    origin: WorldPoint,
    fill: Arc<Mesh>,
    outline: Arc<Mesh>,
    bounds: LatLonBounds,
    centroid: LatLon,
}

/// A shape drawn on a [`map`](super::map) over its tiles, under its
/// labels and markers.
#[derive(Clone, Debug)]
pub struct MapLayer {
    pub(crate) id: String,
    shape: Arc<Shape>,
    pub(crate) fill: Option<(Color, f32)>,
    pub(crate) stroke: Option<(Color, f32)>,
    pub(crate) label: Option<String>,
    pub(crate) selected: bool,
}

impl PartialEq for MapLayer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && Arc::ptr_eq(&self.shape, &other.shape)
            && self.fill == other.fill
            && self.stroke == other.stroke
            && self.label == other.label
            && self.selected == other.selected
    }
}

/// Layers compare by value; opacities and widths are never NaN.
impl Eq for MapLayer {}

fn world(position: &Position) -> Result<WorldPoint, String> {
    match position.as_slice() {
        [lon, lat, ..] if lon.is_finite() && lat.is_finite() => {
            Ok(project(LatLon::new(*lat, *lon)))
        }
        _ => Err(format!("posición inválida: {:?}", position.as_slice())),
    }
}

fn latlon_points(points: &[LatLon]) -> Result<Vec<WorldPoint>, String> {
    points
        .iter()
        .map(|at| {
            if at.lat.is_finite() && at.lon.is_finite() {
                Ok(project(*at))
            } else {
                Err(format!("posición inválida: {at:?}"))
            }
        })
        .collect()
}

fn polygon(rings: &[Vec<Position>]) -> Result<Polygon, String> {
    let mut out = Vec::with_capacity(rings.len());
    for ring in rings {
        let mut points = ring.iter().map(world).collect::<Result<Vec<_>, _>>()?;
        if points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        if points.len() < 3 {
            return Err("un anillo necesita al menos tres puntos".to_owned());
        }
        out.push(points);
    }
    if out.is_empty() {
        return Err("un polígono necesita un anillo".to_owned());
    }
    Ok(out)
}

/// Twice the signed area of `ring` and its centroid.
fn ring_moments(ring: &[WorldPoint]) -> (f64, f64, f64) {
    let (mut area, mut x, mut y) = (0.0, 0.0, 0.0);
    let origin = ring[0];
    for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
        let (ax, ay) = (a.x - origin.x, a.y - origin.y);
        let (bx, by) = (b.x - origin.x, b.y - origin.y);
        let cross = ax * by - bx * ay;
        area += cross;
        x += (ax + bx) * cross;
        y += (ay + by) * cross;
    }
    if area == 0.0 {
        return (0.0, origin.x, origin.y);
    }
    (
        area,
        origin.x + x / (3.0 * area),
        origin.y + y / (3.0 * area),
    )
}

/// The centroid of the polygons' area: outer rings add, holes take away.
fn centroid(polygons: &[Polygon]) -> WorldPoint {
    let (mut total, mut x, mut y) = (0.0, 0.0, 0.0);
    for polygon in polygons {
        for (index, ring) in polygon.iter().enumerate() {
            let (area, cx, cy) = ring_moments(ring);
            let area = if index == 0 { area.abs() } else { -area.abs() };
            total += area;
            x += cx * area;
            y += cy * area;
        }
    }
    if total.abs() < f64::EPSILON * f64::EPSILON {
        return polygons[0][0][0];
    }
    WorldPoint {
        x: x / total,
        y: y / total,
    }
}

impl Shape {
    fn new(polygons: Vec<Polygon>, closed: bool) -> Self {
        let origin = polygons[0][0][0];
        let local = |p: &WorldPoint| Point {
            x: ((p.x - origin.x) * UNITS) as f32,
            y: ((p.y - origin.y) * UNITS) as f32,
        };
        let pixel = (UNITS / world_size(MESH_ZOOM)) as f32;
        let mut fill = MeshBuilder::new(pixel);
        let mut outline = MeshBuilder::new(pixel);
        let style = StrokeStyle {
            join: Join::Round,
            ..StrokeStyle::default()
        };
        for polygon in &polygons {
            let rings: Vec<Vec<Point>> = polygon
                .iter()
                .map(|ring| ring.iter().map(local).collect())
                .collect();
            if closed {
                fill.fill(&rings);
            }
            for ring in &rings {
                outline.stroke(ring, closed, style);
            }
        }
        let bounds =
            LatLonBounds::around(polygons.iter().flatten().flatten().map(|p| unproject(*p)))
                .expect("a polygon has points");
        Self {
            centroid: unproject(centroid(&polygons)),
            polygons,
            closed,
            origin,
            fill: Arc::new(fill.build()),
            outline: Arc::new(outline.build()),
            bounds,
        }
    }

    /// Even-odd over every ring, so holes stay out.
    fn contains(&self, at: WorldPoint) -> bool {
        self.closed
            && self.polygons.iter().any(|polygon| {
                let mut inside = false;
                for ring in polygon {
                    for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                        if (a.y > at.y) != (b.y > at.y)
                            && at.x < a.x + (b.x - a.x) * (at.y - a.y) / (b.y - a.y)
                        {
                            inside = !inside;
                        }
                    }
                }
                inside
            })
    }
}

impl MapLayer {
    /// A `Polygon` or `MultiPolygon` (in longitude, latitude), without
    /// fill or outline yet.
    pub fn polygon(geometry: &Geometry) -> Result<Self, String> {
        let polygons = match &geometry.value {
            GeometryValue::Polygon { coordinates } => vec![polygon(coordinates)?],
            GeometryValue::MultiPolygon { coordinates } => coordinates
                .iter()
                .map(|rings| polygon(rings))
                .collect::<Result<_, _>>()?,
            other => return Err(format!("se esperaba un polígono: {}", other.type_name())),
        };
        if polygons.is_empty() {
            return Err("multipolígono vacío".to_owned());
        }
        Ok(Self::from_shape(Shape::new(polygons, true)))
    }

    /// A polygon with one ring of at least three points, closed or not.
    pub fn ring(points: &[LatLon]) -> Result<Self, String> {
        let mut ring = latlon_points(points)?;
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        if ring.len() < 3 {
            return Err("un anillo necesita al menos tres puntos".to_owned());
        }
        Ok(Self::from_shape(Shape::new(vec![vec![ring]], true)))
    }

    /// An open line through at least two points: an outline only, never
    /// under the pointer.
    pub fn line(points: &[LatLon]) -> Result<Self, String> {
        let line = latlon_points(points)?;
        if line.len() < 2 {
            return Err("una línea necesita al menos dos puntos".to_owned());
        }
        Ok(Self::from_shape(Shape::new(vec![vec![line]], false)))
    }

    fn from_shape(shape: Shape) -> Self {
        Self {
            id: String::new(),
            shape: Arc::new(shape),
            fill: None,
            stroke: None,
            label: None,
            selected: false,
        }
    }

    /// What [`on_area_click`](crate::element::Element::on_area_click)
    /// hands back.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Fill with `color` at `opacity` (0 to 1).
    pub fn fill(mut self, color: Color, opacity: f32) -> Self {
        self.fill = Some((color, opacity.clamp(0.0, 1.0)));
        self
    }

    /// Outline `width` points wide at any zoom.
    pub fn stroke(mut self, color: Color, width: f32) -> Self {
        self.stroke = Some((color, width.max(0.0)));
        self
    }

    /// A name shown at the centroid, in the stroke color.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Drawn emphasized, as under the pointer.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn bounds(&self) -> LatLonBounds {
        self.shape.bounds
    }

    pub fn centroid(&self) -> LatLon {
        self.shape.centroid
    }

    /// The outer ring of its first polygon (or its line), without
    /// repeating the first point.
    pub fn outer_ring(&self) -> Vec<LatLon> {
        self.shape.polygons[0][0]
            .iter()
            .map(|p| unproject(*p))
            .collect()
    }

    pub fn contains(&self, at: LatLon) -> bool {
        self.shape.contains(project(at))
    }

    pub(crate) fn contains_world(&self, at: WorldPoint) -> bool {
        self.shape.contains(at)
    }

    /// The color of the name: the outline's, else the fill's.
    pub(crate) fn ink(&self) -> Option<Color> {
        self.stroke.or(self.fill).map(|(color, _)| color)
    }
}

/// The polygons of a GeoJSON `FeatureCollection` (or a single feature),
/// each with its properties; features of other kinds are skipped.
pub fn polygons(geojson: &str) -> Result<Vec<(MapLayer, JsonObject)>, String> {
    let features: Vec<Feature> = match geojson.parse::<GeoJson>().map_err(|e| e.to_string())? {
        GeoJson::FeatureCollection(collection) => collection.features,
        GeoJson::Feature(feature) => vec![feature],
        GeoJson::Geometry(_) => return Err("se esperaba una colección de features".to_owned()),
    };
    let mut out = Vec::new();
    for feature in features {
        let Some(geometry) = &feature.geometry else {
            continue;
        };
        if !matches!(
            geometry.value,
            GeometryValue::Polygon { .. } | GeometryValue::MultiPolygon { .. }
        ) {
            continue;
        }
        out.push((
            MapLayer::polygon(geometry)?,
            feature.properties.unwrap_or_default(),
        ));
    }
    Ok(out)
}

/// The topmost of `layers` with an id under `local` (points in a map
/// showing `camera`).
pub(crate) fn hit<'a>(
    layers: &'a [MapLayer],
    camera: &Camera,
    local: Point,
) -> Option<&'a MapLayer> {
    let at = camera.local_to_world(local);
    layers
        .iter()
        .rev()
        .find(|layer| !layer.id.is_empty() && layer.contains_world(at))
}

fn emphasized(spec: &MapSpec, layer: &MapLayer) -> bool {
    layer.selected || (!layer.id.is_empty() && spec.hovered.as_deref() == Some(layer.id.as_str()))
}

fn alpha(color: Color, opacity: f32) -> Color {
    color.with_alpha((f32::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8)
}

/// Paint the layers of `spec` over `area`: every fill, then every
/// outline, emphasized ones last so nothing covers their edge.
pub fn paint(list: &mut DrawList, area: Rect, spec: &MapSpec) {
    if spec.layers.is_empty() || area.width <= 0.0 || area.height <= 0.0 {
        return;
    }
    let camera = Camera::new(
        spec.view,
        Size {
            width: area.width,
            height: area.height,
        },
    );
    let scale = (camera.scale() / UNITS) as f32;
    let place = |layer: &MapLayer| {
        let (x, y) = camera.world_to_local(layer.shape.origin);
        Affine2::scale_translate(
            scale,
            (f64::from(area.x) + x) as f32,
            (f64::from(area.y) + y) as f32,
        )
    };
    let mut draws = Vec::new();
    for layer in &spec.layers {
        if let Some((color, opacity)) = layer.fill {
            let strong = if emphasized(spec, layer) {
                opacity * EMPHASIS_FILL
            } else {
                opacity
            };
            draws.push(GeometryDraw::fill(
                Arc::clone(&layer.shape.fill),
                place(layer),
                alpha(color, strong),
            ));
        }
    }
    let mut outlines: Vec<&MapLayer> = spec.layers.iter().filter(|l| l.stroke.is_some()).collect();
    outlines.sort_by_key(|layer| emphasized(spec, layer));
    for layer in outlines {
        let Some((color, width)) = layer.stroke else {
            continue;
        };
        let width = if emphasized(spec, layer) {
            width + EMPHASIS_STROKE
        } else {
            width
        };
        draws.push(GeometryDraw::stroke(
            Arc::clone(&layer.shape.outline),
            place(layer),
            color,
            width,
        ));
    }
    list.push_clip(area);
    for draw in draws {
        if draw
            .extent()
            .is_some_and(|extent| extent.intersection(area).is_some())
        {
            list.geometry(draw);
        }
    }
    list.pop_clip();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::MapView;
    use crate::render::DrawOp;

    const SECTOR: Color = Color::hex(0xc45c26);

    fn square(lon: f64, lat: f64, d: f64) -> Geometry {
        Geometry::new(GeometryValue::Polygon {
            coordinates: vec![vec![
                Position::from(vec![lon - d, lat - d]),
                Position::from(vec![lon + d, lat - d]),
                Position::from(vec![lon + d, lat + d]),
                Position::from(vec![lon - d, lat + d]),
                Position::from(vec![lon - d, lat - d]),
            ]],
        })
    }

    #[test]
    fn a_polygon_knows_its_bounds_centroid_and_inside() {
        let layer = MapLayer::polygon(&square(-71.8265, -36.1415, 0.01)).unwrap();
        let bounds = layer.bounds();
        assert!((bounds.west + 71.8365).abs() < 1e-9 && (bounds.north + 36.1315).abs() < 1e-9);
        let centroid = layer.centroid();
        assert!((centroid.lat + 36.1415).abs() < 1e-4 && (centroid.lon + 71.8265).abs() < 1e-9);
        assert!(layer.contains(LatLon::new(-36.1415, -71.8265)));
        assert!(layer.contains(LatLon::new(-36.135, -71.83)));
        assert!(!layer.contains(LatLon::new(-36.12, -71.8265)));
        assert!(!layer.contains(LatLon::new(-36.1415, -71.80)));
    }

    #[test]
    fn holes_are_outside() {
        let ring = |d: f64| {
            vec![
                Position::from(vec![-d, -d]),
                Position::from(vec![d, -d]),
                Position::from(vec![d, d]),
                Position::from(vec![-d, d]),
            ]
        };
        let donut = MapLayer::polygon(&Geometry::new(GeometryValue::Polygon {
            coordinates: vec![ring(1.0), ring(0.5)],
        }))
        .unwrap();
        assert!(!donut.contains(LatLon::new(0.0, 0.0)));
        assert!(donut.contains(LatLon::new(0.0, 0.75)));
        let centroid = donut.centroid();
        assert!(centroid.lat.abs() < 1e-6 && centroid.lon.abs() < 1e-6);
    }

    #[test]
    fn bad_geometry_is_an_error() {
        let line = Geometry::new(GeometryValue::LineString {
            coordinates: vec![
                Position::from(vec![0.0, 0.0]),
                Position::from(vec![1.0, 1.0]),
            ],
        });
        assert!(MapLayer::polygon(&line).is_err());
        let thin = Geometry::new(GeometryValue::Polygon {
            coordinates: vec![vec![
                Position::from(vec![0.0, 0.0]),
                Position::from(vec![1.0, 1.0]),
            ]],
        });
        assert!(MapLayer::polygon(&thin).is_err());
    }

    #[test]
    fn rings_and_lines_build_from_points() {
        let corners = [
            LatLon::new(-1.0, -1.0),
            LatLon::new(-1.0, 1.0),
            LatLon::new(1.0, 1.0),
            LatLon::new(1.0, -1.0),
        ];
        let ring = MapLayer::ring(&corners).unwrap();
        assert!(ring.contains(LatLon::new(0.0, 0.0)));
        for (back, corner) in ring.outer_ring().iter().zip(&corners) {
            assert!((back.lat - corner.lat).abs() < 1e-9 && (back.lon - corner.lon).abs() < 1e-9);
        }
        let closed = [corners.as_slice(), &corners[..1]].concat();
        assert!(
            MapLayer::ring(&closed)
                .unwrap()
                .contains(LatLon::new(0.5, 0.5))
        );
        assert!(MapLayer::ring(&corners[..2]).is_err());
        assert!(MapLayer::ring(&[corners[0], corners[1], LatLon::new(f64::NAN, 0.0)]).is_err());
        let line = MapLayer::line(&corners[..2]).unwrap();
        assert!(
            !line.contains(LatLon::new(-1.0, 0.0)),
            "a line has no inside"
        );
        assert!(line.shape.fill.is_empty() && !line.shape.outline.is_empty());
        assert!(MapLayer::line(&corners[..1]).is_err());
    }

    #[test]
    fn the_dsp_sectors_parse_with_their_properties() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/maps/dsp-areas.geojson"
        ))
        .unwrap();
        let areas = polygons(&text).unwrap();
        let names: Vec<&str> = areas
            .iter()
            .map(|(_, properties)| properties["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "Plaza de Armas",
                "Catillo",
                "Villa San Pedro",
                "Palma Rosa",
                "Terminal de Buses"
            ]
        );
        let (plaza, properties) = &areas[0];
        assert_eq!(properties["code"], "AOP26000001");
        assert_eq!(properties["type"], "sector");
        let bounds = plaza.bounds();
        assert!((bounds.east - bounds.west - 0.016).abs() < 1e-9);
        assert!((bounds.north - bounds.south - 0.012).abs() < 1e-9);
        assert!(polygons("{\"type\":\"Point\",\"coordinates\":[0,0]}").is_err());
        assert!(polygons("no es json").is_err());
    }

    #[test]
    fn hits_go_to_the_topmost_layer_with_an_id() {
        let center = LatLon::new(-36.1415, -71.8265);
        let size = Size {
            width: 400.0,
            height: 300.0,
        };
        let below = MapLayer::polygon(&square(-71.8265, -36.1415, 0.01))
            .unwrap()
            .id("abajo");
        let above = MapLayer::polygon(&square(-71.82, -36.1415, 0.005))
            .unwrap()
            .id("arriba");
        let mut layers = vec![below, above];
        let camera = Camera::new(MapView::new(center, 14.0), size);
        let hit_id = |layers: &[MapLayer], at| {
            hit(layers, &camera, camera.to_local(at)).map(|layer| layer.id.clone())
        };
        assert_eq!(
            hit_id(&layers, LatLon::new(-36.1415, -71.82)).as_deref(),
            Some("arriba")
        );
        assert_eq!(
            hit_id(&layers, LatLon::new(-36.1415, -71.833)).as_deref(),
            Some("abajo")
        );
        assert_eq!(hit_id(&layers, LatLon::new(-36.10, -71.8265)), None);
        layers[1].id.clear();
        assert_eq!(
            hit_id(&layers, LatLon::new(-36.1415, -71.82)).as_deref(),
            Some("abajo"),
            "layers without an id are not clickable"
        );
    }

    #[test]
    fn paint_places_fills_then_outlines_and_emphasizes() {
        let center = LatLon::new(-36.1415, -71.8265);
        let mut spec = MapSpec::new(MapView::new(center, 14.0));
        let layer = |lon: f64, id: &str| {
            MapLayer::polygon(&square(lon, -36.1415, 0.004))
                .unwrap()
                .id(id)
                .fill(SECTOR, 0.12)
                .stroke(SECTOR, 2.0)
        };
        spec.layers = vec![
            layer(-71.8265, "a"),
            layer(-71.82, "b"),
            layer(-71.5, "lejos"),
        ];
        spec.hovered = Some("a".to_owned());
        let area = Rect {
            x: 10.0,
            y: 20.0,
            width: 400.0,
            height: 300.0,
        };
        let mut list = DrawList::new(Color::rgb(0, 0, 0));
        paint(&mut list, area, &spec);
        let draws: Vec<&GeometryDraw> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                DrawOp::Geometry(draw) => Some(draw),
                _ => None,
            })
            .collect();
        assert_eq!(draws.len(), 4, "the far layer is culled");
        assert_eq!(draws[0].width, 0.0);
        assert_eq!(draws[0].color.a, 61, "hovered: twice 0.12");
        assert_eq!(draws[1].color.a, 31);
        assert_eq!(
            (draws[2].width, draws[3].width),
            (2.0, 3.0),
            "the hovered outline goes last, wider"
        );
        let camera = Camera::new(
            spec.view,
            Size {
                width: area.width,
                height: area.height,
            },
        );
        let corner = camera.to_local(LatLon::new(-36.1415 + 0.004, -71.8265 - 0.004));
        let extent = draws[0].extent().unwrap();
        assert!(
            (extent.x - (area.x + corner.x)).abs() < 0.01,
            "{extent:?} {corner:?}"
        );
        assert!(
            (extent.y - (area.y + corner.y)).abs() < 0.01,
            "{extent:?} {corner:?}"
        );
    }
}
