//! How the basemap looks: a style in Rust (not MapLibre JSON) for the
//! layers of the Protomaps basemap (schema 4). [`MapStyle::buckets`] say
//! which features become meshes on the task pool; [`MapStyle::layers`]
//! paint them bottom to top, each over every tile before the next (so road
//! casings stay under every road), with colors taken from the theme and
//! widths interpolated by zoom. A stroke is drawn from the same mesh at any
//! width, so a road's casing and its fill share it.

use super::MapSpec;
use super::mvt::{Bucket, EXTENT};
use crate::geo::{Camera, TileId};
use crate::geometry::{Rect, Size};
use crate::render::{Affine2, Color, DrawList, GeometryDraw};
use crate::theme::{Appearance, Theme};

/// Zoom and value: widths are linear between stops and constant beyond.
pub type Stop = (f32, f32);

/// Interpolate `stops` (sorted by zoom) at `zoom`.
pub fn interpolate(stops: &[Stop], zoom: f32) -> f32 {
    let Some(&(first_zoom, first)) = stops.first() else {
        return 0.0;
    };
    if zoom <= first_zoom {
        return first;
    }
    for pair in stops.windows(2) {
        let ((z0, v0), (z1, v1)) = (pair[0], pair[1]);
        if zoom <= z1 {
            return v0 + (v1 - v0) * (zoom - z0) / (z1 - z0);
        }
    }
    stops[stops.len() - 1].1
}

/// A color of the [`Palette`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Earth,
    Park,
    Wood,
    Farmland,
    Civic,
    Industrial,
    Water,
    Building,
    Casing,
    Road,
    Highway,
    Path,
    Rail,
    Boundary,
}

/// The basemap's colors, from the theme: the label color over the window
/// background for land, buildings and lines, and the status colors for
/// parks (`success`), water (`info`) and farmland (`warning`). All opaque,
/// so overlapping features never darken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub earth: Color,
    pub park: Color,
    pub wood: Color,
    pub farmland: Color,
    pub civic: Color,
    pub industrial: Color,
    pub water: Color,
    pub building: Color,
    pub casing: Color,
    pub road: Color,
    pub highway: Color,
    pub path: Color,
    pub rail: Color,
    pub boundary: Color,
    /// Names of towns.
    pub place_text: Color,
    /// Names of neighbourhoods and streets.
    pub minor_text: Color,
    /// Outline around label text.
    pub halo: Color,
}

/// `top` over the opaque `under` at `amount` of its own opacity.
fn over(top: Color, under: Color, amount: f32) -> Color {
    top.mix(under, amount * f32::from(top.a) / 255.0)
        .with_alpha(255)
}

impl Palette {
    pub fn new(theme: &Theme) -> Self {
        let dark = theme.appearance == Appearance::Dark;
        let s = &theme.semantic;
        let c = &theme.colors;
        let ground = s.window_background.with_alpha(255);
        let label = s.label;
        let earth = over(label, ground, 0.07);
        let road = if dark {
            over(label, earth, 0.2)
        } else {
            ground
        };
        Self {
            earth,
            park: over(c.success, earth, if dark { 0.3 } else { 0.22 }),
            wood: over(c.success, earth, if dark { 0.4 } else { 0.32 }),
            farmland: over(c.warning, earth, 0.1),
            civic: over(c.error, earth, 0.07),
            industrial: over(label, earth, 0.05),
            water: over(c.info, earth, if dark { 0.45 } else { 0.32 }),
            building: over(label, earth, if dark { 0.1 } else { 0.12 }),
            casing: over(label, earth, if dark { 0.06 } else { 0.2 }),
            road,
            highway: over(c.warning, road, if dark { 0.35 } else { 0.45 }),
            path: over(label, earth, 0.3),
            rail: over(label, earth, 0.35),
            boundary: over(label, earth, 0.3),
            place_text: over(label, earth, 0.9),
            minor_text: over(s.secondary_label, earth, 1.0),
            halo: earth,
        }
    }

    pub fn ink(&self, ink: Ink) -> Color {
        match ink {
            Ink::Earth => self.earth,
            Ink::Park => self.park,
            Ink::Wood => self.wood,
            Ink::Farmland => self.farmland,
            Ink::Civic => self.civic,
            Ink::Industrial => self.industrial,
            Ink::Water => self.water,
            Ink::Building => self.building,
            Ink::Casing => self.casing,
            Ink::Road => self.road,
            Ink::Highway => self.highway,
            Ink::Path => self.path,
            Ink::Rail => self.rail,
            Ink::Boundary => self.boundary,
        }
    }
}

