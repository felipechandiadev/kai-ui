//! /table: search, sort, selection, filters, columns, the row detail and
//! export, with the demo's own wiring (the pause, the toasts).

use kai::element::{ControlState, Role};
use kai::testing::{By, Key, Modifiers, TestApp};
use kai_demo::app;
use kai_demo::sections::table::page::{CLIENT_COUNT, FILTER_PAUSE, client};
use kai_demo::state::State;

type App = TestApp<app::AppRoot>;

fn table() -> App {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/table");
    app
}

fn count(app: &App) -> Option<usize> {
    app.screen()
        .get(By::test_id("table.clients"))
        .node()
        .row_count
}

fn cells(app: &App, row: usize) -> Vec<String> {
    let screen = app.screen();
    screen
        .within(screen.get(By::test_id(format!("table.clients.row.{row}"))))
        .all(By::role(Role::Cell))
        .iter()
        .filter_map(|cell| cell.name())
        .collect()
}

fn name(app: &App, test_id: &str) -> String {
    app.screen()
        .get(By::test_id(test_id))
        .name()
        .unwrap_or_default()
}

#[test]
fn the_search_filters_and_the_headers_sort_the_table() {
    let mut app = table();
    // Classes of Kai's own Table and EmptyState that tw.rs does not know
    // yet (docs/dev/27); the demo adds none.
    let known_gaps = ["kai:focus-visible:ring-inset", "text-center"];
    let unknown = app.unknown_classes();
    assert!(
        unknown.iter().all(|class| known_gaps.contains(class)),
        "{unknown:?}"
    );
    app.type_text(By::test_id("table.find"), "temuco");
    let temuco = (1..=CLIENT_COUNT)
        .map(client)
        .filter(|c| {
            [c.name.as_str(), c.city, c.plan]
                .iter()
                .any(|text| text.to_lowercase().contains("temuco"))
        })
        .count();
    assert!(temuco > 0 && temuco < CLIENT_COUNT as usize);
    assert_eq!(count(&app), Some(temuco));
    assert_eq!(cells(&app, 0)[3], "Temuco");
    assert!(name(&app, "table.total").contains("de 10000 con «temuco»"));

    // A sortable header sorts, and again reverses.
    let amount_header = || By::role(Role::ColumnHeader).name("Monto");
    let amount = |row: Vec<String>| -> u32 { row[5].trim_start_matches("$ ").parse().unwrap() };
    app.click(amount_header());
    assert!(amount(cells(&app, 0)) <= amount(cells(&app, 1)));
    app.click(amount_header());
    assert!(amount(cells(&app, 0)) >= amount(cells(&app, 1)));

    // A click selects a client, which keeps its place through sorting.
    app.click(By::test_id("table.clients.row.1"));
    let chosen = cells(&app, 1)[2].clone();
    assert_eq!(
        name(&app, "table.selection"),
        format!("Seleccionado: {chosen}")
    );
    app.click(amount_header());
    assert_eq!(
        name(&app, "table.selection"),
        format!("Seleccionado: {chosen}")
    );

    // Clearing the search shows the ten thousand again.
    app.click(By::test_id("table.find.clear"));
    assert_eq!(count(&app), Some(10_000));
}

