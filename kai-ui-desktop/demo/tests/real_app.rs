//! /real-app («App real»): the panel, the CRM (list, client page, form
//! with validation, autocompletes and calendar, save with latency) and
//! the map of clients, driven as a person would.

use std::time::Duration;

use kai::element::{ControlState, Role};
use kai::testing::{By, TestApp};
use kai_demo::app::{self, RootMsg};
use kai_demo::crm::sector_name;
use kai_demo::sections::real_app::map::page::Msg as MapMsg;
use kai_demo::state::State;

type App = TestApp<app::AppRoot>;

fn open(path: &str) -> App {
    let mut app = TestApp::builder(app::app(State::default()))
        .viewport(1280.0, 900.0)
        .build();
    app.navigate(path);
    app.run_until_idle();
    app
}

fn name_of(app: &App, test_id: &str) -> String {
    app.screen()
        .get(By::test_id(test_id))
        .name()
        .unwrap_or_default()
}

fn options(app: &App) -> Vec<String> {
    app.screen()
        .all(By::role(Role::ListBoxOption))
        .iter()
        .filter_map(|option| option.name())
        .collect()
}

fn tab(app: &mut App, name: &str) {
    app.click(By::role(Role::Radio).name(name));
    app.run_until_idle();
}

#[test]
fn the_panel_waits_for_the_server_then_shows_figures_and_activity() {
    let mut app = TestApp::builder(app::app(State::default()))
        .viewport(1280.0, 900.0)
        .build();
    app.navigate("/real-app");
    app.screen().get(By::test_id("pattern.loading"));
    app.run_until_idle();
    assert!(app.screen().query(By::test_id("pattern.loading")).is_none());
    app.screen()
        .get(By::test_id("real_app.stat.clients"))
        .node();
    assert!(app.screen().query(By::text("5 activos")).is_some());
    assert!(app.screen().query(By::text("$ 6.290.000")).is_some());
    let plot = app
        .screen()
        .get(By::test_id("real_app.sales.plot"))
        .bounds();
    app.click_at(
        By::test_id("real_app.sales.plot"),
        plot.width * 0.92,
        plot.height * 0.6,
    );
    assert_eq!(
        name_of(&app, "real_app.sales.selected"),
        "Nov: 6.3 millones"
    );
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());

    app.click(By::test_id("real_app.activity.0"));
    assert_eq!(app.route(), Some("/real-app/clients/4"));
    assert_eq!(
        name_of(&app, "real_app.client.facts.name"),
        "Camila Fuentes"
    );
}

#[test]
fn the_bar_the_list_and_the_client_pages_navigate() {
    let mut app = open("/real-app");
    tab(&mut app, "Clientes");
    assert_eq!(app.route(), Some("/real-app/clients"));
    assert_eq!(name_of(&app, "real_app.clients.facts.name"), "Ana Pérez");
    app.click(By::test_id("real_app.clients.table.row.3"));
    assert_eq!(
        name_of(&app, "real_app.clients.facts.name"),
        "Camila Fuentes"
    );
    assert_eq!(
        name_of(&app, "real_app.clients.facts.referrer"),
        "Andrés Soto"
    );
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());

    app.click(By::test_id("real_app.clients.open"));
    assert_eq!(app.route(), Some("/real-app/clients/4"));
    app.click(By::test_id("real_app.client.referrer"));
    assert_eq!(app.route(), Some("/real-app/clients/2"));
    assert_eq!(name_of(&app, "real_app.client.facts.name"), "Andrés Soto");
    app.click(By::test_id("real_app.client.back"));
    app.run_until_idle();
    assert_eq!(app.route(), Some("/real-app/clients"));

    app.type_text(By::test_id("real_app.clients.search"), "longaví");
    assert_eq!(
        app.screen().all(By::role(Role::Row)).len(),
        2,
        "the header and Carlos Muñoz"
    );
    app.type_text(By::test_id("real_app.clients.search"), "zzz");
    app.screen().get(By::test_id("real_app.clients.none"));

    app.navigate("/real-app/clients/99");
    app.screen().get(By::test_id("real_app.client.missing"));
    tab(&mut app, "Mapa");
    assert_eq!(app.route(), Some("/real-app/map"));
    tab(&mut app, "Panel");
    assert_eq!(app.route(), Some("/real-app"));
}

