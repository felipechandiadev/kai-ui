//! /components and /components/<id>: the component pages. One page per
//! component documented in docs/componentes, each with live props, the
//! code of those props («Copiar» puts it on the clipboard) and its
//! keyboard and accessibility notes.

use std::path::Path;

use kai::element::{ControlState, Role};
use kai::testing::{By, MemoryClipboard, TestApp};
use kai_demo::app;
use kai_demo::showcase::COMPONENTS;
use kai_demo::state::State;

type App = TestApp<app::AppRoot>;

fn open(path: &str) -> (App, MemoryClipboard) {
    let clipboard = MemoryClipboard::new();
    let mut app = TestApp::builder(app::app(State::default()))
        .viewport(1440.0, 1000.0)
        .clipboard(clipboard.clone())
        .build();
    app.navigate(path);
    app.run_until_idle();
    (app, clipboard)
}

/// The lines of the code block, as shown.
fn code(app: &App) -> Vec<String> {
    let screen = app.screen();
    let block = screen.get(By::test_id("components.code"));
    screen
        .within(block)
        .all(By::default())
        .iter()
        .filter_map(|line| line.name())
        .map(|line| line.trim_end().to_owned())
        .collect()
}

fn has_line(app: &App, line: &str) -> bool {
    code(app).iter().any(|shown| shown == line)
}

#[test]
fn there_is_a_page_for_every_documented_component() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/componentes");
    let mut documented: Vec<String> = std::fs::read_dir(&docs)
        .unwrap()
        .filter_map(|entry| entry.unwrap().file_name().into_string().ok())
        .filter(|name| {
            name.ends_with(".md")
                && !name.starts_with("._")
                && name != "README.md"
                && name != "elementos-comunes.md"
        })
        .collect();
    documented.sort();
    let mut covered: Vec<String> = COMPONENTS.iter().map(|c| c.doc.to_owned()).collect();
    covered.sort();
    assert_eq!(
        covered, documented,
        "each page of docs/componentes needs an entry in showcase::COMPONENTS"
    );
    for component in COMPONENTS {
        assert_eq!(format!("{}.md", component.id), component.doc);
    }
}

#[test]
fn the_index_lists_every_component_and_opens_its_page() {
    let (mut app, _) = open("/components");
    assert_eq!(app.route(), Some("/components"));
    for component in COMPONENTS {
        app.screen()
            .get(By::test_id(format!("components.item.{}", component.id)));
    }
    app.click(By::role(Role::Button).name("Switch"));
    app.run_until_idle();
    assert_eq!(app.route(), Some("/components/switch"));
    app.screen().get(By::role(Role::Heading).name("Switch"));

    app.click(By::test_id("components.index"));
    app.run_until_idle();
    assert_eq!(app.route(), Some("/components"));
}

#[test]
fn the_index_filter_narrows_the_list() {
    let (mut app, _) = open("/components");
    app.type_text(By::test_id("components.filter"), "tabla");
    app.run_until_idle();
    let screen = app.screen();
    screen.get(By::test_id("components.item.table"));
    assert!(
        screen
            .query(By::test_id("components.item.button"))
            .is_none()
    );

    app.type_text(By::test_id("components.filter"), "zzz");
    app.run_until_idle();
    app.screen().get(By::test_id("components.none"));
}

#[test]
fn every_component_page_renders_its_parts() {
    for component in COMPONENTS {
        let (app, _) = open(&component.route());
        assert_eq!(app.route(), Some(component.route().as_str()));
        let screen = app.screen();
        for part in [
            "components.header",
            "components.preview",
            "components.code",
            "components.copy",
            "components.props",
            "components.notes",
            "components.doc",
        ] {
            assert!(
                screen.query(By::test_id(part)).is_some(),
                "{}: missing {part}",
                component.id
            );
        }
        assert!(
            screen.query(By::test_id("components.live")).is_some(),
            "{}: no live instance",
            component.id
        );
        assert_eq!(
            screen.query(By::test_id("components.matrix")).is_some(),
            !component.matrix.is_empty(),
            "{}",
            component.id
        );
        for prop in component.props {
            assert!(
                screen
                    .query(By::test_id(format!("components.prop.{}", prop.key)))
                    .is_some(),
                "{}: no control for {}",
                component.id,
                prop.key
            );
        }
        assert!(!code(&app).is_empty(), "{}: no code", component.id);
        assert_eq!(
            app.unknown_classes(),
            Vec::<String>::new(),
            "{}",
            component.id
        );
    }
}

#[test]
fn an_unknown_component_says_so() {
    let (mut app, _) = open("/components/nope");
    app.screen().get(By::test_id("components.missing"));
    app.click(By::role(Role::Button).name("Ver todos los componentes"));
    app.run_until_idle();
    assert_eq!(app.route(), Some("/components"));
}

#[test]
fn a_toggle_changes_the_preview_and_the_code() {
    let (mut app, _) = open("/components/button");
    assert!(app.is_enabled(By::test_id("components.live")));
    assert!(!has_line(&app, "    .disabled(true)"));

    app.click(By::role(Role::Switch).name("Deshabilitado"));
    app.run_until_idle();
    assert!(!app.is_enabled(By::test_id("components.live")));
    assert!(has_line(&app, "    .disabled(true)"));
}

