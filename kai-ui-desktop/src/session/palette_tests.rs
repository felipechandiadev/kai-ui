//! The command palette (⇧⌘P): search, selection, keyboard, pointer and
//! assistive tech. It runs on [`Sample`], not the demo (docs/dev/27).

use crate::element::{ControlState, Role};
use crate::testing::sample_app::{Msg, Sample};
use crate::testing::{By, Key, Modifiers, TestApp};

const CMD_SHIFT: Modifiers = Modifiers {
    command: true,
    shift: true,
    ..Modifiers::NONE
};

fn row(id: &str) -> By {
    By::test_id(format!("kai.palette.{id}"))
}

fn search() -> By {
    By::test_id("kai.palette.find")
}

#[test]
fn palette_opens_with_focus_in_search_and_filters() {
    let mut app = TestApp::new(Sample::default());
    app.press(Key::Char('p'), CMD_SHIFT);
    assert!(app.session().palette_open());
    assert!(app.is_focused(search()));
    assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
    let dialog = app
        .screen()
        .get(By::role(Role::Dialog).name("Paleta de comandos"))
        .test_id()
        .map(str::to_owned);
    assert_eq!(dialog.as_deref(), Some("kai.palette"));
    let rows = |app: &TestApp<Sample>| -> Vec<String> {
        let screen = app.screen();
        let panel = screen.get(By::test_id("kai.palette"));
        screen
            .within(panel)
            .all(By::role(Role::Row))
            .iter()
            .filter_map(|found| found.test_id().map(str::to_owned))
            .collect()
    };
    assert_eq!(
        rows(&app),
        [
            "kai.palette.counter.increment",
            "kai.palette.counter.reset",
            "kai.palette.form.submit",
            "kai.palette.form.clear",
            "kai.palette.form.toggle_password",
            "kai.palette.task.wait",
        ],
        "every app action, not kai.*, queries nor actions with input"
    );
    app.type_text(search(), "REIN");
    assert_eq!(rows(&app), ["kai.palette.counter.reset"]);
    assert_eq!(app.value(search()), "REIN");
    app.type_text(search(), "zzz");
    assert_eq!(rows(&app), Vec::<String>::new());
    assert!(app.screen().query(By::text("Sin resultados")).is_some());
}

#[test]
fn enter_runs_the_selected_command_only_when_enabled() {
    let mut app = TestApp::new(Sample::default());
    app.press(Key::Char('p'), CMD_SHIFT);
    app.type_text(search(), "rein");
    app.press(Key::Enter, Modifiers::NONE);
    assert_eq!(app.messages(), [], "Reiniciar is disabled");
    assert!(app.session().palette_open());

    app.press(Key::Escape, Modifiers::NONE);
    app.click(By::test_id("bar.add"));
    app.press(Key::Char('p'), CMD_SHIFT);
    assert_eq!(app.value(search()), "", "a new palette starts empty");
    app.type_text(search(), "rein");
    app.press(Key::Enter, Modifiers::NONE);
    assert_eq!(app.messages(), [Msg::Increment, Msg::Reset]);
    assert!(!app.session().palette_open());
    assert_eq!(app.app().clicks, 0);
}

#[test]
fn arrows_move_the_selection() {
    let mut app = TestApp::new(Sample::default());
    app.press(Key::Char('p'), CMD_SHIFT);
    app.press(Key::ArrowUp, Modifiers::NONE);
    app.press(Key::Enter, Modifiers::NONE);
    assert_eq!(app.messages(), [Msg::Increment], "first row: Incrementar");
    app.press(Key::Char('p'), CMD_SHIFT);
    app.press(Key::ArrowDown, Modifiers::NONE);
    app.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        app.messages(),
        [Msg::Increment, Msg::Reset],
        "second row: Reiniciar"
    );
}

