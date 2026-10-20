use std::time::Duration;

use super::camera::{FLY_DURATION, SETTLE_DELAY, ZOOM_DURATION};
use super::{Anchor, map, marker};
use crate::components::{ButtonVariant, button};
use crate::element::{Element, Role, div};
use crate::geo::{Camera, LatLon, MapView};
use crate::geometry::{Point, Size};
use crate::runtime::SimpleApp;
use crate::testing::{By, TestApp};
use crate::ui_engine::{Key, Modifiers};

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
}

struct Atlas {
    view: MapView,
}

impl Atlas {
    fn at(zoom: f64) -> Self {
        Self {
            view: MapView::new(PLAZA, zoom),
        }
    }
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
        }
    }

    fn view(&self) -> Element<Msg> {
        div().class("flex flex-col gap-2 p-4").children([
            map(&self.view)
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
            Msg::Go => None,
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
