//! /buttons: the buttons that do something, and the waits.

use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

fn buttons() -> TestApp<app::AppRoot> {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/buttons");
    app
}

fn status(app: &TestApp<app::AppRoot>, test_id: &str) -> String {
    app.screen()
        .get(By::test_id(test_id))
        .name()
        .unwrap_or_default()
}

#[test]
fn the_buttons_count_clicks_and_danger_resets_them() {
    let mut app = buttons();
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    app.click(By::test_id("buttons.secondary"));
    app.click(By::test_id("buttons.push"));
    app.click(By::test_id("buttons.icon"));
    app.click(By::test_id("buttons.disabled"));
    assert_eq!(status(&app, "buttons.clicks"), "Clics: 3");
    app.click(By::test_id("buttons.danger"));
    assert_eq!(status(&app, "buttons.clicks"), "Clics: 0");
}

#[test]
fn a_waiting_button_says_when_its_task_ends() {
    let mut app = buttons();
    assert_eq!(status(&app, "wait.state"), "Pulsa un botón: espera 1,5 s");
    app.click(By::test_id("wait.sync"));
    assert_eq!(status(&app, "wait.state"), "Sincronizando (1)…");
    app.click(By::test_id("wait.reload"));
    app.run_until_idle();
    assert_eq!(status(&app, "wait.state"), "Terminó: guardar y recargar");
    app.click(By::test_id("wait.primary"));
    app.run_until_idle();
    assert_eq!(status(&app, "wait.state"), "Terminó: Primary");
}

/// `buttons.simulate_wait` is the section's command: its button and
/// agents run it, and the agent hears back when the wait ends.
#[test]
fn the_section_command_runs_from_its_button_and_from_agents() {
    let mut app = buttons();
    let reply = app.agent_call(
        "Cursor",
        "buttons.simulate_wait",
        kai::serde_json::json!({}),
    );
    assert!(reply.result().is_none(), "open while the wait runs");
    app.run_until_idle();
    assert_eq!(reply.result(), Some(Ok(kai::serde_json::json!(null))));
    assert_eq!(status(&app, "wait.state"), "Terminó: comando");

    let mut app = buttons();
    app.click(By::test_id("buttons.simulate_wait"));
    app.run_until_idle();
    assert_eq!(status(&app, "wait.state"), "Terminó: comando");
}
