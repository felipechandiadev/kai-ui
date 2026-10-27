//! /dialogs: a dialog traps the focus over the whole window and its answer
//! shows a toast; toasts close on their own except errors, whose
//! «Reintentar» goes back to the page; the palette button opens it.

use kai::components::TOAST_DURATION;
use kai::element::Role;
use kai::serde_json::{Value, json};
use kai::testing::{By, Key, Modifiers, TestApp};
use kai_demo::app;
use kai_demo::state::State;

type App = TestApp<app::AppRoot>;

fn dialogs() -> App {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/dialogs");
    app
}

#[test]
fn a_dialog_covers_the_window_traps_the_focus_and_its_answer_shows_a_toast() {
    let mut app = dialogs();
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    let dialog = || By::role(Role::Dialog);
    app.click(By::test_id("dialogs.save"));
    assert_eq!(
        app.screen().get(dialog()).name().as_deref(),
        Some("¿Guardar los cambios?")
    );
    let overlay = app.screen().get(By::test_id("dialog")).bounds();
    let window = app.screen().get(By::test_id("sections")).bounds();
    assert_eq!(overlay.x, window.x, "over the sidebar too");

    assert!(app.is_focused(By::test_id("dialog.cancel")));
    for _ in 0..3 {
        app.press(Key::Tab, Modifiers::NONE);
        assert!(
            app.is_focused(By::test_id("dialog.cancel"))
                || app.is_focused(By::test_id("dialog.default")),
            "the focus stays in the dialog"
        );
    }
    app.press(Key::Escape, Modifiers::NONE);
    assert!(app.screen().query(dialog()).is_none());
    assert!(app.is_focused(By::test_id("dialogs.save")));
    assert!(app.screen().query(By::text("Cambios guardados")).is_none());

    app.click(By::test_id("dialogs.delete"));
    app.click(By::test_id("dialog.default"));
    assert!(app.screen().query(dialog()).is_none());
    app.screen()
        .get(By::role(Role::Status).name("Borrador eliminado"));
}

#[test]
fn toasts_close_on_their_own_but_an_error_waits_with_its_retry() {
    let mut app = dialogs();
    for kind in ["success", "warning", "info"] {
        app.click(By::test_id(format!("toasts.{kind}")));
    }
    app.click(By::test_id("toasts.error"));
    let error = || By::role(Role::Alert).name("No se pudo enviar");
    app.screen().get(error());
    app.advance(TOAST_DURATION);
    assert!(app.screen().query(By::text("Espacio casi lleno")).is_none());
    app.screen().get(error());

    app.click(By::role(Role::Button).name("Reintentar"));
    app.screen()
        .get(By::role(Role::Status).name("Cambios guardados"));

    assert_eq!(
        app.call("toasts.show", json!({ "kind": "warning" })),
        Ok(Value::Null)
    );
    app.screen()
        .get(By::role(Role::Status).name("Espacio casi lleno"));
}

#[test]
fn the_palette_button_opens_the_palette() {
    let mut app = dialogs();
    app.click(By::test_id("dialogs.palette"));
    assert!(app.is_focused(By::test_id("kai.palette.find")));
}
