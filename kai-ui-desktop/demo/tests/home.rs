//! /home: the counter, the form and their commands.

use kai::element::Role;
use kai::serde_json::json;
use kai::testing::{By, Key, Modifiers, TestApp};
use kai_demo::app;
use kai_demo::state::State;

fn home() -> TestApp<app::AppRoot> {
    let app = TestApp::new(app::app(State::default()));
    assert_eq!(app.route(), Some("/home"), "Inicio is the first section");
    app
}

fn name(app: &TestApp<app::AppRoot>, by: By) -> String {
    app.screen().get(by).name().unwrap_or_default()
}

const CMD: Modifiers = Modifiers {
    command: true,
    ..Modifiers::NONE
};

#[test]
fn the_counter_follows_its_button_shortcut_and_commands() {
    let mut app = home();
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    let value = || By::test_id("counter.value");
    assert!(
        !app.is_enabled(By::test_id("counter.reset")),
        "nothing to reset"
    );

    app.click(By::test_id("counter.increment"));
    app.press(Key::Char('i'), CMD);
    assert_eq!(name(&app, value()), "2");
    assert_eq!(
        app.call("counter.add", json!({ "count": 5 })),
        Ok(json!(null))
    );
    assert_eq!(name(&app, value()), "7");
    let screen = app.screen();
    screen
        .within(screen.get(By::test_id("stats")))
        .get(By::text("7"));

    app.press(Key::Char('0'), CMD);
    assert_eq!(name(&app, value()), "0");
    assert!(!app.is_enabled(By::test_id("counter.reset")));
}

#[test]
fn typing_a_name_enables_send_and_the_click_submits_it() {
    let mut app = home();
    let send = || By::test_id("form.submit");
    assert!(!app.is_enabled(send()));
    app.click(send());
    assert_eq!(app.call("form.last_submission", json!({})), Ok(json!(null)));

    app.type_text(By::label("Nombre"), "  Ana ");
    assert!(app.is_enabled(send()));
    assert_eq!(name(&app, By::test_id("form.greeting")), "Hola, Ana 👋");

    app.click(send());
    assert_eq!(app.value(By::label("Nombre")), "");
    assert!(!app.is_enabled(send()));
    app.screen().get(By::text("Último: Ana"));
    assert_eq!(
        app.call("form.last_submission", json!({})),
        Ok(json!("Ana"))
    );

    // Scripts and agents get the name back from `form.submit`.
    app.type_text(By::label("Nombre"), "Bruno");
    assert_eq!(app.call("form.submit", json!({})), Ok(json!("Bruno")));
}

#[test]
fn the_form_picks_a_role_and_accepts_the_terms_and_clear_resets_them() {
    let mut app = home();
    app.screen().get(By::text("Elegir rol"));
    app.click(By::test_id("form.role"));
    app.click(By::test_id("kai.menu.1"));
    app.click(By::test_id("form.terms"));
    app.screen().get(By::text("Desarrollo"));
    let terms = app.screen().get(By::test_id("form.terms"));
    assert_eq!(terms.role(), Role::Checkbox);

    app.click(By::test_id("form.clear"));
    app.screen().get(By::text("Elegir rol"));
}
