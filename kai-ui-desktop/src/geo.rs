//! Geographic coordinates and the Web Mercator projection of maps
//! ([`crate::maps`]).
//!
//! The world is a unit square ([`WorldPoint`], `x` east and `y` south, both
//! `0..1`); at zoom `z` it is [`TILE_SIZE`]` × 2^z` points wide, as in
//! Leaflet and OpenStreetMap tiles (zoom 13 shows a town). Math is in
//! `f64`: at zoom 18 a point is a few billionths of the world.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Size};

/// Points per tile side at an integer zoom.
pub const TILE_SIZE: f64 = 256.0;
/// Web Mercator stops here, where the world becomes square.
pub const MAX_LATITUDE: f64 = 85.051_128_779_806_59;
pub const MIN_ZOOM: f64 = 0.0;
pub const MAX_ZOOM: f64 = 20.0;

/// A place on Earth, in degrees (WGS 84).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LatLon {
    /// Latitude, −90 (south) to 90 (north).
    pub lat: f64,
    /// Longitude, −180 (west) to 180 (east).
    pub lon: f64,
}

impl LatLon {
    pub const fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }
}

/// Places compare by value; maps and their editors keep finite ones.
impl Eq for LatLon {}

/// A point of the Mercator world square, `0..1` on both axes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldPoint {
    pub x: f64,
    pub y: f64,
}

pub fn project(at: LatLon) -> WorldPoint {
    let lat = at.lat.clamp(-MAX_LATITUDE, MAX_LATITUDE).to_radians();
    WorldPoint {
        x: (at.lon + 180.0) / 360.0,
        y: (1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / std::f64::consts::PI) / 2.0,
    }
}

pub fn unproject(world: WorldPoint) -> LatLon {
    let n = std::f64::consts::PI * (1.0 - 2.0 * world.y);
    LatLon {
        lat: n.sinh().atan().to_degrees(),
        lon: world.x * 360.0 - 180.0,
    }
}

/// Points across the whole world at `zoom`.
pub fn world_size(zoom: f64) -> f64 {
    TILE_SIZE * zoom.exp2()
}

/// A latitude and longitude box.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LatLonBounds {
    /// Southern edge, latitude in degrees.
    pub south: f64,
    /// Western edge, longitude in degrees.
    pub west: f64,
    /// Northern edge, latitude in degrees.
    pub north: f64,
    /// Eastern edge, longitude in degrees.
    pub east: f64,
}

impl LatLonBounds {
    /// The smallest box around `points`; `None` without points.
    pub fn around(points: impl IntoIterator<Item = LatLon>) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next()?;
        let mut bounds = Self {
            south: first.lat,
            west: first.lon,
            north: first.lat,
            east: first.lon,
        };
        for p in points {
            bounds.south = bounds.south.min(p.lat);
            bounds.north = bounds.north.max(p.lat);
            bounds.west = bounds.west.min(p.lon);
            bounds.east = bounds.east.max(p.lon);
        }
        Some(bounds)
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            south: self.south.min(other.south),
            west: self.west.min(other.west),
            north: self.north.max(other.north),
            east: self.east.max(other.east),
        }
    }

    pub fn center(self) -> LatLon {
        let a = project(LatLon::new(self.south, self.west));
        let b = project(LatLon::new(self.north, self.east));
        unproject(WorldPoint {
            x: (a.x + b.x) / 2.0,
            y: (a.y + b.y) / 2.0,
        })
    }

    pub fn contains(self, at: LatLon) -> bool {
        (self.south..=self.north).contains(&at.lat) && (self.west..=self.east).contains(&at.lon)
    }

    /// The box grown by `fraction` of its height and width on every side.
    pub fn padded(self, fraction: f64) -> Self {
        let lat = (self.north - self.south) * fraction;
        let lon = (self.east - self.west) * fraction;
        Self {
            south: self.south - lat,
            west: self.west - lon,
            north: self.north + lat,
            east: self.east + lon,
        }
    }
}

/// Boxes compare by value; they come from finite coordinates.
impl Eq for LatLonBounds {}

/// What a map shows: its center and zoom (fractional zooms scale tiles).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MapView {
    pub center: LatLon,
    /// 0 (the whole world in one 256-point tile) to 20.
    pub zoom: f64,
}

/// Views compare by value; the map only keeps [`MapView::clamped`] ones,
/// which are never NaN.
impl Eq for MapView {}

impl MapView {
    pub const fn new(center: LatLon, zoom: f64) -> Self {
        Self { center, zoom }
    }

