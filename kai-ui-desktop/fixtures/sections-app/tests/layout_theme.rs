//! A layout gives the window its theme and background on the routes it
//! frames, after the page's own (docs/dev/31): the customers layout asks
//! for a marked Kai theme and a glow, sales keeps the app's.

use kai::background::BackgroundKind;
use kai::runtime::App as _;
use kai::testing::TestApp;
use kai::theme::MixStyle;
use kai_sections_app::app;
use kai_sections_app::state::State;

#[test]
fn the_section_layout_themes_only_its_routes() {
    let mut app = TestApp::new(app::app(State::sample()));
    app.navigate("/sales");
    // Sales loads its data in a task: navigating again waits for it.
    app.run_until_idle();
    assert!(app.app().theme().is_none(), "sales keeps the app's theme");
    assert_eq!(app.app().background().kind, BackgroundKind::None);

    for path in ["/customers", "/customers/1"] {
        app.navigate(path);
        let theme = app.app().theme().expect("the customers layout's theme");
        assert_eq!(theme.mix, MixStyle::A, "{path}");
        assert_eq!(app.app().background().kind, BackgroundKind::Glow, "{path}");
    }

    app.navigate("/sales");
    assert!(app.app().theme().is_none());
}