#[test]
fn assistive_tech_reads_the_selected_row_while_focus_stays_in_search() {
    let mut app = TestApp::new(Sample::default());
    app.press(Key::Char('p'), CMD_SHIFT);
    let active = |app: &TestApp<Sample>| {
        let search = app.screen().get(search()).id();
        let update = app.session().a11y_tree(1.0);
        let node = |id| {
            update
                .nodes
                .iter()
                .find(|(node, _)| *node == id)
                .map(|(_, node)| node.clone())
                .unwrap()
        };
        let row = node(crate::a11y::node_id(search)).active_descendant();
        let row = row.map(|row| (row, node(row).role(), node(row).is_selected()));
        (row, update.focus == crate::a11y::node_id(search))
    };
    let first = app.screen().get(row("counter.increment")).id();
    let second = app.screen().get(row("counter.reset")).id();
    assert_eq!(
        active(&app),
        (
            Some((
                crate::a11y::node_id(first),
                accesskit::Role::Row,
                Some(true)
            )),
            true
        )
    );
    app.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(
        active(&app),
        (
            Some((
                crate::a11y::node_id(second),
                accesskit::Role::Row,
                Some(true)
            )),
            true
        ),
        "the row changes, the focus does not"
    );
    assert!(app.is_focused(search()));
    assert_eq!(
        app.screen()
            .get(row("counter.increment"))
            .node()
            .control_state,
        Some(ControlState::Off)
    );
}

#[test]
fn escape_gives_focus_back_and_app_ids_do_not_change() {
    let mut app = TestApp::new(Sample::default());
    let name = By::test_id("form.name");
    app.click(name.clone());
    let reset_id = app.screen().get(By::test_id("counter.reset")).id();
    app.press(Key::Char('p'), CMD_SHIFT);
    assert!(!app.is_focused(name.clone()));
    assert_eq!(
        app.screen().get(By::test_id("counter.reset")).id(),
        reset_id
    );
    app.press(Key::Escape, Modifiers::NONE);
    assert!(!app.session().palette_open());
    assert!(app.is_focused(name));
    assert!(app.screen().query(By::test_id("kai.palette")).is_none());

    app.press(Key::Char('p'), CMD_SHIFT);
    app.press(Key::Char('p'), CMD_SHIFT);
    assert!(!app.session().palette_open(), "its shortcut toggles it");
}

#[test]
fn tab_stays_inside_the_palette() {
    let mut app = TestApp::new(Sample::default());
    app.press(Key::Char('p'), CMD_SHIFT);
    for _ in 0..8 {
        app.press(Key::Tab, Modifiers::NONE);
        let focused = app.session().focused().expect("something has focus");
        let test_id = app
            .screen()
            .by_id(focused)
            .and_then(|found| found.test_id());
        assert!(
            test_id.is_some_and(|id| id.starts_with("kai.palette.")),
            "focus left the palette: {test_id:?}"
        );
    }
}

#[test]
fn clicks_run_a_row_or_close_on_the_backdrop() {
    let mut app = TestApp::new(Sample::default());
    app.press(Key::Char('p'), CMD_SHIFT);
    app.click(row("counter.increment"));
    assert_eq!(app.messages(), [Msg::Increment]);
    assert!(!app.session().palette_open());

    app.press(Key::Char('p'), CMD_SHIFT);
    app.click_point(crate::geometry::Point { x: 4.0, y: 4.0 });
    assert!(!app.session().palette_open());
    assert_eq!(
        app.messages(),
        [Msg::Increment],
        "the click hit nothing below"
    );
}

#[test]
fn the_palette_link_opens_the_palette() {
    let mut app = TestApp::new(Sample::default());
    let link = By::test_id("palette.open");
    assert_eq!(
        app.screen().get(link.clone()).name().as_deref(),
        Some("Abrir la paleta (⇧⌘P)")
    );
    app.click(link.clone());
    app.screen().get_by_test_id("kai.palette");
    assert!(app.is_focused(search()));
    app.press(Key::Escape, Modifiers::NONE);
    assert!(app.screen().query(By::test_id("kai.palette")).is_none());
    assert!(app.is_focused(link));
}
