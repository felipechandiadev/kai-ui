//! Layouts of the generated root (docs/dev/28): they frame the page, keep
//! their state while the route stays in their folder and are dropped when
//! it leaves.

use kai::element::Role;
use kai::testing::{By, TestApp};
use kai_sections_app::app;
use kai_sections_app::state::State;

fn crm() -> TestApp<app::AppRoot> {
    TestApp::new(app::app(State::sample()))
}

fn pressed(app: &TestApp<app::AppRoot>) -> String {
    app.screen()
        .get(By::test_id("frame.press"))
        .name()
        .unwrap_or_default()
}

#[test]
fn the_root_layout_frames_every_page_and_reads_the_route() {
    let mut app = crm();
    app.screen().get(By::test_id("frame.bar"));
    app.screen().get(By::test_id("customers.find"));
    assert_eq!(
        app.screen()
            .get(By::test_id("frame.title"))
            .name()
            .as_deref(),
        Some("Clientes")
    );
    assert_eq!(
        app.screen()
            .get(By::test_id("frame.path"))
            .name()
            .as_deref(),
        Some("/customers")
    );
    app.click(By::test_id("sections.1"));
    app.screen().get(By::test_id("frame.bar"));
    assert_eq!(
        app.screen()
            .get(By::test_id("frame.path"))
            .name()
            .as_deref(),
        Some("/sales")
    );
}

#[test]
fn the_root_layout_keeps_its_state_across_every_navigation() {
    let mut app = crm();
    app.click(By::test_id("frame.press"));
    app.click(By::test_id("frame.press"));
    assert_eq!(pressed(&app), "Pulsado 2");
    app.click(By::test_id("sections.1"));
    assert_eq!(pressed(&app), "Pulsado 2");
    assert!(app.perform("kai.back"));
    assert_eq!(pressed(&app), "Pulsado 2");
}

#[test]
fn a_section_layout_lives_only_while_the_route_is_in_its_folder() {
    let mut app = crm();
    app.screen().get(By::text("Visto al abrir: Bruno Díaz"));
    // Its page is inside it, and so is a page deeper in the folder.
    app.click(By::test_id("customers.row.1"));
    assert_eq!(app.route(), Some("/customers/1"));
    app.screen().get(By::test_id("customers.frame"));
    app.screen().get(By::test_id("customer.name"));

    // Saving an edit goes back to the list, still inside the folder: the
    // layout was created once and keeps what it saw when it opened.
    app.type_text(By::test_id("customer.name"), " Rojas");
    app.click(By::test_id("customer.save"));
    assert_eq!(app.route(), Some("/customers"));
    app.screen().get(By::text("Visto al abrir: Bruno Díaz"));

    // Leaving the folder drops it; coming back creates it again.
    app.click(By::test_id("sections.1"));
    assert!(app.screen().query(By::test_id("customers.frame")).is_none());
    app.click(By::test_id("sections.0"));
    app.screen()
        .get(By::text("Visto al abrir: Bruno Díaz Rojas"));
}

#[test]
fn nested_layouts_wrap_the_page_from_the_inside_out() {
    let app = crm();
    let tree = kai::runtime::App::view(app.app());
    assert_eq!(tree.outlet_count(), 0, "every outlet got its content");
    let text = tree.text_content();
    let bar = text.find("Pulsado 0").expect("root layout bar");
    let section = text
        .find("Visto al abrir: Bruno Díaz")
        .expect("section layout");
    let page = text
        .find("Nombre")
        .or_else(|| text.find("Ana Pérez"))
        .expect("page");
    assert!(bar < section && section < page, "{text}");
}

#[test]
fn the_root_layout_knows_whether_history_has_somewhere_to_go() {
    let history = |app: &TestApp<app::AppRoot>| {
        app.screen()
            .get(By::test_id("frame.history"))
            .name()
            .unwrap_or_default()
    };
    let mut app = crm();
    assert_eq!(history(&app), "false/false");
    app.click(By::test_id("sections.1"));
    assert_eq!(history(&app), "true/false");
    assert!(app.perform("kai.back"));
    assert_eq!(history(&app), "false/true");
}

#[test]
fn the_content_column_is_the_main_landmark_and_holds_the_page() {
    let app = crm();
    let main = app.screen().get(By::role(Role::Main));
    assert!(
        app.screen()
            .within(main)
            .query(By::test_id("customers.find"))
            .is_some(),
        "the page lives inside the main landmark"
    );
}

#[test]
fn a_layout_in_a_parameter_folder_lives_while_the_parameter_stays() {
    let visits = |app: &TestApp<app::AppRoot>| {
        app.screen()
            .get(By::test_id("customer.visits"))
            .name()
            .unwrap_or_default()
    };
    let mut app = crm();
    assert!(app.screen().query(By::test_id("customer.visits")).is_none());
    app.navigate("/customers/1");
    app.click(By::test_id("customer.visits"));
    app.click(By::test_id("customer.visits"));
    assert_eq!(visits(&app), "Visitas 2");

    // Another customer is another layout, born from zero.
    app.navigate("/customers/2");
    assert_eq!(visits(&app), "Visitas 0");

    // Leaving the folder drops it for good: back to the first customer, zero.
    app.click(By::test_id("customer.visits"));
    app.navigate("/customers");
    assert!(app.screen().query(By::test_id("customer.visits")).is_none());
    app.navigate("/customers/2");
    assert_eq!(visits(&app), "Visitas 0");
}