/// How a style layer draws its bucket's mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    Fill,
    /// Stroke `width(zoom)` points wide.
    Line(&'static [Stop]),
    /// The casing of a [`Paint::Line`] with those stops: [`CASING`] wider.
    Casing(&'static [Stop]),
}

/// Extra width of a road casing (both sides together).
pub const CASING: &[Stop] = &[(12.0, 1.0), (16.0, 2.0), (18.0, 3.0)];

impl Paint {
    /// Stroke width at `zoom`; `0` for fills.
    pub fn width(self, zoom: f32) -> f32 {
        match self {
            Self::Fill => 0.0,
            Self::Line(stops) => interpolate(stops, zoom),
            Self::Casing(stops) => interpolate(stops, zoom) + interpolate(CASING, zoom),
        }
    }
}

/// One drawing step: bucket `bucket` of the style, painted with `ink`
/// while the view zoom is in `zooms` (min inclusive, max exclusive).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyleLayer {
    pub id: &'static str,
    pub bucket: usize,
    pub ink: Ink,
    pub paint: Paint,
    pub zooms: (f32, f32),
}

impl StyleLayer {
    pub const fn fill(id: &'static str, bucket: usize, ink: Ink) -> Self {
        Self {
            id,
            bucket,
            ink,
            paint: Paint::Fill,
            zooms: (0.0, 99.0),
        }
    }

    pub const fn line(id: &'static str, bucket: usize, ink: Ink, stops: &'static [Stop]) -> Self {
        Self {
            id,
            bucket,
            ink,
            paint: Paint::Line(stops),
            zooms: (0.0, 99.0),
        }
    }

    pub const fn casing(id: &'static str, bucket: usize, stops: &'static [Stop]) -> Self {
        Self {
            id,
            bucket,
            ink: Ink::Casing,
            paint: Paint::Casing(stops),
            zooms: (0.0, 99.0),
        }
    }

    pub const fn from_zoom(mut self, zoom: f32) -> Self {
        self.zooms.0 = zoom;
        self
    }

    pub fn shows_at(&self, zoom: f32) -> bool {
        (self.zooms.0..self.zooms.1).contains(&zoom)
    }
}

/// What a tile source draws: its buckets (meshes built per tile) and the
/// layers that paint them, bottom first.
#[derive(Debug)]
pub struct MapStyle {
    pub buckets: &'static [Bucket],
    pub layers: &'static [StyleLayer],
}

impl MapStyle {
    /// The basemap of a Protomaps archive.
    pub fn protomaps() -> &'static Self {
        &PROTOMAPS
    }
}

mod bucket {
    pub const EARTH: usize = 0;
    pub const LANDCOVER_WOOD: usize = 1;
    pub const LANDCOVER_FARMLAND: usize = 2;
    pub const FARMLAND: usize = 3;
    pub const WOOD: usize = 4;
    pub const PARK: usize = 5;
    pub const CIVIC: usize = 6;
    pub const INDUSTRIAL: usize = 7;
    pub const WATER: usize = 8;
    pub const WATERWAY: usize = 9;
    pub const BUILDINGS: usize = 10;
    pub const PATH: usize = 11;
    pub const OTHER: usize = 12;
    pub const MINOR: usize = 13;
    pub const MAJOR: usize = 14;
    pub const HIGHWAY: usize = 15;
    pub const RAIL: usize = 16;
    pub const BOUNDARIES: usize = 17;
}

const PARK_KINDS: &[&str] = &[
    "park",
    "garden",
    "grass",
    "golf_course",
    "playground",
    "pitch",
    "recreation_ground",
    "cemetery",
    "dog_park",
    "village_green",
    "meadow",
    "grassland",
    "national_park",
    "nature_reserve",
    "protected_area",
    "zoo",
];
const WOOD_KINDS: &[&str] = &["wood", "forest", "scrub", "wetland"];
const FARM_KINDS: &[&str] = &[
    "farmland",
    "farmyard",
    "orchard",
    "vineyard",
    "allotments",
    "plant_nursery",
];
const CIVIC_KINDS: &[&str] = &[
    "hospital",
    "school",
    "kindergarten",
    "university",
    "college",
];
const INDUSTRIAL_KINDS: &[&str] = &[
    "industrial",
    "railway",
    "commercial",
    "retail",
    "aerodrome",
    "military",
    "quarry",
    "landfill",
];