#[test]
fn saving_an_empty_or_wrong_form_shows_its_errors() {
    let mut app = open("/real-app/clients/new");
    app.click(By::test_id("real_app.form.save"));
    app.screen().get(By::text("Escribe el nombre"));
    app.screen().get(By::text("Elige una comuna de la lista"));
    assert_eq!(
        app.screen()
            .all(By::text("Este campo es obligatorio"))
            .len(),
        2,
        "RUT and correo"
    );
    app.run_until_idle();
    assert_eq!(
        app.route(),
        Some("/real-app/clients/new"),
        "nothing was sent"
    );

    app.type_text(By::label("RUT"), "123456784");
    app.screen().get(By::text("RUT no válido"));
    app.type_text(By::label("Próxima visita"), "01/10/2026");
    app.screen()
        .get(By::text("Debe estar entre el 02/11/2026 y el 02/11/2027"));
    app.type_text(By::label("Hora"), "2130");
    app.screen()
        .get(By::text("Debe estar entre las 08:00 y las 19:00"));

    // A referrer typed but not picked is not sent either.
    app.type_text(By::test_id("real_app.form.referido.input"), "zz");
    app.click(By::test_id("real_app.form.save"));
    app.screen()
        .get(By::text("Elige un cliente de la lista o borra el texto"));
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
}

#[test]
fn the_form_autocompletes_commune_and_referrer_and_saving_updates_the_list() {
    let mut app = open("/real-app/clients/new");
    app.type_text(By::label("Nombre"), "Gabriela Torres");
    app.type_text(By::label("RUT"), "200000005");
    app.type_text(By::label("Correo"), "gabriela@ejemplo.cl");

    // Commune: from a list, as one types.
    app.type_text(By::test_id("real_app.form.comuna.input"), "lin");
    assert_eq!(options(&app), ["Linares"]);
    app.click(By::test_id("real_app.form.comuna.option.0"));
    assert_eq!(
        app.value(By::test_id("real_app.form.comuna.input")),
        "Linares"
    );

    // Referrer: searched on the server after a pause.
    app.type_text(By::test_id("real_app.form.referido.input"), "ana");
    assert_eq!(name_of(&app, "real_app.form.referido.status"), "Buscando…");
    app.advance(Duration::from_millis(300));
    app.run_until_idle();
    assert_eq!(options(&app), ["Ana Pérez"]);
    app.click(By::test_id("real_app.form.referido.option.0"));
    assert_eq!(
        app.value(By::test_id("real_app.form.referido.input")),
        "Ana Pérez"
    );

    // Next visit from the calendar, today marked and the past disabled.
    app.click(By::test_id("real_app.form.visita.open"));
    let day = |iso: &str| By::test_id(format!("real_app.form.visita.calendar.grid.day.{iso}"));
    assert!(!app.is_enabled(day("2026-11-01")));
    app.click(day("2026-11-05"));
    assert_eq!(app.value(By::label("Próxima visita")), "05/11/2026");
    app.type_text(By::label("Hora"), "1000");
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());

    app.click(By::test_id("real_app.form.save"));
    assert_eq!(name_of(&app, "real_app.form.save"), "Guardando…");
    app.run_until_idle();
    app.screen()
        .get(By::role(Role::Status).name("Gabriela Torres guardado"));
    assert_eq!(app.route(), Some("/real-app/clients"));
    app.run_until_idle();
    assert_eq!(
        name_of(&app, "real_app.clients.facts.name"),
        "Gabriela Torres",
        "the list selects the client saved"
    );
    assert_eq!(name_of(&app, "real_app.clients.facts.commune"), "Linares");
    assert_eq!(
        name_of(&app, "real_app.clients.facts.referrer"),
        "Ana Pérez"
    );
    assert_eq!(
        name_of(&app, "real_app.clients.facts.visit"),
        "05/11/2026 a las 10:00"
    );
    assert_eq!(
        app.screen().all(By::role(Role::Row)).len(),
        10,
        "header + 9"
    );

    tab(&mut app, "Panel");
    assert!(app.screen().query(By::text("9")).is_some());
    assert_eq!(
        name_of(&app, "real_app.activity.0"),
        "Abrir Cliente nuevo: Gabriela Torres"
    );
}

