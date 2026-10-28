//! /map: gestures and agents move the map; sectors are chosen from the
//! list, the map and agents, colored, and their outline edited on the map
//! or replaced by an agent.

use std::time::Duration;

use kai::element::{ControlState, Role};
use kai::geo::{Camera, LatLon, LatLonBounds, MapView};
use kai::geometry::{Point, Size};
use kai::serde_json::{Value, json};
use kai::testing::Cursor;
use kai::testing::{By, Key, Modifiers, TestApp};
use kai_demo::app::{self, RootMsg};
use kai_demo::sections::map::page::{INITIAL_VIEW, MAIN_SQUARE, Msg, SECTORS};
use kai_demo::state::State;

type App = TestApp<app::AppRoot>;

const CATILLO: &str = "AOP26000002";

fn map_app() -> App {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/map");
    app
}

fn view(app: &mut App) -> MapView {
    let value = app.call("map.get_view", json!({})).unwrap();
    kai::serde_json::from_value(value).unwrap()
}

fn status(app: &App, test_id: &str) -> String {
    app.screen()
        .get(By::test_id(test_id))
        .name()
        .unwrap_or_default()
}

/// The selected sector, from the line under the map.
fn selected(app: &App) -> String {
    status(app, "map.sector")
}

/// Where `at` falls in the window on the map.
fn on_map(app: &mut App, at: LatLon) -> Point {
    let area = app.screen().get(By::test_id("map.view")).node().padding_box;
    let size = Size {
        width: area.width,
        height: area.height,
    };
    let local = Camera::new(view(app), size).to_local(at);
    Point {
        x: area.x + local.x,
        y: area.y + local.y,
    }
}

fn sector(code: &str) -> &'static kai_demo::sections::map::page::Sector {
    SECTORS.iter().find(|sector| sector.code == code).unwrap()
}

/// The outline of the sector being edited, from its vertex handles.
fn handles(app: &App) -> usize {
    app.screen()
        .all(By::role(Role::Button))
        .iter()
        .filter(|node| {
            node.test_id()
                .is_some_and(|id| id.starts_with("map.view.vertex."))
        })
        .count()
}

fn vertex(n: usize) -> By {
    By::test_id(format!("map.view.vertex.{n}"))
}

/// Select Catillo, press «Editar sector» and let the map frame it.
fn edit_catillo(app: &mut App) {
    app.click(By::test_id(format!("map.sector.{CATILLO}")));
    app.click(By::test_id("map.edit"));
    app.advance(Duration::from_secs(1));
    app.screen().get(By::test_id("map.editing"));
}

#[test]
fn the_map_follows_gestures_and_agents() {
    let mut app = map_app();
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    app.drag_and_hold(By::test_id("map.view"), -100.0, 0.0);
    let moved = app
        .messages()
        .iter()
        .find_map(|msg| match msg {
            RootMsg::Map(Msg::Moved(view)) => Some(*view),
            _ => None,
        })
        .expect("the drag is reported");
    assert!(moved.center.lon > MAIN_SQUARE.lon, "{moved:?}");
    assert_eq!(view(&mut app), moved);

    let call = json!({
        "center": {"lat": -36.155, "lon": -71.82},
        "zoom": 15,
        "animate": false,
    });
    assert_eq!(app.call("map.set_view", call), Ok(Value::Null));
    assert_eq!(
        view(&mut app),
        MapView::new(LatLon::new(-36.155, -71.82), 15.0)
    );
    assert!(!app.animating(), "it jumps");
    app.screen()
        .get(By::role(Role::Status).name("-36.1550, -71.8200 · zoom 15.0"));
    let pin = app
        .screen()
        .get(By::role(Role::Image).name("Plaza de Armas"))
        .bounds();
    let map = app.screen().get(By::test_id("map.view")).bounds();
    assert!(
        pin.y + pin.height < map.y + map.height / 2.0,
        "the plaza is north of the terminal"
    );

    let before = app.messages().len();
    app.click(By::test_id("map.back"));
    assert!(app.animating(), "it glides back");
    app.advance(Duration::from_secs(1));
    assert_eq!(view(&mut app), INITIAL_VIEW);
    assert!(
        !app.messages()[before + 1..]
            .iter()
            .any(|msg| matches!(msg, RootMsg::Map(Msg::Moved(_)))),
        "the app's moves are not reported back"
    );
}

