//! The three templates: list + detail, form and data view.

use kai::element::{ControlState, Role};
use kai::serde_json::json;
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
fn the_form_formats_the_rut_and_blocks_saving_with_errors() {
    let mut app = at("/template-form");
    app.type_text(By::label("RUT"), "123456784");
    assert_eq!(app.value(By::label("RUT")), "12.345.678-4");
    app.click(By::test_id("template.form.save"));
    app.screen().get(By::text("RUT no válido"));
    app.screen().get(By::text("Este campo es obligatorio"));
    assert!(
        app.screen()
            .query(By::role(Role::Status).name("Cliente guardado"))
            .is_none()
    );

    app.type_text(By::label("Correo"), "ana@ejemplo");
    app.screen().get(By::text("Correo no válido"));
    app.type_text(By::label("Correo"), ".cl");
    assert!(app.screen().query(By::text("Correo no válido")).is_none());
}

#[test]
fn the_form_formats_phone_and_amount() {
    let mut app = at("/template-form");
    app.type_text(By::label("Teléfono"), "912345678");
    assert_eq!(app.value(By::label("Teléfono")), "+56 9 1234 5678");
    app.type_text(By::label("Cupo"), "250000");
    assert_eq!(app.value(By::label("Cupo")), "250.000");
    app.type_text(By::label("Nombre"), "Ana");
    assert!(app.screen().query(By::text("Teléfono no válido")).is_none());
    assert!(
        app.screen()
            .query(By::text("Debe estar entre $ 1.000 y $ 10.000.000"))
            .is_none()
    );
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

#[test]
fn saving_with_errors_shows_every_message_and_reports_the_first_field() {
    let mut app = at("/template-form");
    app.type_text(By::label("RUT"), "123456784");
    app.type_text(By::label("Cupo"), "5");
    // Salir del RUT (al Cupo) lo deja tocado: su error ya se ve.
    app.screen().get(By::text("RUT no válido"));
    let errors = app.call("client.submit", json!({})).unwrap();
    assert_eq!(
        errors,
        json!([
            {"field": "correo", "message": "Este campo es obligatorio"},
            {"field": "rut", "message": "RUT no válido"},
            {"field": "cupo", "message": "Debe estar entre $ 1.000 y $ 10.000.000"},
        ])
    );
    for message in [
        "Este campo es obligatorio",
        "RUT no válido",
        "Debe estar entre $ 1.000 y $ 10.000.000",
    ] {
        app.screen().get(By::text(message));
    }
    app.click(By::test_id("template.form.cancel"));
    assert_eq!(app.value(By::label("RUT")), "");
    assert!(app.screen().query(By::text("RUT no válido")).is_none());
}

#[test]
fn agents_fill_and_read_the_form_through_its_commands() {
    let mut app = at("/template-form");
    let errors = app
        .call(
            "client.set_field",
            json!({"name": "rut", "value": "12.345.678-5"}),
        )
        .unwrap();
    assert_eq!(
        errors,
        json!([{"field": "correo", "message": "Este campo es obligatorio"}])
    );
    assert_eq!(app.value(By::label("RUT")), "12.345.678-5");
    app.call(
        "client.set_field",
        json!({"name": "telefono", "value": "56912345678"}),
    )
    .unwrap();
    assert_eq!(app.value(By::label("Teléfono")), "+56 9 1234 5678");
    app.call(
        "client.set_field",
        json!({"name": "correo", "value": " Ana@Ejemplo.CL"}),
    )
    .unwrap();
    app.call(
        "client.set_field",
        json!({"name": "nombre", "value": "Ana"}),
    )
    .unwrap();
    assert_eq!(app.call("client.errors", json!({})), Ok(json!([])));
    let values = app.call("client.values", json!({})).unwrap();
    assert_eq!(values["rut"], "12345678-5");
    assert_eq!(values["telefono"], "+56912345678");
    assert_eq!(values["correo"], "Ana@ejemplo.cl");

    assert_eq!(app.call("client.submit", json!({})), Ok(json!([])));
    app.screen()
        .get(By::role(Role::Status).name("Ana guardado"));
    assert_eq!(app.call("client.values", json!({})).unwrap()["rut"], "");
    assert_eq!(app.call("client.last_submit", json!({})), Ok(json!([])));
}