    /// Zoom within [`MIN_ZOOM`]..[`MAX_ZOOM`], latitude within the
    /// projection and longitude within −180..180; non-finite numbers give
    /// way to 0.
    pub fn clamped(self) -> Self {
        let finite = |value: f64| if value.is_finite() { value } else { 0.0 };
        let lon = (finite(self.center.lon) + 180.0).rem_euclid(360.0) - 180.0;
        Self {
            center: LatLon {
                lat: finite(self.center.lat).clamp(-MAX_LATITUDE, MAX_LATITUDE),
                lon,
            },
            zoom: finite(self.zoom).clamp(MIN_ZOOM, MAX_ZOOM),
        }
    }
}

/// A [`MapView`] shown on an area of `size` points: conversions between
/// geographic, world and local coordinates (`(0, 0)` is the area's top
/// left corner).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub view: MapView,
    pub size: Size,
}

impl Camera {
    pub fn new(view: MapView, size: Size) -> Self {
        Self { view, size }
    }

    /// Points per world unit.
    pub fn scale(&self) -> f64 {
        world_size(self.view.zoom)
    }

    fn half(&self) -> (f64, f64) {
        (
            f64::from(self.size.width) / 2.0,
            f64::from(self.size.height) / 2.0,
        )
    }

    /// The world point at the center of the area.
    pub fn center(&self) -> WorldPoint {
        project(self.view.center)
    }

    /// Where `world` falls on the area, in `f64` points.
    pub fn world_to_local(&self, world: WorldPoint) -> (f64, f64) {
        let center = self.center();
        let scale = self.scale();
        let (half_x, half_y) = self.half();
        (
            (world.x - center.x) * scale + half_x,
            (world.y - center.y) * scale + half_y,
        )
    }

    pub fn to_local(&self, at: LatLon) -> Point {
        let (x, y) = self.world_to_local(project(at));
        Point {
            x: x as f32,
            y: y as f32,
        }
    }

    pub fn local_to_world(&self, local: Point) -> WorldPoint {
        let center = self.center();
        let scale = self.scale();
        let (half_x, half_y) = self.half();
        WorldPoint {
            x: center.x + (f64::from(local.x) - half_x) / scale,
            y: center.y + (f64::from(local.y) - half_y) / scale,
        }
    }

    pub fn to_lat_lon(&self, local: Point) -> LatLon {
        unproject(self.local_to_world(local))
    }

    /// The view after the content moves by `(dx, dy)` points, as under a
    /// dragging pointer.
    pub fn panned(&self, dx: f64, dy: f64) -> MapView {
        let center = self.center();
        let scale = self.scale();
        MapView {
            center: unproject(WorldPoint {
                x: center.x - dx / scale,
                y: (center.y - dy / scale).clamp(0.0, 1.0),
            }),
            ..self.view
        }
        .clamped()
    }

    /// The view at `zoom` that keeps the world point under `anchor` (local
    /// points) where it is.
    pub fn zoomed_around(&self, anchor: Point, zoom: f64) -> MapView {
        let world = self.local_to_world(anchor);
        anchored_view(world, anchor, zoom, self.size)
    }

    /// The box the area shows.
    pub fn visible_bounds(&self) -> LatLonBounds {
        let top_left = self.to_lat_lon(Point::default());
        let bottom_right = self.to_lat_lon(Point {
            x: self.size.width,
            y: self.size.height,
        });
        LatLonBounds {
            south: bottom_right.lat,
            west: top_left.lon,
            north: top_left.lat,
            east: bottom_right.lon,
        }
    }

    /// Tiles of zoom `z` that cover the area, nearest to its center first.
    pub fn visible_tiles(&self, z: u8) -> Vec<TileId> {
        let count = 1u32 << z;
        let tiles = f64::from(count);
        let top_left = self.local_to_world(Point::default());
        let bottom_right = self.local_to_world(Point {
            x: self.size.width,
            y: self.size.height,
        });
        let index = |value: f64| (value * tiles).floor().clamp(0.0, tiles - 1.0) as u32;
        let (x0, x1) = (index(top_left.x), index(bottom_right.x));
        let (y0, y1) = (index(top_left.y), index(bottom_right.y));
        let center = self.center();
        let (cx, cy) = (center.x * tiles, center.y * tiles);
        let mut out = Vec::with_capacity(((x1 - x0 + 1) * (y1 - y0 + 1)) as usize);
        for y in y0..=y1 {
            for x in x0..=x1 {
                out.push(TileId { z, x, y });
            }
        }
        let distance = |tile: &TileId| {
            let dx = f64::from(tile.x) + 0.5 - cx;
            let dy = f64::from(tile.y) + 0.5 - cy;
            dx * dx + dy * dy
        };
        out.sort_by(|a, b| distance(a).total_cmp(&distance(b)));
        out
    }
}