#[test]
fn the_server_refuses_a_taken_rut_and_an_edit_saves_in_place() {
    let mut app = open("/real-app/clients/new");
    app.type_text(By::label("Nombre"), "Otra Ana");
    app.type_text(By::label("RUT"), "111111111");
    app.type_text(By::label("Correo"), "otra@ejemplo.cl");
    app.type_text(By::test_id("real_app.form.comuna.input"), "parr");
    app.click(By::test_id("real_app.form.comuna.option.0"));
    app.click(By::test_id("real_app.form.save"));
    app.run_until_idle();
    app.screen().get(By::text(
        "No se pudo guardar: El RUT 11.111.111-1 ya es de Ana Pérez",
    ));
    assert_eq!(app.route(), Some("/real-app/clients/new"));

    app.navigate("/real-app/clients/3");
    app.click(By::test_id("real_app.client.edit"));
    assert_eq!(app.value(By::label("Nombre")), "Bruno Díaz");
    assert_eq!(
        app.value(By::test_id("real_app.form.comuna.input")),
        "Retiro"
    );
    app.type_text(By::label("Notas"), "Llamar el lunes");
    app.click(By::test_id("real_app.form.save"));
    app.run_until_idle();
    app.screen()
        .get(By::role(Role::Status).name("Ficha guardada"));
    assert_eq!(app.route(), Some("/real-app/clients/3"));
    assert_eq!(
        name_of(&app, "real_app.client.facts.notes"),
        "Llamar el lunes"
    );
}

#[test]
fn choosing_a_client_on_the_map_highlights_it_and_its_sector() {
    let mut app = open("/real-app/map");
    assert_eq!(
        name_of(&app, "real_app.map.status"),
        "Elige un cliente en la lista o en el mapa"
    );
    app.click(By::test_id("real_app.map.client.4"));
    assert!(
        app.messages()
            .iter()
            .any(|msg| matches!(msg, RootMsg::RealAppMap(MapMsg::Select(4))))
    );
    let state = State::default();
    let camila = state.crm.client(4).unwrap();
    assert_eq!(
        name_of(&app, "real_app.map.status"),
        format!(
            "Camila Fuentes · sector {} · Parral",
            sector_name(&camila.sector)
        )
    );
    for part in ["real_app.map.client.4", "real_app.map.pin.4"] {
        assert_eq!(
            app.screen().get(By::test_id(part)).node().control_state,
            Some(ControlState::On),
            "{part}"
        );
    }
    app.click(By::test_id("real_app.map.pin.2"));
    assert!(name_of(&app, "real_app.map.status").starts_with("Andrés Soto · sector "));
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    app.click(By::test_id("real_app.map.open"));
    assert_eq!(app.route(), Some("/real-app/clients/2"));
}

#[test]
fn see_on_map_opens_the_map_on_the_client() {
    let mut app = open("/real-app/clients");
    app.click(By::test_id("real_app.clients.table.row.6"));
    app.click(By::test_id("real_app.clients.map"));
    assert_eq!(app.route(), Some("/real-app/map"));
    assert!(name_of(&app, "real_app.map.status").starts_with("Esteban Vera · sector "));
    // From the bar the map opens without a client.
    tab(&mut app, "Panel");
    tab(&mut app, "Mapa");
    assert_eq!(
        name_of(&app, "real_app.map.status"),
        "Elige un cliente en la lista o en el mapa"
    );
}
