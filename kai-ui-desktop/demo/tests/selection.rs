//! /selection: «Todos» is mixed and sets every permission; the select, the
//! segmented control and the switch follow their clicks.

use kai::element::{ControlState, Role};
use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

type App = TestApp<app::AppRoot>;

fn state(app: &App, by: By) -> Option<ControlState> {
    app.screen().get(by).node().control_state
}

#[test]
fn the_all_checkbox_is_mixed_and_sets_every_permission() {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/selection");
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    let all = || By::test_id("selection.all");
    let permission = |index: usize| By::test_id(format!("selection.permission.{index}"));
    assert_eq!(state(&app, all()), Some(ControlState::Mixed));
    app.click(all());
    assert_eq!(state(&app, all()), Some(ControlState::On));
    assert_eq!(state(&app, permission(1)), Some(ControlState::On));
    app.click(all());
    assert_eq!(state(&app, permission(0)), Some(ControlState::Off));
    app.click(permission(1));
    assert_eq!(state(&app, all()), Some(ControlState::Mixed));
    assert!(!app.is_enabled(By::test_id("selection.admin")));
    assert!(!app.is_enabled(By::test_id("selection.bluetooth")));

    app.click(By::test_id("selection.country"));
    app.click(By::role(Role::MenuItem).name("Perú"));
    app.screen().get(By::text("Perú"));
    app.click(By::test_id("selection.alignment.2"));
    assert_eq!(
        state(&app, By::test_id("selection.alignment.2")),
        Some(ControlState::On)
    );
    assert_eq!(
        state(&app, By::test_id("selection.wifi")),
        Some(ControlState::On)
    );
    app.click(By::test_id("selection.wifi"));
    assert_eq!(
        state(&app, By::test_id("selection.wifi")),
        Some(ControlState::Off)
    );
}
