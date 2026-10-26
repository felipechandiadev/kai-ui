//! /containers: the card's actions, the tabs and one setting shown in two
//! places.

use kai::element::{ControlState, Role};
use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

type App = TestApp<app::AppRoot>;

fn stat(app: &App, text: &str) {
    let screen = app.screen();
    screen
        .within(screen.get(By::test_id("containers.stats")))
        .get(By::text(text));
}

#[test]
fn the_card_counts_its_actions_and_the_tabs_show_one_panel_at_a_time() {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/containers");
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());

    app.click(By::test_id("containers.accept"));
    app.click(By::test_id("containers.accept"));
    app.click(By::test_id("containers.discard"));
    stat(&app, "2");
    stat(&app, "1");

    // The group box and the «Cuenta» tab show the same setting; while
    // «General» is open only the group box's switch is on screen.
    stat(&app, "General");
    assert!(
        app.screen()
            .query(By::test_id("containers.tab_sync"))
            .is_none()
    );
    app.click(By::role(Role::Switch).name("Sincronizar"));
    app.click(By::test_id("containers.tabs.tab.1"));
    stat(&app, "Cuenta");
    let screen = app.screen();
    let in_tab = screen
        .within(screen.get(By::test_id("containers.tab_sync")))
        .get(By::role(Role::Switch));
    assert_eq!(in_tab.node().control_state, Some(ControlState::On));
}
