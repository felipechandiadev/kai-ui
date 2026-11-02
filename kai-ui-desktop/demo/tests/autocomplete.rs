//! /fields, Autocompletar: the client search with its pause and states,
//! and the commune list picked with the keys.

use std::time::Duration;

use kai::element::Role;
use kai::testing::{By, Key, Modifiers, TestApp};
use kai_demo::app;
use kai_demo::state::State;

fn fields() -> TestApp<app::AppRoot> {
    let mut app = TestApp::builder(app::app(State::default()))
        .viewport(1024.0, 900.0)
        .build();
    app.navigate("/fields");
    app
}

fn options(app: &TestApp<app::AppRoot>) -> Vec<String> {
    app.screen()
        .all(By::role(Role::ListBoxOption))
        .iter()
        .filter_map(|option| option.name())
        .collect()
}

#[test]
fn the_client_search_waits_shows_its_states_and_picks() {
    let mut app = fields();
    app.type_text(By::test_id("fields.client.input"), "an");
    assert_eq!(
        app.screen()
            .get(By::test_id("fields.client.status"))
            .name()
            .as_deref(),
        Some("Buscando…")
    );
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    app.advance(Duration::from_millis(300));
    app.run_until_idle();
    assert_eq!(options(&app), ["Ana Pérez", "Andrés Soto", "Anita Rojas"]);
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    app.press(Key::ArrowDown, Modifiers::NONE);
    app.press(Key::ArrowDown, Modifiers::NONE);
    app.press(Key::Enter, Modifiers::NONE);
    assert_eq!(app.value(By::test_id("fields.client.input")), "Andrés Soto");
    assert!(
        app.screen()
            .query(By::text("Cliente elegido: Andrés Soto (12.345.678-5)"))
            .is_some()
    );

    app.type_text(By::test_id("fields.client.input"), "x");
    app.press(Key::Escape, Modifiers::NONE);
    app.press(Key::Escape, Modifiers::NONE);
    app.type_text(By::test_id("fields.client.input"), "error");
    app.advance(Duration::from_millis(300));
    app.run_until_idle();
    let status = app.screen().get(By::test_id("fields.client.status"));
    assert_eq!(status.role(), Role::Alert);
    assert_eq!(
        status.name().as_deref(),
        Some("No se pudo conectar con el servidor")
    );
}

#[test]
fn the_commune_list_opens_with_down_and_filters_without_accents() {
    let mut app = fields();
    app.click(By::test_id("fields.commune.input"));
    app.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(options(&app).len(), 8, "the first eight of the list");
    app.type_text(By::test_id("fields.commune.input"), "nunoa");
    assert!(options(&app).is_empty(), "ñ is not n");
    for _ in 0..5 {
        app.press(Key::Backspace, Modifiers::NONE);
    }
    app.type_text(By::test_id("fields.commune.input"), "valpa");
    assert_eq!(options(&app), ["Valparaíso"]);
    app.click(By::test_id("fields.commune.option.0"));
    assert_eq!(app.value(By::test_id("fields.commune.input")), "Valparaíso");
    assert!(app.is_focused(By::test_id("fields.commune.input")));
}
