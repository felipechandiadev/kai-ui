//! /badges: every variant shows its text, and the alerts are there.

use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

#[test]
fn every_badge_variant_and_both_alerts_are_shown() {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/badges");
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    let screen = app.screen();
    for row in ["badges.solid", "badges.outlined"] {
        let row = screen.within(screen.get(By::test_id(row)));
        for name in [
            "Primary",
            "Secondary",
            "Success",
            "Error",
            "Warning",
            "Info",
        ] {
            row.get(By::text(name));
        }
    }
    let alerts = screen.within(screen.get(By::test_id("badges.alerts")));
    alerts.get(By::text("Operación guardada con éxito"));
    alerts.get(By::text("Revisa los campos requeridos"));
}
