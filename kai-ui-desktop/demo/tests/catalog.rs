//! /catalog: the theme, density and size chosen at the top apply to the
//! whole window while the section is open, and only then.

use kai::element::{ControlSize, ControlState};
use kai::runtime::App as _;
use kai::testing::{By, TestApp};
use kai::theme::Density;
use kai_demo::app;
use kai_demo::state::State;

#[test]
fn the_catalog_themes_the_window_while_it_is_open() {
    let mut app = TestApp::new(app::app(State::default()));
    assert!(app.app().theme().is_none(), "Inicio keeps the app's theme");
    app.navigate("/catalog");
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    let theme = app.app().theme().expect("the catalog's theme");
    assert!(theme.is_appkit(), "Glass to start");
    assert_eq!(theme.control_size, ControlSize::Large);

    app.click(By::test_id("catalog.preset.4"));
    app.click(By::test_id("catalog.density.1"));
    app.click(By::test_id("catalog.size.1"));
    let theme = app.app().theme().unwrap();
    assert!(!theme.is_appkit(), "Kai");
    assert_eq!(theme.density, Density::Compact);
    assert_eq!(theme.control_size, ControlSize::Small);
    assert_eq!(app.unknown_classes(), Vec::<&str>::new(), "under Kai too");

    let check = || By::test_id("catalog.check");
    assert_eq!(
        app.screen().get(check()).node().control_state,
        Some(ControlState::On)
    );
    app.click(check());
    assert_eq!(
        app.screen().get(check()).node().control_state,
        Some(ControlState::Off)
    );
    app.type_text(By::test_id("catalog.field.default"), "Ana");

    app.navigate("/home");
    assert!(app.app().theme().is_none(), "back to the app's theme");
    app.navigate("/catalog");
    assert!(
        !app.app().theme().unwrap().is_appkit(),
        "it keeps its choice"
    );
}

/// «Fondo de ventana» shows one live effect under the window while the
/// catalog is open, with every control; other sections keep their own.
#[test]
fn the_catalog_switches_every_background_effect() {
    use kai::background::{
        BackgroundEffect, BackgroundKind, BackgroundPalette, BackgroundSpeed,
    };
    use kai::element::Role;

    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/catalog");
    assert_eq!(app.app().background(), BackgroundEffect::NONE);
    let animated = By::role(Role::Switch).name("Animado");
    assert!(!app.is_enabled(animated.clone()), "nothing to animate");

    for (index, kind) in BackgroundKind::ALL.into_iter().enumerate() {
        app.click(By::test_id(format!("catalog.background.effect.{index}")));
        assert_eq!(app.app().background().kind, kind);
        app.screen().get(By::test_id("catalog.background.usage"));
    }
    assert_eq!(app.app().background().kind, BackgroundKind::Glow);

    app.click(animated);
    app.click(By::test_id("catalog.background.palette.1"));
    app.click(By::test_id("catalog.background.speed.2"));
    app.click(By::test_id("catalog.background.blur.0"));
    app.click(By::test_id("catalog.background.intensity.0"));
    let effect = app.app().background();
    assert!(effect.is_animated());
    assert_eq!(effect.palette, BackgroundPalette::Mint);
    assert_eq!(effect.speed, BackgroundSpeed::Fast);
    assert_eq!(effect.blur, 40.0);
    assert_eq!(effect.intensity, 0.4);

    app.click(By::test_id("catalog.preset.3"));
    assert!(app.app().background().is_animated(), "dark keeps the effect");
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());

    app.navigate("/home");
    assert_eq!(app.app().background(), BackgroundEffect::NONE);
    app.navigate("/catalog");
    assert_eq!(app.app().background(), effect, "it keeps its choice");
}
