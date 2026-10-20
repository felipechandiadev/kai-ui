use std::time::Duration;

use super::camera::{FLY_DURATION, SETTLE_DELAY, ZOOM_DURATION};
use super::editor::{PolygonDraft, PolygonEdit};
use super::labels::{FADE, MapLabel};
use super::layers::MapLayer;
use super::mvt::Bucket;
use super::style::{Ink, MapStyle, StyleLayer};
use super::tiles::TileSource;
use super::{Anchor, FitRequest, map, marker};
use crate::components::{ButtonVariant, button};
use crate::element::{Element, Role, div};
use crate::geo::{Camera, LatLon, LatLonBounds, MapView, TileId};
use crate::geometry::{Point, Size};
use crate::layout::Content;
use crate::render::Color;
use crate::runtime::SimpleApp;
use crate::style::Cursor;
use crate::testing::{By, TestApp};
use crate::ui_engine::{Key, Modifiers};
use geojson::{Geometry, GeometryValue, Position};

const PLAZA: LatLon = LatLon::new(-36.1415, -71.8265);
/// Villa San Pedro, north east of the plaza: in view at zoom 14.
const PIN: LatLon = LatLon::new(-36.135, -71.815);
const SIZE: Size = Size {
    width: 400.0,
    height: 300.0,
};
const ELSEWHERE: MapView = MapView::new(LatLon::new(-36.155, -71.82), 15.0);

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Moved(MapView),
    Go,
    Area(String),
    Edit(PolygonEdit),
}

struct Atlas {
    view: MapView,
    basemap: Option<TileSource>,
    layers: Vec<MapLayer>,
    fit: Option<FitRequest>,
    draft: Option<PolygonDraft>,
}

impl Atlas {
    fn at(zoom: f64) -> Self {
        Self {
            view: MapView::new(PLAZA, zoom),
            basemap: None,
            layers: Vec::new(),
            fit: None,
            draft: None,
        }
    }
}

/// A square `d` degrees from `center` on each side.
fn square(center: LatLon, d: f64) -> MapLayer {
    let corner = |lon: f64, lat: f64| Position::from(vec![lon, lat]);
    let (lon, lat) = (center.lon, center.lat);
    MapLayer::polygon(&Geometry::new(GeometryValue::Polygon {
        coordinates: vec![vec![
            corner(lon - d, lat - d),
            corner(lon + d, lat - d),
            corner(lon + d, lat + d),
            corner(lon - d, lat + d),
        ]],
    }))
    .unwrap()
}

static LAND: MapStyle = MapStyle {
    buckets: &[Bucket::fill("earth", &[])],
    layers: &[StyleLayer::fill("earth", 0, Ink::Earth)],
};

fn parral() -> TileSource {
    TileSource::pmtiles(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/maps/parral.pmtiles"),
        &LAND,
    )
}

impl SimpleApp for Atlas {
    type Msg = Msg;