#[test]
fn sectors_are_chosen_from_the_list_the_map_and_agents() {
    let mut app = map_app();
    let screen = app.screen();
    let list = screen.get(By::role(Role::List).name("Sectores"));
    let names: Vec<String> = screen
        .within(list)
        .all(By::role(Role::Button))
        .iter()
        .filter_map(|row| row.name())
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

    app.click(By::test_id(format!("map.sector.{CATILLO}")));
    assert!(app.animating(), "it glides to the sector");
    app.advance(Duration::from_secs(1));
    let framed = view(&mut app);
    assert!(framed.zoom > INITIAL_VIEW.zoom, "{framed:?}");
    let middle = sector(CATILLO).layer.centroid();
    assert!((framed.center.lat - middle.lat).abs() < 1e-3, "{framed:?}");
    assert!((framed.center.lon - middle.lon).abs() < 1e-3, "{framed:?}");
    assert_eq!(selected(&app), "Sector Catillo · AOP26000002 · 191,7 ha");
    assert_eq!(
        app.screen()
            .get(By::test_id(format!("map.sector.{CATILLO}")))
            .node()
            .control_state,
        Some(ControlState::On)
    );

    let everything = SECTORS
        .iter()
        .map(|sector| sector.layer.bounds())
        .reduce(LatLonBounds::union)
        .unwrap();
    assert_eq!(
        app.call("map.fit_bounds", json!({ "bounds": everything })),
        Ok(Value::Null)
    );
    app.advance(Duration::from_secs(1));
    let terminal = sector("AOP26000005").layer.centroid();
    let point = on_map(&mut app, terminal);
    app.move_pointer(point);
    assert_eq!(app.mouse_cursor(), Cursor::Pointer);
    app.click_point(point);
    assert!(selected(&app).starts_with("Sector Terminal de Buses"));

    assert_eq!(
        app.call("areas.select", json!({ "code": "AOP99" })),
        Ok(Value::Null)
    );
    assert!(
        selected(&app).starts_with("Sector Terminal de Buses"),
        "unknown"
    );
    assert_eq!(
        app.call("areas.select", json!({ "code": "AOP26000001" })),
        Ok(Value::Null)
    );
    assert!(selected(&app).starts_with("Sector Plaza de Armas"));

    app.click(By::role(Role::Switch).name("Sectores"));
    app.advance(Duration::from_secs(1));
    let square = on_map(&mut app, MAIN_SQUARE);
    app.move_pointer(square);
    assert_eq!(app.mouse_cursor(), Cursor::Default, "no sectors drawn");
}

/// The selected sector takes a color from the palette or typed; while the
/// hex is incomplete, and with «Sin color», it shows its type's.
#[test]
fn the_selected_sector_changes_color_with_the_color_field() {
    let mut app = map_app();
    let field = By::role(Role::Group).name("Color del sector");
    assert!(app.screen().query(field.clone()).is_none(), "no sector yet");
    app.click(By::test_id(format!("map.sector.{CATILLO}")));
    let for_kind = sector(CATILLO).color().to_hex();
    assert_eq!(app.value(By::test_id("map.color.hex")), for_kind);

    app.click(By::role(Role::Radio).name("Violeta"));
    assert_eq!(app.value(By::test_id("map.color.hex")), "#7c3aed");

    app.click(By::test_id("map.color.clear"));
    app.type_text(By::test_id("map.color.hex"), "#12");
    app.screen().get(By::test_id("map.color.error"));
    app.type_text(By::test_id("map.color.hex"), "3456");
    assert_eq!(app.value(By::test_id("map.color.hex")), "#123456");

    app.click(By::test_id("map.edit"));
    assert!(
        app.screen().query(field).is_none(),
        "hidden while the outline is edited"
    );
}