#[test]
fn exporting_saves_the_rows_the_table_shows_where_the_person_says() {
    let mut app = table();
    app.type_text(By::test_id("table.find"), "temuco");
    app.click(By::role(Role::ColumnHeader).name("Monto"));
    app.click(By::role(Role::Button).name("Exportar"));
    let request = app.file_save_request().cloned().expect("the save panel");
    assert_eq!(request.name, "clientes.xlsx");
    assert_eq!(request.types, ["xlsx"]);

    let dir = std::env::temp_dir().join(format!("kai-demo-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("clientes.xlsx");
    app.on_file_save(Some(path.clone()));
    app.run_until_idle();
    assert!(std::fs::read(&path).unwrap().starts_with(b"PK"));
    let rows = count(&app).unwrap();
    app.screen().get(By::text("Excel exportado"));
    app.screen()
        .get(By::text(format!("{rows} filas en clientes.xlsx")));

    // The command does the same, from the menu, the palette or an agent.
    std::fs::remove_file(&path).unwrap();
    assert!(app.perform("table.export"));
    app.run_until_idle();
    assert!(path.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_columns_button_hides_a_column_and_the_search_still_looks_in_it() {
    let mut app = table();
    app.click(By::role(Role::Button).name("Columnas"));
    let items: Vec<_> = app
        .screen()
        .all(By::role(Role::MenuItemCheckBox))
        .iter()
        .filter_map(|item| item.name())
        .collect();
    assert_eq!(items, ["Ciudad", "Plan", "Monto"], "N.º and Nombre stay");
    app.click(By::role(Role::MenuItemCheckBox).name("Ciudad"));
    app.press(Key::Escape, Modifiers::NONE);
    assert!(app.is_focused(By::role(Role::Button).name("Columnas")));
    let screen = app.screen();
    let headers: Vec<_> = screen
        .within(screen.get(By::test_id("table.clients")))
        .all(By::role(Role::ColumnHeader))
        .iter()
        .filter_map(|header| header.name())
        .collect();
    assert_eq!(
        headers,
        ["Detalle", "N.º", "Nombre", "Plan", "Monto", "Acciones"]
    );

    app.type_text(By::test_id("table.find"), "temuco");
    let shown = count(&app).unwrap();
    assert!(
        shown > 0 && shown < 10_000,
        "Temuco is in the hidden column"
    );
}

#[test]
fn a_client_row_opens_its_detail_and_its_button_shows_a_toast() {
    let mut app = table();
    app.click(By::test_id("table.clients.row.0"));
    app.press(Key::ArrowRight, Modifiers::NONE);
    let first = client(1);
    let detail = app.screen().get(By::role(Role::Region));
    assert!(
        detail
            .name()
            .unwrap()
            .starts_with(&format!("1 {}", first.name))
    );
    let text = By::text(format!("Cliente N.º 1 de {}", first.city));
    assert_eq!(app.screen().all(text.clone()).len(), 1);

    app.press(Key::Tab, Modifiers::NONE);
    assert!(app.is_focused(By::test_id("table.card.1")));
    app.press(Key::Space, Modifiers::NONE);
    assert_eq!(app.screen().all(text).len(), 2, "and the toast");
}

#[test]
fn the_filter_button_shows_a_field_per_column_and_typing_waits_for_a_pause() {
    let mut app = table();
    let filter_rows = By::role(Role::Button).name("Filtrar");
    assert!(
        app.screen()
            .query(By::role(Role::SearchInput).name("Filtrar Ciudad"))
            .is_none()
    );

    app.click(filter_rows.clone());
    assert_eq!(
        app.screen().get(filter_rows.clone()).node().control_state,
        Some(ControlState::On)
    );
    let screen = app.screen();
    let fields: Vec<_> = screen
        .within(screen.get(By::test_id("table.clients")))
        .all(By::role(Role::SearchInput))
        .iter()
        .filter_map(|field| field.name())
        .collect();
    assert_eq!(
        fields,
        [
            "Filtrar Nombre",
            "Filtrar Ciudad",
            "Filtrar Plan",
            "Filtrar Monto"
        ],
        "N.º is not filterable"
    );

    app.type_text(By::test_id("table.clients.filter.2"), "temuco");
    assert_eq!(count(&app), Some(10_000), "the table waits for a pause");
    app.advance(FILTER_PAUSE);
    app.type_text(By::test_id("table.clients.filter.3"), "pro");
    app.advance(FILTER_PAUSE);
    let expected = (1..=CLIENT_COUNT)
        .map(client)
        .filter(|c| c.city == "Temuco" && c.plan == "Pro")
        .count();
    assert!(expected > 0);
    assert_eq!(count(&app), Some(expected));
    assert!(name(&app, "table.total").contains("filtrados"));

    // Turning the fields off clears the filters.
    app.click(filter_rows);
    assert_eq!(count(&app), Some(10_000));
}

#[test]
fn the_table_shows_an_avatar_by_each_name() {
    let app = table();
    let screen = app.screen();
    let row = screen.within(screen.get(By::test_id("table.clients.row.0")));
    let name = client(1).name;
    row.get(By::role(Role::Image).name(&name));
    let names: Vec<_> = row
        .all(By::role(Role::Cell))
        .iter()
        .filter_map(|cell| cell.name())
        .collect();
    assert_eq!(
        names[2], name,
        "after the chevron and N.º; the avatar stays out of the cell's name"
    );
}