/// The view at `zoom` that shows `world` at `anchor` on an area of `size`.
pub fn anchored_view(world: WorldPoint, anchor: Point, zoom: f64, size: Size) -> MapView {
    let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    let scale = world_size(zoom);
    let center = WorldPoint {
        x: world.x - (f64::from(anchor.x) - f64::from(size.width) / 2.0) / scale,
        y: (world.y - (f64::from(anchor.y) - f64::from(size.height) / 2.0) / scale).clamp(0.0, 1.0),
    };
    MapView {
        center: unproject(center),
        zoom,
    }
    .clamped()
}

/// The view that shows all of `bounds` on an area of `size` with
/// `padding` points free on every side, no closer than `max_zoom`.
pub fn fit_bounds(bounds: LatLonBounds, size: Size, padding: f32, max_zoom: f64) -> MapView {
    let a = project(LatLon::new(bounds.north, bounds.west));
    let b = project(LatLon::new(bounds.south, bounds.east));
    let span_x = (b.x - a.x).abs().max(1e-12);
    let span_y = (b.y - a.y).abs().max(1e-12);
    let room_x = f64::from((size.width - 2.0 * padding).max(1.0));
    let room_y = f64::from((size.height - 2.0 * padding).max(1.0));
    let zoom = (room_x / (span_x * TILE_SIZE))
        .min(room_y / (span_y * TILE_SIZE))
        .log2()
        .min(max_zoom);
    MapView {
        center: unproject(WorldPoint {
            x: (a.x + b.x) / 2.0,
            y: (a.y + b.y) / 2.0,
        }),
        zoom,
    }
    .clamped()
}

/// A tile of the `2^z × 2^z` grid at zoom `z` (XYZ scheme: `y` grows
/// southward).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileId {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

impl TileId {
    pub const fn new(z: u8, x: u32, y: u32) -> Self {
        Self { z, x, y }
    }

    /// The tile of zoom `z - 1` that contains this one.
    pub fn parent(self) -> Option<Self> {
        (self.z > 0).then(|| Self::new(self.z - 1, self.x / 2, self.y / 2))
    }

    /// The tile of zoom `z` (≤ this one's) that contains it.
    pub fn ancestor(self, z: u8) -> Option<Self> {
        (z <= self.z).then(|| {
            let shift = self.z - z;
            Self::new(z, self.x >> shift, self.y >> shift)
        })
    }

    /// Top left corner in world units and side.
    pub fn world_origin(self) -> (WorldPoint, f64) {
        let side = 1.0 / f64::from(1u32 << self.z);
        (
            WorldPoint {
                x: f64::from(self.x) * side,
                y: f64::from(self.y) * side,
            },
            side,
        )
    }

