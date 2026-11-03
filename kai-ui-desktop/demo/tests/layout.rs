//! The bar of the root layout (`src/layout.rs`): it frames every page and
//! its buttons run the navigation commands.

use kai::runtime::App as _;
use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

fn demo() -> TestApp<app::AppRoot> {
    TestApp::new(app::app(State::default()))
}

#[test]
fn the_bar_is_above_the_page_and_names_it() {
    let mut app = demo();
    let title = app.app().title();
    let bar = app.screen().get(By::test_id("bar.title"));
    // The page's title, inside the bar (the sidebar and the page repeat it).
    app.screen().within(bar).get(By::text(&title));
    // The bar spans the window (`WRAPS_SIDEBAR`): over the sidebar and the
    // page; the page is right of the sidebar.
    let bar = bar.bounds();
    let page = app.screen().get(By::test_id("counter.reset")).bounds();
    let sidebar = app.screen().get(By::test_id("sections")).bounds();
    assert!(
        bar.y + bar.height <= page.y,
        "bar {bar:?} over page {page:?}"
    );
    assert!(
        bar.y + bar.height <= sidebar.y,
        "bar {bar:?} over sidebar {sidebar:?}"
    );
    assert!(page.x >= sidebar.x + sidebar.width);

    app.navigate("/catalog");
    let title = app.app().title();
    let bar = app.screen().get(By::test_id("bar.title"));
    app.screen().within(bar).get(By::text(&title));
}

#[test]
fn back_and_forward_are_enabled_only_when_there_is_somewhere_to_go() {
    let mut app = demo();
    assert!(!app.is_enabled(By::test_id("bar.back")));
    assert!(!app.is_enabled(By::test_id("bar.forward")));

    app.click(By::test_id("sections.1"));
    let second = app.route().map(str::to_owned);
    assert!(app.is_enabled(By::test_id("bar.back")));

    app.click(By::test_id("bar.back"));
    assert_eq!(app.route(), Some("/home"));
    assert!(app.is_enabled(By::test_id("bar.forward")));
    app.click(By::test_id("bar.forward"));
    assert_eq!(app.route().map(str::to_owned), second);
}

#[test]
fn the_sidebar_button_collapses_it_to_a_rail() {
    let mut app = demo();
    let width =
        |app: &TestApp<app::AppRoot>| app.screen().get(By::test_id("sections")).bounds().width;
    assert!(width(&app) > 48.0);
    app.click(By::test_id("bar.sidebar"));
    assert_eq!(width(&app), 48.0);
    app.click(By::test_id("bar.sidebar"));
    assert!(width(&app) > 48.0);
}

#[test]
fn a_page_brings_its_own_actions_to_the_bar_through_the_slot() {
    let mut app = demo();
    // The home page has nothing for the slot: the bar keeps its slot empty.
    app.screen().get(By::test_id("bar.actions"));
    assert!(app.screen().query(By::test_id("table.find")).is_none());

    app.navigate("/table");
    let bar = app.screen().get(By::test_id("bar.actions")).bounds();
    let find = app.screen().get(By::test_id("table.find")).bounds();
    let table = app.screen().get(By::test_id("table.clients")).bounds();
    assert!(
        find.y >= bar.y && find.y + find.height <= bar.y + bar.height,
        "the search {find:?} sits in the bar's slot {bar:?}"
    );
    assert!(
        find.y + find.height <= table.y,
        "the search {find:?} is above the table {table:?}"
    );

    // Typing in the bar filters the table, and the search is gone with the page.
    let all = app
        .screen()
        .get(By::test_id("table.clients"))
        .node()
        .row_count;
    app.type_text(By::test_id("table.find"), "temuco");
    let filtered = app
        .screen()
        .get(By::test_id("table.clients"))
        .node()
        .row_count;
    assert!(
        filtered < all,
        "{filtered:?} rows after the search, {all:?} before"
    );
    app.navigate("/catalog");
    assert!(app.screen().query(By::test_id("table.find")).is_none());
}

#[test]
fn app_real_puts_its_pill_tabs_in_the_center_of_the_bar_by_route() {
    use kai::background::BackgroundKind;
    use kai::element::ControlState;
    let mut app = demo();
    app.navigate("/real-app/clients");
    let on = |app: &mut TestApp<app::AppRoot>, id: &str| {
        app.screen().get(By::test_id(id)).node().control_state == Some(ControlState::On)
    };
    let center = app.screen().get(By::test_id("bar.center"));
    app.screen().within(center).get(By::test_id("real_app.nav"));
    assert!(on(&mut app, "real_app.nav.1"));
    app.click(By::test_id("real_app.nav.2"));
    assert_eq!(app.route(), Some("/real-app/map"));
    assert!(on(&mut app, "real_app.nav.2"));
    // The bottom bar is thinner than the top one and spans the window.
    let top = app.screen().get(By::test_id("bar")).bounds();
    let bottom = app.screen().get(By::test_id("bar.bottom")).bounds();
    assert!(bottom.height < top.height);
    assert_eq!(bottom.width, top.width);
    app.screen().get(By::test_id("real_app.status"));
    // Its still "Aurora 2".
    let background = app.app().background();
    assert_eq!(background.kind, BackgroundKind::Aurora2);
    assert!(!background.is_animated());
}