#[test]
fn a_sector_is_edited_on_the_map_and_kept_only_on_finish() {
    let mut app = map_app();
    let edit = By::test_id("map.edit");
    assert!(
        app.screen().get(edit.clone()).node().disabled,
        "nothing selected"
    );
    edit_catillo(&mut app);
    assert!(app.screen().get(edit.clone()).node().disabled, "editing");
    let ring = sector(CATILLO).ring_points();
    for (n, at) in ring.iter().enumerate() {
        let handle = app.screen().get(vertex(n)).bounds();
        let point = on_map(&mut app, *at);
        assert!(
            (handle.x + handle.width / 2.0 - point.x).abs() < 0.5
                && (handle.y + handle.height / 2.0 - point.y).abs() < 0.5,
            "vertex {n}: {handle:?} vs {point:?}"
        );
    }
    assert_eq!(status(&app, "map.surface"), "Superficie 191,7 ha");

    let framed = view(&mut app);
    app.drag_and_hold(vertex(0), -30.0, 20.0);
    assert_eq!(view(&mut app), framed, "the map stays put");
    assert_ne!(status(&app, "map.surface"), "Superficie 191,7 ha");

    app.click(By::test_id("map.sector.AOP26000001"));
    assert!(
        selected(&app).starts_with("Editando Catillo"),
        "no switching mid-edit"
    );

    app.click(By::test_id("map.view.midpoint.0"));
    assert_eq!(handles(&app), 5);
    app.click(By::test_id("map.finish"));
    assert!(app.screen().query(By::test_id("map.editing")).is_none());
    assert!(!app.screen().get(edit.clone()).node().disabled);
    let saved = selected(&app);
    assert_ne!(saved, "Sector Catillo · AOP26000002 · 191,7 ha", "kept");

    edit_catillo(&mut app);
    assert_eq!(handles(&app), 5, "the saved outline");
    app.drag_and_hold(vertex(1), 15.0, 0.0);
    app.press(Key::Escape, Modifiers::NONE);
    assert!(
        app.screen().query(By::test_id("map.editing")).is_none(),
        "Esc cancels"
    );
    assert_eq!(selected(&app), saved);

    edit_catillo(&mut app);
    let surface = status(&app, "map.surface");
    app.drag_and_hold(vertex(1), 15.0, 0.0);
    assert_ne!(status(&app, "map.surface"), surface);
    app.press(
        Key::Char('z'),
        Modifiers {
            command: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(status(&app, "map.surface"), surface, "⌘Z undoes");
    app.click(By::test_id("map.cancel"));
    assert_eq!(selected(&app), saved);
}

#[test]
fn crossing_edges_block_finish_until_fixed() {
    let mut app = map_app();
    edit_catillo(&mut app);
    let ring = sector(CATILLO).ring_points();
    let finish = By::test_id("map.finish");
    assert!(!app.screen().get(finish.clone()).node().disabled);
    assert!(app.screen().all(By::test_id("map.crossing")).is_empty());

    let (south_west, south_east, north_east) = (
        on_map(&mut app, ring[0]),
        on_map(&mut app, ring[1]),
        on_map(&mut app, ring[2]),
    );
    app.drag_and_hold(
        vertex(0),
        south_east.x - south_west.x + 30.0,
        (north_east.y - south_west.y) / 2.0,
    );
    assert!(app.screen().get(finish.clone()).node().disabled);
    assert_eq!(status(&app, "map.crossing"), "Los bordes se cruzan");
    app.press(Key::Enter, Modifiers::NONE);
    app.screen().get(By::test_id("map.editing"));

    app.press(
        Key::Char('z'),
        Modifiers {
            command: true,
            ..Modifiers::NONE
        },
    );
    assert!(!app.screen().get(finish).node().disabled);
    app.press(Key::Enter, Modifiers::NONE);
    assert!(
        app.screen().query(By::test_id("map.editing")).is_none(),
        "Return finishes from a vertex"
    );
    assert_eq!(selected(&app), "Sector Catillo · AOP26000002 · 191,7 ha");
}

#[test]
fn agents_replace_a_sector_outline_with_a_valid_ring() {
    let mut app = map_app();
    let mut error = |input: Value| {
        app.call("areas.edit_geometry", input)
            .expect_err("it should fail")
            .message
    };
    let bow_tie = json!({
        "code": CATILLO,
        "coordinates": [[-71.84, -36.15], [-71.82, -36.13], [-71.82, -36.15], [-71.84, -36.13]],
    });
    assert!(error(bow_tie).contains("se cruzan"));
    let unknown = json!({
        "code": "AOP99",
        "coordinates": [[-71.84, -36.15], [-71.82, -36.15], [-71.82, -36.13]],
    });
    assert!(error(unknown).contains("AOP99"));
    let short = json!({ "code": CATILLO, "coordinates": [[-71.84, -36.15], [-71.82, -36.15]] });
    assert!(error(short).contains("al menos 3"));
    assert_eq!(selected(&app), "Elige un sector en la lista o en el mapa");

    let triangle = json!({
        "code": CATILLO,
        "coordinates": [[-71.84, -36.15], [-71.82, -36.15], [-71.83, -36.13], [-71.84, -36.15]],
    });
    assert_eq!(app.call("areas.edit_geometry", triangle), Ok(Value::Null));
    assert!(
        selected(&app).starts_with("Sector Catillo"),
        "and selects it"
    );
    app.click(By::test_id("map.edit"));
    app.advance(Duration::from_secs(1));
    assert_eq!(handles(&app), 3, "the triangle");
}