    pub fn is_valid(self) -> bool {
        self.z <= 30 && self.x < (1u32 << self.z) && self.y < (1u32 << self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Camera, LatLon, LatLonBounds, MapView, TileId, fit_bounds, project, unproject, world_size,
    };
    use crate::geometry::{Point, Size};

    const PLAZA: LatLon = LatLon::new(-36.1415, -71.8265);
    const SIZE: Size = Size {
        width: 800.0,
        height: 600.0,
    };

    #[test]
    fn projection_round_trips_and_matches_reference_points() {
        for at in [
            PLAZA,
            LatLon::new(0.0, 0.0),
            LatLon::new(60.0, 179.0),
            LatLon::new(-80.0, -179.5),
        ] {
            let back = unproject(project(at));
            assert!((back.lat - at.lat).abs() < 1e-9, "{at:?} {back:?}");
            assert!((back.lon - at.lon).abs() < 1e-9);
        }
        let origin = project(LatLon::new(0.0, 0.0));
        assert_eq!((origin.x, origin.y), (0.5, 0.5));
        assert!(
            project(LatLon::new(89.0, 0.0)).y >= 0.0,
            "clamped at the pole"
        );
        // OpenStreetMap: Parral's Plaza de Armas is in tile 13/2461/4979.
        let world = project(PLAZA);
        let tiles = f64::from(1u32 << 13);
        assert_eq!(
            ((world.x * tiles) as u32, (world.y * tiles) as u32),
            (2461, 4979)
        );
        assert_eq!(world_size(1.0), 512.0);
    }

    #[test]
    fn the_center_is_in_the_middle_and_local_points_round_trip() {
        let camera = Camera::new(MapView::new(PLAZA, 13.0), SIZE);
        let center = camera.to_local(PLAZA);
        assert!((center.x - 400.0).abs() < 1e-3 && (center.y - 300.0).abs() < 1e-3);
        let local = Point {
            x: 123.5,
            y: 456.25,
        };
        let back = camera.to_local(camera.to_lat_lon(local));
        assert!((back.x - local.x).abs() < 1e-3 && (back.y - local.y).abs() < 1e-3);
        let bounds = camera.visible_bounds();
        assert!(bounds.contains(PLAZA));
        assert!(bounds.west < PLAZA.lon && bounds.east > PLAZA.lon);
    }

    #[test]
    fn panning_moves_the_content_with_the_pointer() {
        let camera = Camera::new(MapView::new(PLAZA, 14.0), SIZE);
        let moved = Camera::new(camera.panned(30.0, -20.0), SIZE);
        let plaza = moved.to_local(PLAZA);
        assert!((plaza.x - 430.0).abs() < 1e-3, "{plaza:?}");
        assert!((plaza.y - 280.0).abs() < 1e-3);
        assert_eq!(moved.view.zoom, 14.0);
    }

    #[test]
    fn zooming_keeps_the_anchor_in_place() {
        let camera = Camera::new(MapView::new(PLAZA, 13.0), SIZE);
        let anchor = Point { x: 650.0, y: 120.0 };
        let under = camera.to_lat_lon(anchor);
        for zoom in [12.0, 13.7, 15.0, 18.0] {
            let zoomed = Camera::new(camera.zoomed_around(anchor, zoom), SIZE);
            let at = zoomed.to_local(under);
            assert!((at.x - anchor.x).abs() < 1e-3 && (at.y - anchor.y).abs() < 1e-3);
            assert_eq!(zoomed.view.zoom, zoom);
        }
        let far = camera.zoomed_around(anchor, 40.0);
        assert_eq!(far.zoom, super::MAX_ZOOM);
    }

    #[test]
    fn fit_bounds_shows_the_box_inside_the_padding() {
        let bounds = LatLonBounds {
            south: -36.155,
            west: -71.846,
            north: -36.13,
            east: -71.80,
        };
        let view = fit_bounds(bounds, SIZE, 40.0, 18.0);
        let camera = Camera::new(view, SIZE);
        let nw = camera.to_local(LatLon::new(bounds.north, bounds.west));
        let se = camera.to_local(LatLon::new(bounds.south, bounds.east));
        assert!(nw.x >= 39.9 && nw.y >= 39.9, "{nw:?}");
        assert!(se.x <= 760.1 && se.y <= 560.1, "{se:?}");
        let touches = (nw.x - 40.0).abs() < 0.1 || (nw.y - 40.0).abs() < 0.1;
        assert!(touches, "one side fills its room: {nw:?}");
        let point = fit_bounds(LatLonBounds::around([PLAZA]).unwrap(), SIZE, 40.0, 16.0);
        assert_eq!(point.zoom, 16.0, "a single point stops at max_zoom");
    }

    #[test]
    fn visible_tiles_cover_the_area_center_first() {
        let camera = Camera::new(MapView::new(PLAZA, 13.0), SIZE);
        let tiles = camera.visible_tiles(13);
        // 800×600 points over 256-point tiles: 4 or 5 columns, 3 or 4 rows.
        assert!((12..=20).contains(&tiles.len()), "{}", tiles.len());
        let world = project(PLAZA);
        let tiles_per_side = 8192.0;
        assert_eq!(
            tiles[0],
            TileId::new(
                13,
                (world.x * tiles_per_side) as u32,
                (world.y * tiles_per_side) as u32
            )
        );
        assert!(tiles.iter().all(|tile| tile.is_valid()));
        let tile = TileId::new(13, 2461, 4979);
        assert_eq!(tile.parent(), Some(TileId::new(12, 1230, 2489)));
        assert_eq!(tile.ancestor(10), Some(TileId::new(10, 307, 622)));
        assert_eq!(tile.ancestor(14), None);
        assert_eq!(TileId::new(0, 0, 0).parent(), None);
    }

    #[test]
    fn views_are_clamped_to_the_projection() {
        let view = MapView::new(LatLon::new(89.0, 190.0), 25.0).clamped();
        assert_eq!(view.zoom, 20.0);
        assert!(view.center.lat <= super::MAX_LATITUDE);
        assert!((view.center.lon - -170.0).abs() < 1e-9);
        let nan = MapView::new(LatLon::new(f64::NAN, 0.0), f64::NAN).clamped();
        assert_eq!(nan, MapView::new(LatLon::new(0.0, 0.0), 0.0));
    }
}
