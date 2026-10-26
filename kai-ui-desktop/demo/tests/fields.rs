//! /fields: typing, the eyes of the secret fields, the text area and a
//! disabled field.

use kai::element::Role;
use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

fn fields() -> TestApp<app::AppRoot> {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/fields");
    app
}

#[test]
fn the_fields_take_what_is_typed_and_their_ids_reach_the_controls() {
    let mut app = fields();
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    let screen = app.screen();
    assert_eq!(
        screen.get(By::label("Contraseña")).test_id(),
        Some("fields.password.input")
    );
    assert_eq!(
        screen.get(By::label("PIN")).test_id(),
        Some("fields.pin.input")
    );
    assert_eq!(
        screen.get(By::test_id("fields.notes.input")).role(),
        Role::TextArea
    );

    app.type_text(By::label("Monto"), "1500");
    assert_eq!(app.value(By::label("Monto")), "1500");
    app.type_text(By::test_id("fields.notes.input"), "Primera");
    assert_eq!(app.value(By::test_id("fields.notes.input")), "Primera");
    assert!(
        !app.is_enabled(By::test_id("fields.branch.input")),
        "a disabled field takes no input"
    );
}

#[test]
fn the_eye_shows_and_hides_each_secret_on_its_own() {
    let mut app = fields();
    app.type_text(By::label("Contraseña"), "secreto1");
    let toggle = By::test_id("fields.password.toggle");
    assert_eq!(
        app.screen().get(toggle.clone()).name().as_deref(),
        Some("Mostrar contraseña")
    );
    app.click(toggle.clone());
    assert_eq!(
        app.screen().get(toggle).name().as_deref(),
        Some("Ocultar contraseña")
    );
    assert_eq!(
        app.screen()
            .get(By::test_id("fields.pin.toggle"))
            .name()
            .as_deref(),
        Some("Mostrar contraseña"),
        "the PIN keeps its own eye"
    );
    assert_eq!(app.value(By::label("Contraseña")), "secreto1");
}
