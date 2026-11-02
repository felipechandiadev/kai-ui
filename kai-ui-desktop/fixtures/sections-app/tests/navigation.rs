//! The generated root behaves like the one written by hand in
//! kai-ui-desktop/src/session/pages_tests.rs.

use kai::router::Route as _;
use kai::testing::{By, TestApp};
use kai_sections_app::app::{self, Route};
use kai_sections_app::state::State;

fn crm() -> TestApp<app::AppRoot> {
    TestApp::new(app::app(State::sample()))
}

#[test]
fn every_route_round_trips_and_fixed_paths_win_over_parameters() {
    for route in [
        Route::Customers,
        Route::CustomersId { id: 4 },
        Route::CustomersNew,
        Route::Sales,
    ] {
        assert_eq!(Route::parse(&route.path()), Some(route));
    }
    assert_eq!(Route::parse("/customers/new"), Some(Route::CustomersNew));
    assert_eq!(Route::parse("/customers/ana"), None);
    assert_eq!(
        Route::CustomersId { id: 4 }.parent(),
        Some(Route::Customers)
    );
}

#[test]
fn a_page_navigates_and_changes_the_shared_state() {
    let mut app = crm();
    assert_eq!(app.route(), Some("/customers"));
    app.click(By::test_id("customers.row.1"));
    assert_eq!(app.route(), Some("/customers/1"));
    app.type_text(By::test_id("customer.name"), " Rojas");
    app.click(By::test_id("customer.save"));
    assert_eq!(app.route(), Some("/customers"));
    app.screen().get(By::text("Bruno Díaz Rojas"));

    app.click(By::test_id("customers.new"));
    app.screen().get(By::test_id("customers.new.form"));
    assert!(app.perform("kai.up"));
    assert_eq!(app.route(), Some("/customers"));
}

#[test]
fn a_section_keeps_its_state_and_the_sidebar_switches_sections() {
    let mut app = crm();
    app.type_text(By::test_id("customers.find"), "car");
    app.click(By::test_id("sections.1"));
    assert_eq!(app.route(), Some("/sales"));
    app.run_until_idle();
    assert_eq!(
        app.screen()
            .get(By::test_id("sales.total"))
            .name()
            .as_deref(),
        Some("Total 1250")
    );
    assert!(app.perform("kai.back"));
    assert_eq!(app.value(By::test_id("customers.find")), "car");
}

#[test]
fn kai_dev_reopens_the_route_on_screen_and_ignores_an_unknown_one() {
    use kai::runtime::App as _;

    let mut before = crm();
    before.navigate("/customers/1");
    let state = before.app().dev_state().unwrap();

    let mut after = app::app(State::sample());
    after.restore_dev_state(state);
    let after = TestApp::new(after);
    assert_eq!(after.route(), Some("/customers/1"));
    after.screen().get(By::test_id("customer.name"));

    let mut older = app::app(State::sample());
    older.restore_dev_state(kai::serde_json::json!({ "path": "/gone" }));
    assert_eq!(TestApp::new(older).route(), Some("/customers"));
}

#[test]
fn the_app_reopens_at_the_last_section_it_showed() {
    use kai::router::LastSection;
    use kai::store::Store;

    let dir = std::env::temp_dir().join(format!("kai-last-section-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = || Store::at(&dir);

    let fresh = TestApp::new(app::app(State::sample()).with_store(store()));
    assert_eq!(fresh.route(), Some("/customers"));

    let mut first = TestApp::new(app::app(State::sample()).with_store(store()));
    first.navigate("/customers/1");
    first.run_until_idle();
    assert_eq!(store().load::<LastSection>().unwrap(), None, "same section");
    first.click(By::test_id("sections.1"));
    first.run_until_idle();
    assert_eq!(
        store().load::<LastSection>().unwrap(),
        Some(LastSection {
            path: "/sales".into()
        })
    );

    let second = TestApp::new(app::app(State::sample()).with_store(store()));
    assert_eq!(second.route(), Some("/sales"));

    store()
        .save(&LastSection {
            path: "/gone".into(),
        })
        .unwrap();
    let older = TestApp::new(app::app(State::sample()).with_store(store()));
    assert_eq!(older.route(), Some("/customers"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// ⌃⌘S, the button at the bottom and `kai.toggle_sidebar` collapse the
/// sidebar to a rail and back; a narrow window starts with it collapsed,
/// and `kai dev` keeps it over the first window size.
#[test]
fn the_sidebar_collapses_to_a_rail() {
    use kai::runtime::App as _;
    use kai::testing::{Key, Modifiers};

    let width =
        |app: &TestApp<app::AppRoot>| app.screen().get(By::test_id("sections")).bounds().width;
    let mut app = crm();
    assert!(width(&app) > 48.0);
    let ctrl_cmd = Modifiers {
        control: true,
        command: true,
        ..Modifiers::NONE
    };
    app.press(Key::Char('s'), ctrl_cmd);
    assert_eq!(width(&app), 48.0);
    app.click(By::test_id("sections.1"));
    assert_eq!(app.route(), Some("/sales"), "the rail still navigates");
    app.click(By::test_id("sections.toggle"));
    assert!(width(&app) > 48.0);
    assert!(app.perform("kai.toggle_sidebar"));
    assert_eq!(width(&app), 48.0);

    let narrow = TestApp::builder(app::app(State::sample()))
        .viewport(560.0, 700.0)
        .build();
    assert_eq!(width(&narrow), 48.0);

    let state = app.app().dev_state().unwrap();
    let mut restarted = app::app(State::sample());
    restarted.restore_dev_state(state);
    let restarted = TestApp::builder(restarted).viewport(1024.0, 700.0).build();
    assert_eq!(width(&restarted), 48.0);
}
