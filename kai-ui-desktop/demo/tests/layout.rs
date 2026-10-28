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
    // The page starts below the bar and to the right of the sidebar.
    let bar = bar.bounds();
    let page = app.screen().get(By::test_id("counter.reset")).bounds();
    let sidebar = app.screen().get(By::test_id("sections")).bounds();
    assert!(
        bar.y + bar.height <= page.y,
        "bar {bar:?} over page {page:?}"
    );
    assert!(
        bar.x >= sidebar.x + sidebar.width,
        "bar {bar:?} beside sidebar"
    );

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
