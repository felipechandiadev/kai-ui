//! The three templates: list + detail, form and data view.

use kai::element::{ControlState, Role};
use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

type App = TestApp<app::AppRoot>;

fn at(path: &str) -> App {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate(path);
    assert_eq!(app.unknown_classes(), Vec::<&str>::new(), "{path}");
    app
}

#[test]
fn the_list_shows_the_chosen_client_in_the_detail() {
    let mut app = at("/template-list");
    let name = |app: &App| {
        app.screen()
            .get(By::test_id("template.items.name"))
            .name()
            .unwrap_or_default()
    };
    assert_eq!(name(&app), "Ana Ruiz");
    app.click(By::test_id("template.items.row.3"));
    assert_eq!(name(&app), "Diego Núñez");
    assert_eq!(
        app.screen()
            .get(By::test_id("template.items.row.3"))
            .node()
            .control_state,
        Some(ControlState::On)
    );
    app.click(By::test_id("template.items.archive"));
    app.screen()
        .get(By::role(Role::Status).name("Cliente archivado"));
}

#[test]
fn the_form_saves_and_clears_its_fields() {
    let mut app = at("/template-form");
    app.type_text(By::label("Nombre"), "Ana Ruiz");
    app.type_text(By::label("Correo"), "ana@ejemplo.cl");
    app.click(By::test_id("template.form.save"));
    app.screen()
        .get(By::role(Role::Status).name("Ana Ruiz guardado"));
    assert_eq!(app.value(By::label("Nombre")), "");
    app.type_text(By::label("Notas"), "Llamar el lunes");
    app.click(By::test_id("template.form.cancel"));
    assert_eq!(app.value(By::label("Notas")), "");
}

#[test]
fn the_data_view_shows_its_metrics_and_orders() {
    let mut app = at("/template-data");
    for value in ["$ 128.4k", "3.2 %", "47"] {
        app.screen().get(By::text(value));
    }
    let screen = app.screen();
    let last = screen.within(screen.get(By::test_id("template.data.row.5")));
    last.get(By::text("Felipe Chandía"));
    app.click(By::test_id("template.data.export"));
    app.screen().get(By::text("6 operaciones"));
}
