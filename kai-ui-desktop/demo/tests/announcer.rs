//! A change of route is announced: the live region follows the title.

use kai::runtime::App as _;
use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

#[test]
fn the_announcer_follows_the_page_title() {
    let mut app = TestApp::new(app::app(State::default()));
    let title = app.app().title();
    assert!(!title.is_empty());
    let found = app.screen().get(By::test_id("announcer"));
    assert_eq!(found.name().unwrap_or_default(), title);

    app.navigate("/catalog");
    let title = app.app().title();
    let found = app.screen().get(By::test_id("announcer"));
    assert_eq!(found.name().unwrap_or_default(), title);
}
