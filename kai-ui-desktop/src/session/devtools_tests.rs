//! The inspector (⌘⌥I): what is under the pointer, outlined and
//! described, without taking clicks. On [`Sample`] (docs/dev/27).

use crate::element::Role;
use crate::testing::sample_app::Sample;
use crate::testing::{By, Key, Modifiers, TestApp};

const CMD_OPTION: Modifiers = Modifiers {
    command: true,
    option: true,
    ..Modifiers::NONE
};

const CMD_SHIFT: Modifiers = Modifiers {
    command: true,
    shift: true,
    ..Modifiers::NONE
};

#[test]
fn the_inspector_describes_what_is_under_the_pointer() {
    let mut app = TestApp::new(Sample::default());
    assert!(app.screen().query(By::test_id("kai.inspector")).is_none());
    app.press(Key::Char('i'), CMD_OPTION);
    assert!(app.session().inspector_open());
    assert_eq!(app.messages(), [], "⌘⌥I is not ⌘I");
    app.screen().get(By::test_id("kai.inspector.panel"));
    assert!(
        app.screen()
            .query(By::test_id("kai.inspector.outline"))
            .is_none()
    );

    app.hover(By::test_id("bar.add"));
    let screen = app.screen();
    let panel = screen.get(By::test_id("kai.inspector.panel"));
    let shown: Vec<String> = screen
        .within(panel)
        .all(By::role(Role::Label))
        .iter()
        .filter_map(|line| line.name())
        .collect();
    let shown = shown.join("\n");
    assert!(
        shown.starts_with("Image #"),
        "the icon is under the pointer: {shown}"
    );
    assert!(
        shown.contains("clic → Button #") && shown.contains("«Sumar clic» [bar.add]"),
        "{shown}"
    );
    let outline = screen.get(By::test_id("kai.inspector.outline")).bounds();
    let icon = screen
        .within(screen.get(By::test_id("bar.add")))
        .get(By::role(Role::Image))
        .bounds();
    assert_eq!(
        (outline.x, outline.y, outline.width, outline.height),
        (
            icon.x.round(),
            icon.y.round(),
            icon.width.round(),
            icon.height.round()
        )
    );
    assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());

    app.click(By::test_id("bar.add"));
    assert_eq!(app.app().clicks, 1, "the inspector takes no clicks");

    app.press(Key::Char('p'), CMD_SHIFT);
    assert!(app.is_focused(By::test_id("kai.palette.find")));
    app.press(Key::Escape, Modifiers::NONE);
    assert!(!app.session().palette_open());
    assert!(app.session().inspector_open());

    app.press(Key::Char('i'), CMD_OPTION);
    assert!(app.screen().query(By::test_id("kai.inspector")).is_none());
    assert!(app.dump().contains("[bar.add]"));
}