    fn title(&self) -> String {
        "Atlas".into()
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Moved(view) => self.view = view,
            Msg::Go => self.view = ELSEWHERE,
            Msg::Area(_) => {}
            Msg::Edit(edit) => {
                if let Some(draft) = &mut self.draft {
                    draft.apply(edit);
                }
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        let mut atlas = map(&self.view);
        if let Some(source) = &self.basemap {
            atlas = atlas.basemap(source.clone());
        }
        if !self.layers.is_empty() {
            atlas = atlas
                .layers(self.layers.iter().cloned())
                .on_area_click(Msg::Area);
        }
        if let Some(fit) = self.fit {
            atlas = atlas.fit_bounds(fit.bounds, fit.serial);
        }
        if let Some(draft) = &self.draft {
            atlas = atlas.polygon_editor(draft, Color::hex(0xc45c26), Msg::Edit);
        }
        div().class("flex flex-col gap-2 p-4").children([
            atlas
                .on_view_change(Msg::Moved)
                .label("Mapa")
                .test_id("mapa")
                .class("w-[400px] h-[300px]")
                .children([marker(
                    PIN,
                    Anchor::Bottom,
                    div().class("size-4").test_id("pin"),
                )]),
            button("Ir a la terminal", ButtonVariant::Secondary).on_click(Msg::Go),
        ])
    }
}

fn mapa() -> By {
    By::test_id("mapa")
}

/// Where `at` falls in the window when the map shows `view`.
fn on_screen(app: &TestApp<Atlas>, view: MapView, at: LatLon) -> Point {
    let area = app.screen().get(mapa()).bounds();
    let local = Camera::new(view, SIZE).to_local(at);
    Point {
        x: area.x + local.x,
        y: area.y + local.y,
    }
}

/// The bottom middle of the pin, the point on its coordinate.
fn pin_tip(app: &TestApp<Atlas>) -> Point {
    let pin = app.screen().get(By::test_id("pin")).bounds();
    Point {
        x: pin.x + pin.width / 2.0,
        y: pin.y + pin.height,
    }
}

#[track_caller]
fn assert_near(actual: Point, expected: Point, tolerance: f32) {
    assert!(
        (actual.x - expected.x).abs() <= tolerance && (actual.y - expected.y).abs() <= tolerance,
        "{actual:?} lejos de {expected:?}"
    );
}

fn views(app: &TestApp<Atlas>) -> Vec<MapView> {
    app.messages()
        .iter()
        .filter_map(|msg| match msg {
            Msg::Moved(view) => Some(*view),
            Msg::Go | Msg::Area(_) | Msg::Edit(_) => None,
        })
        .collect()
}

#[test]
fn markers_stay_on_their_coordinate_at_every_zoom() {
    for zoom in [3.0, 9.0, 13.0, 14.5, 17.25, 20.0] {
        let app = TestApp::new(Atlas::at(zoom));
        let view = MapView::new(PLAZA, zoom);
        assert_near(pin_tip(&app), on_screen(&app, view, PIN), 0.5);
    }
}

#[test]
fn the_map_is_a_named_group_with_zoom_buttons_and_credit() {
    let app = TestApp::new(Atlas::at(14.0));
    let found = app.screen().get(mapa());
    assert_eq!(found.role(), Role::Group);
    assert_eq!(found.name().as_deref(), Some("Mapa"));
    let screen = app.screen();
    let zoom_in = screen.get(By::test_id("mapa.zoom_in"));
    assert_eq!(zoom_in.role(), Role::Button);
    assert_eq!(zoom_in.name().as_deref(), Some("Acercar"));
    screen.get(By::test_id("mapa.zoom_out"));
    screen.get(By::test_id("mapa.attribution"));
}

#[test]
fn a_drag_pans_the_map_and_reports_once_when_released() {
    let mut app = TestApp::new(Atlas::at(14.0));
    let before = pin_tip(&app);
    app.drag_and_hold(mapa(), 60.0, -40.0);
    let moved = views(&app);
    assert_eq!(moved.len(), 1, "{moved:?}");
    assert_eq!(app.app().view, moved[0]);
    let after = pin_tip(&app);
    assert_near(
        after,
        Point {
            x: before.x + 60.0,
            y: before.y - 40.0,
        },
        0.5,
    );
    assert_near(after, on_screen(&app, moved[0], PIN), 0.5);
    app.advance(Duration::from_secs(1));
    assert_eq!(views(&app).len(), 1, "nothing glides after a held drag");
}

#[test]
fn a_flick_glides_on_and_reports_when_it_stops() {
    let mut app = TestApp::new(Atlas::at(14.0));
    let before = pin_tip(&app);
    app.drag(mapa(), 120.0, 0.0);
    assert!(app.session().animating(), "it glides");
    assert!(views(&app).is_empty(), "no report while gliding");
    let released = pin_tip(&app).x - before.x;
    assert!((released - 120.0).abs() < 0.5, "{released}");
    app.advance(Duration::from_millis(100));
    let gliding = pin_tip(&app).x - before.x;
    assert!(gliding > released + 20.0, "{released} → {gliding}");
    app.advance(Duration::from_secs(3));
    assert!(!app.session().animating());
    let moved = views(&app);
    assert_eq!(moved.len(), 1, "{moved:?}");
    assert_near(pin_tip(&app), on_screen(&app, moved[0], PIN), 0.5);
}

#[test]
fn double_clicks_zoom_around_the_pointer() {
    let mut app = TestApp::new(Atlas::at(14.0));
    let pointer = Point { x: 300.0, y: 80.0 };
    let under = Camera::new(app.app().view, SIZE).to_lat_lon(pointer);
    app.double_click_at(mapa(), pointer.x, pointer.y);
    assert!(views(&app).is_empty(), "no report while zooming");
    app.advance(ZOOM_DURATION);
    let moved = views(&app);
    assert_eq!(moved.len(), 1, "{moved:?}");
    assert_eq!(moved[0].zoom, 15.0);
    assert_near(Camera::new(moved[0], SIZE).to_local(under), pointer, 1e-3);

    app.hold_modifiers(Modifiers {
        shift: true,
        ..Modifiers::NONE
    });
    app.double_click_at(mapa(), pointer.x, pointer.y);
    app.advance(ZOOM_DURATION);
    let moved = views(&app);
    assert_eq!(moved.len(), 2, "{moved:?}");
    assert_eq!(moved[1].zoom, 14.0, "⇧ zooms out");
}

#[test]
fn wheel_scrolls_and_pinches_report_once_per_gesture() {
    let mut app = TestApp::new(Atlas::at(14.0));
    let before = pin_tip(&app);
    app.scroll_wheel_xy(mapa(), 10.0, 5.0);
    app.scroll_wheel_xy(mapa(), 10.0, 5.0);
    assert!(
        views(&app).is_empty(),
        "a scroll burst reports once it settles"
    );
    assert_near(
        pin_tip(&app),
        Point {
            x: before.x + 20.0,
            y: before.y + 10.0,
        },
        0.5,
    );
    app.advance(SETTLE_DELAY);
    assert_eq!(views(&app).len(), 1);

    let center = Point {
        x: SIZE.width / 2.0,
        y: SIZE.height / 2.0,
    };
    let under = Camera::new(app.app().view, SIZE).to_lat_lon(center);
    app.wheel_notches(mapa(), 1.0);
    app.advance(ZOOM_DURATION);
    let moved = views(&app);
    assert_eq!(moved.len(), 2);
    assert_eq!(moved[1].zoom, 15.0);
    assert_near(Camera::new(moved[1], SIZE).to_local(under), center, 1e-3);

    app.pinch(mapa(), 0.5);
    let moved = views(&app);
    assert_eq!(moved.len(), 3, "a pinch reports when it ends");
    assert!((moved[2].zoom - 14.0).abs() < 1e-9);
    assert_near(pin_tip(&app), on_screen(&app, moved[2], PIN), 0.5);
}

#[test]
fn zoom_buttons_and_keys_move_a_focused_map() {
    let mut app = TestApp::new(Atlas::at(14.0));
    app.click(By::test_id("mapa.zoom_in"));
    app.advance(ZOOM_DURATION);
    assert_eq!(views(&app).last().map(|view| view.zoom), Some(15.0));
    assert!(!app.is_focused(mapa()), "the buttons leave focus alone");

    app.click_at(mapa(), 40.0, 250.0);
    assert!(app.is_focused(mapa()));
    let before = pin_tip(&app);
    app.press(Key::ArrowLeft, Modifiers::NONE);
    app.press(Key::ArrowUp, Modifiers::NONE);
    app.advance(SETTLE_DELAY);
    assert_eq!(views(&app).len(), 2, "one report for the key burst");
    assert_near(
        pin_tip(&app),
        Point {
            x: before.x + 80.0,
            y: before.y + 80.0,
        },
        0.5,
    );
    app.press(Key::Char('-'), Modifiers::NONE);
    app.advance(ZOOM_DURATION);
    assert_eq!(views(&app).last().map(|view| view.zoom), Some(14.0));
}

/// The tiles the map draws now: stand-ins (with the tile they cover)
/// first.
fn drawn(app: &TestApp<Atlas>) -> Vec<(TileId, Option<TileId>)> {
    let id = app.screen().get(mapa()).id();
    match &app.session().layout().node(id).unwrap().content {
        Content::Map(spec) => spec
            .tiles
            .tiles
            .iter()
            .map(|placed| (placed.tile, placed.clip))
            .collect(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_basemap_loads_off_the_ui_thread_and_follows_the_camera() {
    let mut app = TestApp::new(Atlas {
        basemap: Some(parral()),
        ..Atlas::at(14.0)
    });
    assert!(app.tasks_pending(), "tiles load on the pool");
    app.run_until_idle();
    let exact = |view: MapView| -> Vec<(TileId, Option<TileId>)> {
        Camera::new(view, SIZE)
            .visible_tiles(14)
            .into_iter()
            .map(|tile| (tile, None))
            .collect()
    };
    assert_eq!(drawn(&app), exact(app.app().view));

    app.drag_and_hold(mapa(), -250.0, 0.0);
    let moved = app.app().view;
    let during = drawn(&app);
    assert!(
        during.iter().any(|(_, clip)| clip.is_none()),
        "the tiles still in view stay"
    );
    app.run_until_idle();
    assert_eq!(drawn(&app), exact(moved));

    app.double_click_at(mapa(), 200.0, 150.0);
    app.advance(ZOOM_DURATION);
    let zoomed = drawn(&app);
    assert!(
        zoomed
            .iter()
            .all(|(tile, clip)| tile.z == 14 && clip.is_some_and(|clip| clip.z == 15)),
        "zoom 14 stands in for zoom 15: {zoomed:?}"
    );
    app.run_until_idle();
    assert!(
        drawn(&app)
            .iter()
            .all(|(tile, clip)| tile.z == 15 && clip.is_none())
    );
}

fn labels(app: &TestApp<Atlas>) -> Vec<MapLabel> {
    let id = app.screen().get(mapa()).id();
    match &app.session().layout().node(id).unwrap().content {
        Content::Map(spec) => spec.labels.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn labels_fade_in_once_the_map_holds_still_and_ride_along_while_it_moves() {
    let mut app = TestApp::new(Atlas {
        basemap: Some(parral()),
        ..Atlas::at(15.0)
    });
    app.run_until_idle();
    let shown = labels(&app);
    assert!(!shown.is_empty(), "the plaza's streets are named");
    assert!(app.session().animating(), "they fade in");
    app.advance(FADE);
    let shown = labels(&app);
    assert!(shown.iter().all(|label| label.opacity == 1.0));
    assert!(!app.session().animating());

    app.drag_steps(mapa(), -120.0, 0.0);
    assert_eq!(
        labels(&app),
        shown,
        "same labels, same places, while dragging"
    );
    app.advance(Duration::from_millis(150));
    app.release_pointer();
    app.run_until_idle();
    app.advance(FADE);
    let after = labels(&app);
    assert!(!after.is_empty());
    assert!(
        after
            .iter()
            .filter(|label| shown.iter().any(|before| before.text == label.text))
            .count()
            > 0,
        "the labels still in view stay"
    );
}

#[test]
fn a_new_view_from_the_app_glides_there_without_echo() {
    let mut app = TestApp::new(Atlas::at(14.0));
    app.click(By::role(Role::Button).name("Ir a la terminal"));
    assert!(app.session().animating(), "it glides");
    app.advance(FLY_DURATION);
    assert!(!app.session().animating());
    assert_near(pin_tip(&app), on_screen(&app, ELSEWHERE, PIN), 0.5);
    assert!(
        views(&app).is_empty(),
        "the app moved it; nothing to report"
    );
}

/// The id of the layer the map shows under the pointer.
fn hovered(app: &TestApp<Atlas>) -> Option<String> {
    let id = app.screen().get(mapa()).id();
    match &app.session().layout().node(id).unwrap().content {
        Content::Map(spec) => spec.hovered.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn layers_light_up_under_the_pointer_and_report_clicks() {
    let east = LatLon::new(PLAZA.lat, -71.81);
    let mut app = TestApp::new(Atlas {
        layers: vec![
            square(PLAZA, 0.008).id("fondo"),
            square(PLAZA, 0.003)
                .id("plaza")
                .fill(Color::hex(0xc45c26), 0.12),
            square(east, 0.002),
        ],
        ..Atlas::at(14.0)
    });
    let view = app.app().view;
    let at = |app: &TestApp<Atlas>, place: LatLon| on_screen(app, view, place);
    assert_eq!(app.mouse_cursor(), Cursor::Default);

    let plaza = at(&app, PLAZA);
    app.move_pointer(plaza);
    assert_eq!(hovered(&app).as_deref(), Some("plaza"), "the topmost one");
    assert_eq!(app.mouse_cursor(), Cursor::Pointer);
    let west = at(&app, LatLon::new(PLAZA.lat, PLAZA.lon - 0.006));
    app.move_pointer(west);
    assert_eq!(hovered(&app).as_deref(), Some("fondo"));
    let without_id = at(&app, east);
    app.move_pointer(without_id);
    assert_eq!(
        hovered(&app),
        None,
        "a layer without an id is not clickable"
    );
    assert_eq!(app.mouse_cursor(), Cursor::Default);

    app.click_point(plaza);
    assert_eq!(app.messages(), [Msg::Area("plaza".to_owned())]);
    app.drag_and_hold(mapa(), -40.0, 0.0);
    assert!(
        matches!(app.messages(), [Msg::Area(_), Msg::Moved(_)]),
        "a drag is not a click: {:?}",
        app.messages()
    );
}

#[test]
fn fit_bounds_glides_to_the_box_once_and_reports_it() {
    let bounds = LatLonBounds {
        south: -36.16,
        west: -71.85,
        north: -36.13,
        east: -71.81,
    };
    let mut app = TestApp::new(Atlas {
        fit: Some(FitRequest { bounds, serial: 1 }),
        ..Atlas::at(16.0)
    });
    assert!(app.session().animating(), "it glides from the app's view");
    app.advance(FLY_DURATION);
    let [fitted] = views(&app)[..] else {
        panic!("{:?}", app.messages());
    };
    assert!(fitted.zoom < 16.0);
    for corner in [
        LatLon::new(bounds.north, bounds.west),
        LatLon::new(bounds.south, bounds.east),
    ] {
        let local = Camera::new(fitted, SIZE).to_local(corner);
        assert!(local.x >= 15.9 && local.x <= SIZE.width - 15.9, "{local:?}");
        assert!(
            local.y >= 15.9 && local.y <= SIZE.height - 15.9,
            "{local:?}"
        );
    }

    app.drag_and_hold(mapa(), -100.0, 0.0);
    let dragged = app.app().view;
    app.advance(FLY_DURATION);
    assert_eq!(
        app.app().view,
        dragged,
        "the request stands but is served once"
    );
}

/// A draft `d` degrees around the plaza: south west, south east, north
/// east and north west.
fn draft_around_plaza(d: f64) -> PolygonDraft {
    PolygonDraft::from_ring([
        LatLon::new(PLAZA.lat - d, PLAZA.lon - d),
        LatLon::new(PLAZA.lat - d, PLAZA.lon + d),
        LatLon::new(PLAZA.lat + d, PLAZA.lon + d),
        LatLon::new(PLAZA.lat + d, PLAZA.lon - d),
    ])
    .unwrap()
}

fn editing(draft: PolygonDraft) -> TestApp<Atlas> {
    TestApp::new(Atlas {
        draft: Some(draft),
        ..Atlas::at(15.0)
    })
}

fn vertex(n: usize) -> By {
    By::test_id(format!("mapa.vertex.{n}"))
}

fn center(app: &TestApp<Atlas>, by: By) -> Point {
    let bounds = app.screen().get(by).bounds();
    Point {
        x: bounds.x + bounds.width / 2.0,
        y: bounds.y + bounds.height / 2.0,
    }
}

fn ring(app: &TestApp<Atlas>) -> Vec<LatLon> {
    app.app().draft.as_ref().unwrap().ring().to_vec()
}

fn edits(app: &TestApp<Atlas>) -> Vec<PolygonEdit> {
    app.messages()
        .iter()
        .filter_map(|msg| match msg {
            Msg::Edit(edit) => Some(*edit),
            _ => None,
        })
        .collect()
}

fn offset(point: Point, dx: f32, dy: f32) -> Point {
    Point {
        x: point.x + dx,
        y: point.y + dy,
    }
}

#[test]
fn dragging_a_vertex_moves_it_and_not_the_map() {
    let mut app = editing(draft_around_plaza(0.003));
    let view = app.app().view;
    let start = ring(&app);
    for (n, at) in start.iter().enumerate() {
        assert_near(center(&app, vertex(n)), on_screen(&app, view, *at), 0.5);
    }
    let vertex_0 = app.screen().get(vertex(0));
    assert_eq!(vertex_0.role(), Role::Button);
    assert_eq!(vertex_0.name().as_deref(), Some("Vértice 1 de 4"));

    app.drag_and_hold(vertex(0), 20.0, 10.0);
    assert!(
        views(&app).is_empty(),
        "the map stays: {:?}",
        app.messages()
    );
    assert_eq!(app.app().view, view);
    let firsts: Vec<bool> = edits(&app)
        .iter()
        .map(|edit| match edit {
            PolygonEdit::Move {
                index: 0, first, ..
            } => *first,
            other => panic!("{other:?}"),
        })
        .collect();
    assert!(firsts.len() > 1 && firsts[0] && !firsts[1..].contains(&true));
    let expected = offset(on_screen(&app, view, start[0]), 20.0, 10.0);
    assert_near(on_screen(&app, view, ring(&app)[0]), expected, 0.5);
    assert_near(center(&app, vertex(0)), expected, 0.5);
    assert_eq!(ring(&app)[1..], start[1..]);

    assert!(app.is_focused(vertex(0)));
    app.press(
        Key::Char('z'),
        Modifiers {
            command: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(ring(&app), start, "⌘Z takes the whole drag back");
}

#[test]
fn keys_move_remove_and_finish_on_a_focused_vertex() {
    let mut app = editing(draft_around_plaza(0.003));
    let view = app.app().view;
    let start = ring(&app);
    app.click(vertex(2));
    assert!(app.is_focused(vertex(2)));
    assert!(edits(&app).is_empty(), "a click moves nothing");

    app.press(Key::ArrowRight, Modifiers::NONE);
    app.press(
        Key::ArrowDown,
        Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    );
    assert_near(
        on_screen(&app, view, ring(&app)[2]),
        offset(on_screen(&app, view, start[2]), 1.0, 10.0),
        0.01,
    );
    assert_eq!(
        app.app().view,
        view,
        "the arrows move the vertex, not the map"
    );

    app.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(ring(&app), [start[0], start[1], start[3]]);
    assert!(app.is_focused(vertex(2)), "the focus stays on the slot");
    app.press(Key::Delete, Modifiers::NONE);
    assert_eq!(ring(&app).len(), 3, "three is the least");

    app.press(Key::Enter, Modifiers::NONE);
    assert_eq!(edits(&app).last(), Some(&PolygonEdit::Finish));
    app.press(Key::Escape, Modifiers::NONE);
    assert_eq!(edits(&app).last(), Some(&PolygonEdit::Cancel));
}

#[test]
fn a_midpoint_adds_a_vertex_and_crossing_edges_turn_red() {
    let mut app = editing(draft_around_plaza(0.003));
    let view = app.app().view;
    let middle = app.app().draft.as_ref().unwrap().midpoint(1);
    let handle = By::test_id("mapa.midpoint.1");
    assert_near(
        center(&app, handle.clone()),
        on_screen(&app, view, middle),
        0.5,
    );
    assert_eq!(
        app.screen().get(handle.clone()).name().as_deref(),
        Some("Agregar vértice entre 2 y 3")
    );
    app.click(handle);
    assert_eq!(ring(&app).len(), 5);
    assert_eq!(ring(&app)[2], middle);
    assert_eq!(
        app.screen().get(vertex(4)).name().as_deref(),
        Some("Vértice 5 de 5")
    );

    let map_layers = |app: &TestApp<Atlas>| {
        let id = app.screen().get(mapa()).id();
        match &app.session().layout().node(id).unwrap().content {
            Content::Map(spec) => spec.layers.clone(),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(map_layers(&app).len(), 1, "the draft");
    let square = draft_around_plaza(0.003);
    let corners = square.ring();
    let bow_tie =
        PolygonDraft::from_ring([corners[0], corners[2], corners[1], corners[3]]).unwrap();
    let crossed = editing(bow_tie);
    let layers = map_layers(&crossed);
    assert_eq!(layers.len(), 3, "the draft and the two edges that cross");
    for edge in &layers[1..] {
        assert_eq!(
            edge.stroke.map(|(color, _)| color),
            Some(Color::hex(0xd64545))
        );
    }
}