#[test]
fn a_segmented_choice_changes_the_preview_and_the_code() {
    let (mut app, _) = open("/components/button");
    app.click(By::role(Role::Radio).name("Lg"));
    app.run_until_idle();
    assert!(has_line(
        &app,
        "button_sized(\"Guardar\", ButtonVariant::Primary, ButtonSize::Lg)"
    ));
}

#[test]
fn a_text_prop_changes_the_preview_and_the_code() {
    let (mut app, _) = open("/components/button");
    app.type_text(By::test_id("components.prop.label"), " todo");
    app.run_until_idle();
    assert_eq!(
        app.screen()
            .get(By::test_id("components.live"))
            .name()
            .as_deref(),
        Some("Guardar todo")
    );
    assert!(
        code(&app)
            .iter()
            .any(|line| line.starts_with("button_sized(\"Guardar todo\", "))
    );
}

#[test]
fn a_choice_changes_the_preview_and_the_code() {
    let (mut app, _) = open("/components/badge");
    assert!(has_line(&app, "badge(\"Activo\", BadgeVariant::Success)"));
    // Six colors: a select («kai.menu» lists them).
    app.click(By::test_id("components.prop.color"));
    app.run_until_idle();
    app.click(By::test_id("kai.menu.4"));
    app.run_until_idle();
    assert!(has_line(&app, "badge(\"Activo\", BadgeVariant::Warning)"));
    app.click(By::role(Role::Switch).name("Contorno"));
    app.run_until_idle();
    assert!(has_line(
        &app,
        "badge(\"Activo\", BadgeVariant::WarningOutlined)"
    ));
}

#[test]
fn the_preview_writes_back_to_the_props() {
    let (mut app, _) = open("/components/switch");
    assert!(has_line(&app, "    self.avisos, // true"));
    // The live switch, not the «Encendido» prop control.
    app.click(By::test_id("components.live.track"));
    app.run_until_idle();
    assert!(has_line(&app, "    self.avisos, // false"));
    let screen = app.screen();
    let control = screen.get(By::role(Role::Switch).name("Encendido"));
    assert_eq!(control.node().control_state, Some(ControlState::Off));
}

#[test]
fn copy_puts_the_code_on_the_clipboard_and_says_so() {
    let (mut app, clipboard) = open("/components/button");
    app.click(By::role(Role::Switch).name("Ocupado"));
    app.run_until_idle();
    assert_eq!(clipboard.text(), None);

    app.click(By::test_id("components.copy"));
    app.run_until_idle();
    let copied = clipboard.text().expect("copied");
    assert_eq!(copied.lines().collect::<Vec<_>>(), code(&app));
    assert!(copied.starts_with("use kai::components::{button_sized, ButtonSize, ButtonVariant};"));
    assert!(copied.contains("    .busy(true)"));
    app.screen().get(By::role(Role::Button).name("Copiado"));
    app.screen().get(By::test_id("components.copied"));

    app.advance(kai_demo::sections::components::param_id::page::COPIED_FOR);
    app.run_until_idle();
    app.screen().get(By::role(Role::Button).name("Copiar"));
    assert!(
        app.screen()
            .query(By::test_id("components.copied"))
            .is_none()
    );
}

#[test]
fn props_are_kept_when_the_page_is_opened_again_and_reset() {
    let (mut app, _) = open("/components/button");
    app.click(By::role(Role::Switch).name("Deshabilitado"));
    app.run_until_idle();
    app.navigate("/components/badge");
    app.run_until_idle();
    app.navigate("/components/button");
    app.run_until_idle();
    assert!(has_line(&app, "    .disabled(true)"));

    app.click(By::test_id("components.reset"));
    app.run_until_idle();
    assert!(!has_line(&app, "    .disabled(true)"));
}

#[test]
fn the_matrix_shows_each_variant_side_by_side() {
    let (app, _) = open("/components/button");
    let screen = app.screen();
    let matrix = screen.get(By::test_id("components.matrix"));
    let cells = screen.within(matrix).all(By::role(Role::Button));
    // Five variants, three sizes, three states.
    assert_eq!(cells.len(), 11);
    assert!(!app.is_enabled(By::test_id("components.matrix.2.1")));
}

#[test]
fn a_dialog_opens_over_the_window_with_its_props() {
    let (mut app, _) = open("/components/dialog");
    app.click(By::test_id("components.live"));
    app.run_until_idle();
    app.screen()
        .get(By::role(Role::Dialog).name("¿Borrar el cliente?"));
    app.click(By::role(Role::Button).name("Cancelar"));
    app.run_until_idle();
    assert!(app.screen().query(By::role(Role::Dialog)).is_none());
    assert_eq!(
        app.screen()
            .get(By::test_id("components.event"))
            .name()
            .as_deref(),
        Some("Último evento: cancel_button ×1")
    );
}

#[test]
fn the_notes_come_from_the_component_and_link_to_the_demo() {
    let (mut app, _) = open("/components/segmented");
    let screen = app.screen();
    let keyboard = screen.get(By::test_id("components.keyboard"));
    assert!(screen.within(keyboard).all(By::default()).len() > 1);
    screen.get(By::test_id("components.a11y"));
    app.click(By::test_id("components.demo"));
    app.run_until_idle();
    assert_eq!(app.route(), Some("/selection"));
}
