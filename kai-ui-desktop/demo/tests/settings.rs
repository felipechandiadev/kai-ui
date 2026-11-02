//! The global settings of the top bar (`src/layout.rs`, docs/dev/31):
//! theme, density, size and background for every section, a page's own
//! choice first, saved in the data folder for the next run.

use kai::background::BackgroundKind;
use kai::element::ControlSize;
use kai::runtime::App as _;
use kai::store::Store;
use kai::testing::{By, TestApp};
use kai::theme::{Appearance, Density};
use kai_demo::app;
use kai_demo::settings::{BackgroundChoice, Settings, SizeChoice, ThemeChoice};
use kai_demo::state::State;

fn temp_store(name: &str) -> Store {
    Store::at(std::env::temp_dir().join(format!(
        "kai-demo-settings-{name}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    )))
}

#[test]
fn the_strip_opens_from_the_bar() {
    let mut app = TestApp::new(app::app(State::default()));
    assert!(app.screen().query(By::test_id("settings")).is_none());
    app.click(By::test_id("bar.settings"));
    let bar = app.screen().get(By::test_id("bar.title")).bounds();
    let strip = app.screen().get(By::test_id("settings")).bounds();
    let page = app.screen().get(By::test_id("counter.reset")).bounds();
    assert!(
        bar.y + bar.height <= strip.y,
        "strip {strip:?} under the bar"
    );
    assert!(
        strip.y + strip.height <= page.y,
        "page {page:?} under the strip"
    );
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    app.click(By::test_id("bar.settings"));
    assert!(app.screen().query(By::test_id("settings")).is_none());
}

#[test]
fn the_settings_theme_every_section() {
    let mut app = TestApp::new(app::app(State::default()));
    assert!(app.app().theme().is_none(), "defaults: the initial theme");
    app.click(By::test_id("bar.settings"));
    app.click(By::test_id("settings.theme.2"));
    app.click(By::test_id("settings.density.2"));
    app.click(By::test_id("settings.size.2"));
    let theme = app.app().theme().expect("the settings' theme");
    assert!(theme.is_appkit());
    assert_eq!(theme.appearance, Appearance::Dark);
    assert_eq!(theme.density, Density::Compact);
    assert_eq!(theme.control_size, ControlSize::Small);
    assert_eq!(
        app.theme().appearance,
        Appearance::Dark,
        "the window uses it"
    );

    app.navigate("/table");
    assert_eq!(app.app().theme(), Some(theme), "in another section too");
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
}

#[test]
fn a_page_with_its_own_theme_wins_while_it_is_open() {
    let mut app = TestApp::new(app::app(State::default()));
    app.click(By::test_id("bar.settings"));
    app.click(By::test_id("settings.theme.5"));
    assert!(!app.app().theme().unwrap().is_appkit(), "Kai everywhere");
    // The catalog compares presets: it starts on Glass, whatever the bar says.
    app.navigate("/catalog");
    assert!(app.app().theme().unwrap().is_appkit());
    app.navigate("/home");
    assert!(!app.app().theme().unwrap().is_appkit());
}

#[test]
fn the_settings_background_backs_pages_without_one() {
    let mut app = TestApp::new(app::app(State::default()));
    app.click(By::test_id("bar.settings"));
    app.click(By::test_id("settings.background.3"));
    assert_eq!(app.app().background().kind, BackgroundKind::Glow);
    // Inicio shows its own effect once one is chosen there.
    app.click(By::test_id("background.effect.1"));
    assert_eq!(app.app().background().kind, BackgroundKind::Mesh);
    app.click(By::test_id("background.effect.0"));
    assert_eq!(app.app().background().kind, BackgroundKind::Glow);
    app.click(By::test_id("settings.background.0"));
    assert_eq!(app.app().background().kind, BackgroundKind::None);
}

#[test]
fn the_settings_survive_a_restart() {
    let store = temp_store("restart");
    let mut app = TestApp::new(app::app(State::with_store(store.clone())));
    app.click(By::test_id("bar.settings"));
    app.click(By::test_id("settings.theme.4"));
    app.click(By::test_id("settings.size.5"));
    app.click(By::test_id("settings.background.1"));
    app.run_until_idle();

    let saved: Settings = store.load().unwrap().expect("saved");
    assert_eq!(saved.theme, ThemeChoice::AppKitDark);
    assert_eq!(saved.size, SizeChoice::ExtraLarge);
    assert_eq!(saved.background, BackgroundChoice::Mesh);

    let app = TestApp::new(app::app(State::with_store(store.clone())));
    let theme = app.app().theme().expect("the saved theme");
    assert_eq!(theme.appearance, Appearance::Dark);
    assert_eq!(theme.control_size, ControlSize::ExtraLarge);
    assert_eq!(app.app().background().kind, BackgroundKind::Mesh);
    let _ = std::fs::remove_dir_all(store.dir());
}

#[test]
fn kai_theme_wins_over_the_saved_theme_until_one_is_picked() {
    let store = temp_store("env");
    store
        .save(&Settings {
            theme: ThemeChoice::Kai,
            ..Settings::default()
        })
        .unwrap();
    let mut state = State::with_store(store.clone());
    state.env_theme = true;
    let mut app = TestApp::new(app::app(state));
    assert!(app.app().theme().is_none(), "KAI_THEME's initial theme");
    app.click(By::test_id("bar.settings"));
    app.click(By::test_id("settings.size.1"));
    app.run_until_idle();
    let saved: Settings = store.load().unwrap().unwrap();
    assert_eq!(saved.theme, ThemeChoice::Kai, "the choice on disk is kept");
    app.click(By::test_id("settings.theme.5"));
    assert!(!app.app().theme().unwrap().is_appkit());
    let _ = std::fs::remove_dir_all(store.dir());
}