const PATH_WIDTH: &[Stop] = &[(14.0, 0.6), (17.0, 1.5), (19.0, 2.5)];
const OTHER_WIDTH: &[Stop] = &[(14.0, 0.6), (16.0, 2.5), (18.0, 7.0), (20.0, 18.0)];
const MINOR_WIDTH: &[Stop] = &[
    (12.0, 0.4),
    (14.0, 1.5),
    (16.0, 5.0),
    (18.0, 14.0),
    (20.0, 40.0),
];
const MAJOR_WIDTH: &[Stop] = &[
    (9.0, 0.5),
    (13.0, 2.0),
    (15.0, 5.0),
    (18.0, 18.0),
    (20.0, 48.0),
];
const HIGHWAY_WIDTH: &[Stop] = &[
    (7.0, 0.6),
    (12.0, 2.5),
    (15.0, 6.0),
    (18.0, 22.0),
    (20.0, 56.0),
];
const RAIL_WIDTH: &[Stop] = &[(11.0, 0.5), (15.0, 1.2), (18.0, 2.5)];
const WATERWAY_WIDTH: &[Stop] = &[(10.0, 0.5), (14.0, 1.5), (17.0, 4.0), (20.0, 12.0)];
const BOUNDARY_WIDTH: &[Stop] = &[(6.0, 0.5), (12.0, 1.0)];

static PROTOMAPS: MapStyle = MapStyle {
    buckets: &[
        Bucket::fill("earth", &[]),
        Bucket::fill("landcover", &["forest", "scrub", "grassland"]),
        Bucket::fill("landcover", &["farmland"]),
        Bucket::fill("landuse", FARM_KINDS),
        Bucket::fill("landuse", WOOD_KINDS),
        Bucket::fill("landuse", PARK_KINDS),
        Bucket::fill("landuse", CIVIC_KINDS),
        Bucket::fill("landuse", INDUSTRIAL_KINDS),
        Bucket::fill("water", &[]),
        Bucket::line("water", &["river", "stream", "canal", "drain", "ditch"]),
        Bucket::fill("buildings", &[]).from_zoom(14),
        Bucket::line("roads", &["path"]).from_zoom(14),
        Bucket::line("roads", &["other"]).from_zoom(14),
        Bucket::line("roads", &["minor_road"]),
        Bucket::line("roads", &["major_road"]),
        Bucket::line("roads", &["highway"]),
        Bucket::line("roads", &["rail"]),
        Bucket::line("boundaries", &[]),
    ],
    layers: &[
        StyleLayer::fill("earth", bucket::EARTH, Ink::Earth),
        StyleLayer::fill("landcover-wood", bucket::LANDCOVER_WOOD, Ink::Wood),
        StyleLayer::fill(
            "landcover-farmland",
            bucket::LANDCOVER_FARMLAND,
            Ink::Farmland,
        ),
        StyleLayer::fill("farmland", bucket::FARMLAND, Ink::Farmland),
        StyleLayer::fill("industrial", bucket::INDUSTRIAL, Ink::Industrial),
        StyleLayer::fill("civic", bucket::CIVIC, Ink::Civic),
        StyleLayer::fill("wood", bucket::WOOD, Ink::Wood),
        StyleLayer::fill("park", bucket::PARK, Ink::Park),
        StyleLayer::fill("water", bucket::WATER, Ink::Water),
        StyleLayer::line("waterway", bucket::WATERWAY, Ink::Water, WATERWAY_WIDTH),
        StyleLayer::fill("buildings", bucket::BUILDINGS, Ink::Building).from_zoom(15.0),
        StyleLayer::line("path", bucket::PATH, Ink::Path, PATH_WIDTH).from_zoom(15.0),
        StyleLayer::casing("other-casing", bucket::OTHER, OTHER_WIDTH).from_zoom(15.0),
        StyleLayer::casing("minor-casing", bucket::MINOR, MINOR_WIDTH).from_zoom(14.0),
        StyleLayer::casing("major-casing", bucket::MAJOR, MAJOR_WIDTH).from_zoom(12.0),
        StyleLayer::casing("highway-casing", bucket::HIGHWAY, HIGHWAY_WIDTH).from_zoom(10.0),
        StyleLayer::line("other", bucket::OTHER, Ink::Road, OTHER_WIDTH).from_zoom(15.0),
        StyleLayer::line("minor", bucket::MINOR, Ink::Road, MINOR_WIDTH),
        StyleLayer::line("major", bucket::MAJOR, Ink::Road, MAJOR_WIDTH),
        StyleLayer::line("highway", bucket::HIGHWAY, Ink::Highway, HIGHWAY_WIDTH),
        StyleLayer::line("rail", bucket::RAIL, Ink::Rail, RAIL_WIDTH),
        StyleLayer::line(
            "boundaries",
            bucket::BOUNDARIES,
            Ink::Boundary,
            BOUNDARY_WIDTH,
        ),
    ],
};

