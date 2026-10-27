//! /charts: a click on a bar selects its month, and the page says which.

use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

#[test]
fn clicking_a_bar_shows_its_month_and_the_sparkline_is_there() {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/charts");
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    let selected = |app: &TestApp<app::AppRoot>| {
        app.screen()
            .get(By::test_id("charts.bars.selected"))
            .name()
            .unwrap_or_default()
    };
    assert_eq!(selected(&app), "Elige una barra");
    let plot = app.screen().get(By::test_id("charts.bars.plot")).bounds();
    app.click_at(
        By::test_id("charts.bars.plot"),
        plot.width * 0.92,
        plot.height * 0.5,
    );
    assert_eq!(selected(&app), "Seleccionado: Jun (11)");
    app.click_at(
        By::test_id("charts.bars.plot"),
        plot.width * 0.58,
        plot.height * 0.5,
    );
    assert_eq!(selected(&app), "Seleccionado: Abr (15)");
    assert_eq!(
        app.screen()
            .get(By::test_id("charts.sparkline.plot"))
            .name()
            .as_deref(),
        Some("Tendencia")
    );
}