/// Where `tile`'s units land on a map showing `camera` over `area`, and
/// the rect the tile covers there.
fn place(tile: TileId, camera: &Camera, area: Rect) -> (Affine2, Rect) {
    let (corner, side) = tile.world_origin();
    let (x, y) = camera.world_to_local(corner);
    let points = side * camera.scale();
    let left = f64::from(area.x) + x;
    let top = f64::from(area.y) + y;
    (
        Affine2::scale_translate((points / f64::from(EXTENT)) as f32, left as f32, top as f32),
        Rect {
            x: left as f32,
            y: top as f32,
            width: points as f32,
            height: points as f32,
        },
    )
}

/// Paint the tiles of `spec` over `area` (the map's padding box), clipped
/// to it; a stand-in only inside the tile it stands in for.
pub fn paint(list: &mut DrawList, area: Rect, spec: &MapSpec, theme: &Theme) {
    let Some(source) = &spec.source else {
        return;
    };
    if spec.tiles.tiles.is_empty() || area.width <= 0.0 || area.height <= 0.0 {
        return;
    }
    let camera = Camera::new(
        spec.view,
        Size {
            width: area.width,
            height: area.height,
        },
    );
    let placed: Vec<_> = spec
        .tiles
        .tiles
        .iter()
        .map(|placed| {
            let (transform, _) = place(placed.tile, &camera, area);
            let clip = placed.clip.map(|clip| place(clip, &camera, area).1);
            (transform, clip, &placed.meshes)
        })
        .collect();
    let palette = Palette::new(theme);
    let zoom = spec.view.zoom as f32;
    list.push_clip(area);
    for layer in source.style().layers {
        if !layer.shows_at(zoom) {
            continue;
        }
        let color = palette.ink(layer.ink);
        let width = layer.paint.width(zoom);
        for (transform, clip, meshes) in &placed {
            let Some(mesh) = meshes.mesh(layer.bucket) else {
                continue;
            };
            let draw = match layer.paint {
                Paint::Fill => GeometryDraw::fill(mesh.clone(), *transform, color),
                Paint::Line(_) | Paint::Casing(_) => {
                    GeometryDraw::stroke(mesh.clone(), *transform, color, width)
                }
            };
            if draw
                .extent()
                .is_none_or(|extent| extent.intersection(area).is_none())
            {
                continue;
            }
            if let Some(clip) = clip {
                list.push_clip(*clip);
            }
            list.geometry(draw);
            if clip.is_some() {
                list.pop_clip();
            }
        }
    }
    list.pop_clip();
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::geo::{LatLon, MapView, project};
    use crate::geometry::{ClipRect, Point};
    use crate::maps::mvt::TileMeshes;
    use crate::maps::tiles::{PlacedTile, TileFrame, TileSource, TileStore};
    use crate::render::{DrawOp, Mesh};
    use crate::theme::{AppKitGeneration, Density};

    const PLAZA: LatLon = LatLon::new(-36.1415, -71.8265);

    fn parral() -> TileSource {
        TileSource::pmtiles(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/maps/parral.pmtiles"),
            MapStyle::protomaps(),
        )
    }

    #[test]
    fn widths_interpolate_between_stops_and_hold_beyond() {
        let stops = &[(10.0, 1.0), (14.0, 5.0), (16.0, 9.0)];
        assert_eq!(interpolate(stops, 5.0), 1.0);
        assert_eq!(interpolate(stops, 12.0), 3.0);
        assert_eq!(interpolate(stops, 15.0), 7.0);
        assert_eq!(interpolate(stops, 19.0), 9.0);
        assert_eq!(interpolate(&[], 12.0), 0.0);
        let road = Paint::Line(MINOR_WIDTH);
        let casing = Paint::Casing(MINOR_WIDTH);
        assert_eq!(road.width(14.0), 1.5);
        assert_eq!(casing.width(14.0), 1.5 + 1.5);
        assert_eq!(Paint::Fill.width(14.0), 0.0);
        assert!(road.width(17.0) > road.width(15.0), "wider closer in");
    }

    #[test]
    fn the_style_uses_every_bucket_and_cases_roads_under_all_of_them() {
        let style = MapStyle::protomaps();
        for (index, _) in style.buckets.iter().enumerate() {
            assert!(
                style.layers.iter().any(|layer| layer.bucket == index),
                "bucket {index} unused"
            );
        }
        assert!(
            style
                .layers
                .iter()
                .all(|layer| layer.bucket < style.buckets.len())
        );
        let position = |id: &str| {
            style
                .layers
                .iter()
                .position(|layer| layer.id == id)
                .unwrap()
        };
        let last_casing = style
            .layers
            .iter()
            .rposition(|layer| matches!(layer.paint, Paint::Casing(_)))
            .unwrap();
        assert!(last_casing < position("other"));
        assert!(position("earth") == 0);
        assert!(position("water") < position("buildings"));
        assert!(position("buildings") < position("minor"));
    }

    #[test]
    fn the_palette_follows_the_theme_and_is_opaque() {
        let light = Palette::new(&Theme::kai(Density::Comfortable));
        let dark = Palette::new(&Theme::appkit_static(
            AppKitGeneration::Aqua11,
            Appearance::Dark,
        ));
        assert_ne!(light, dark);
        let luma =
            |c: Color| 0.2126 * f32::from(c.r) + 0.7152 * f32::from(c.g) + 0.0722 * f32::from(c.b);
        for palette in [light, dark] {
            for ink in [
                Ink::Earth,
                Ink::Park,
                Ink::Water,
                Ink::Building,
                Ink::Casing,
                Ink::Road,
                Ink::Highway,
                Ink::Boundary,
            ] {
                assert_eq!(palette.ink(ink).a, 255, "{ink:?}");
            }
            assert!(palette.water.b > palette.water.r, "water is blue");
            assert!(palette.park.g > palette.park.r, "parks are green");
            assert!(
                (luma(palette.road) - luma(palette.earth)).abs() > 10.0,
                "roads stand out from the land"
            );
        }
        assert!(luma(light.earth) > 200.0 && luma(dark.earth) < 100.0);
    }

    fn square() -> Arc<Mesh> {
        Arc::new(Mesh::fill(
            &[vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: EXTENT, y: 0.0 },
                Point {
                    x: EXTENT,
                    y: EXTENT,
                },
                Point { x: 0.0, y: EXTENT },
            ]],
            8.0,
        ))
    }

    fn spec(tiles: Vec<PlacedTile>) -> MapSpec {
        MapSpec {
            source: Some(parral()),
            tiles: TileFrame {
                tiles,
                pending: false,
                revealing: false,
            },
            ..MapSpec::new(MapView::new(PLAZA, 14.0))
        }
    }

    const AREA: Rect = Rect {
        x: 100.0,
        y: 50.0,
        width: 400.0,
        height: 300.0,
    };

    #[test]
    fn tiles_land_where_the_camera_puts_them_and_stand_ins_are_clipped() {
        let camera = Camera::new(
            MapView::new(PLAZA, 14.0),
            Size {
                width: AREA.width,
                height: AREA.height,
            },
        );
        let exact = camera.visible_tiles(14)[0];
        let parent = exact.parent().unwrap();
        let other = camera.visible_tiles(14)[1];
        let meshes = |buckets: &[usize]| {
            Arc::new(TileMeshes {
                meshes: buckets.iter().map(|bucket| (*bucket, square())).collect(),
                labels: Vec::new(),
            })
        };
        let spec = spec(vec![
            PlacedTile {
                tile: parent,
                clip: Some(other),
                meshes: meshes(&[bucket::EARTH, bucket::MINOR]),
            },
            PlacedTile {
                tile: exact,
                clip: None,
                meshes: meshes(&[bucket::EARTH, bucket::MINOR]),
            },
        ]);
        let theme = Theme::kai(Density::Comfortable);
        let mut list = DrawList::new(Color::rgb(255, 255, 255));
        paint(&mut list, AREA, &spec, &theme);
        let ops = &list.ops;
        assert_eq!(ops.first(), Some(&DrawOp::PushClip(ClipRect::sharp(AREA))));
        assert_eq!(ops.last(), Some(&DrawOp::PopClip));

        let geometry: Vec<&GeometryDraw> = ops
            .iter()
            .filter_map(|op| match op {
                DrawOp::Geometry(draw) => Some(draw),
                _ => None,
            })
            .collect();
        let palette = Palette::new(&theme);
        let colors: Vec<Color> = geometry.iter().map(|draw| draw.color).collect();
        assert_eq!(
            colors,
            [
                palette.earth,
                palette.earth,
                palette.casing,
                palette.casing,
                palette.road,
                palette.road
            ],
            "layer by layer across tiles"
        );
        assert_eq!(geometry[2].width, MINOR_WIDTH[1].1 + 1.5);

        // The exact tile's corner is where the camera projects it.
        let (corner, _) = exact.world_origin();
        let (x, y) = camera.world_to_local(corner);
        let at = geometry[1].transform.apply(Point::default());
        assert!((at.x - (AREA.x + x as f32)).abs() < 1e-2);
        assert!((at.y - (AREA.y + y as f32)).abs() < 1e-2);
        let side = geometry[1].transform.apply(Point { x: EXTENT, y: 0.0 }).x - at.x;
        assert!(
            (side - 256.0).abs() < 1e-2,
            "a zoom 14 tile at zoom 14: {side}"
        );

        // The stand-in draws only inside the tile it stands in for.
        let (_, inside) = place(other, &camera, AREA);
        let index = ops
            .iter()
            .position(|op| matches!(op, DrawOp::Geometry(_)))
            .unwrap();
        assert_eq!(ops[index - 1], DrawOp::PushClip(ClipRect::sharp(inside)));
        assert_eq!(ops[index + 1], DrawOp::PopClip);
        assert_eq!(
            geometry[0].transform.a,
            geometry[1].transform.a * 2.0,
            "the parent is drawn at its own scale"
        );
    }

    #[test]
    fn nothing_is_drawn_without_tiles_and_layers_respect_their_zooms() {
        let theme = Theme::kai(Density::Comfortable);
        let mut list = DrawList::new(Color::rgb(255, 255, 255));
        paint(&mut list, AREA, &spec(Vec::new()), &theme);
        assert!(list.ops.is_empty());

        let style = MapStyle::protomaps();
        let buildings = style.layers.iter().find(|l| l.id == "buildings").unwrap();
        assert!(!buildings.shows_at(14.9) && buildings.shows_at(15.0));
    }

    #[test]
    fn parral_draws_land_streets_and_water_at_zoom_15() {
        let size = Size {
            width: 600.0,
            height: 400.0,
        };
        let camera = Camera::new(MapView::new(PLAZA, 15.0), size);
        let mut store = TileStore::default();
        let frame = store.load_now(&parral(), camera);
        assert!(!frame.pending && !frame.tiles.is_empty());
        assert!(frame.tiles.iter().all(|placed| placed.tile.z == 15));
        let spec = MapSpec {
            tiles: frame,
            ..spec(Vec::new())
        };
        let spec = MapSpec {
            view: camera.view,
            ..spec
        };
        let theme = Theme::kai(Density::Comfortable);
        let palette = Palette::new(&theme);
        let mut list = DrawList::new(Color::rgb(255, 255, 255));
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        };
        paint(&mut list, area, &spec, &theme);
        let colors: Vec<Color> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                DrawOp::Geometry(draw) => Some(draw.color),
                _ => None,
            })
            .collect();
        for (ink, name) in [
            (palette.earth, "earth"),
            (palette.road, "streets"),
            (palette.casing, "casings"),
            (palette.building, "buildings"),
            (palette.park, "parks"),
        ] {
            assert!(colors.contains(&ink), "{name}");
        }
        // The plaza sits in the middle of the area.
        let middle = camera.world_to_local(project(PLAZA));
        assert!((middle.0 - 300.0).abs() < 1e-6 && (middle.1 - 200.0).abs() < 1e-6);
    }
}
