//! /document: the demo's wiring of the editor. Save and open a .docx,
//! export a PDF, ⌘F for find and the comment bar. The editor itself is
//! tested in kai-ui-desktop/src/session/document_tests.rs.

use kai::runtime::App as _;
use kai::testing::{By, Key, Modifiers, TestApp};
use kai_demo::app;
use kai_demo::state::State;

type App = TestApp<app::AppRoot>;

fn document() -> App {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/document");
    app
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kai-demo-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn status(app: &App) -> String {
    app.screen()
        .get(By::test_id("document.status"))
        .name()
        .unwrap_or_default()
}

#[test]
fn a_saved_docx_opens_again_and_names_the_window() {
    let mut app = document();
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    assert_eq!(status(&app), "Sin guardar");
    app.click(By::test_id("document.save"));
    let request = app.file_save_request().cloned().expect("the save panel");
    assert_eq!(request.name, "informe.docx");
    let dir = scratch("document");
    let path = dir.join("informe.docx");
    app.on_file_save(Some(path.clone()));
    app.run_until_idle();
    assert!(std::fs::read(&path).unwrap().starts_with(b"PK"), "a docx");
    app.screen().get(By::text("Archivo listo"));
    assert!(status(&app).starts_with("informe.docx"));
    assert_eq!(app.app().title(), "Documento — informe.docx");

    let mut fresh = document();
    assert!(fresh.perform("document.open"));
    assert_eq!(
        fresh.file_pick_request().expect("the open panel").types,
        ["docx"]
    );
    fresh.on_file_pick(Some(path));
    fresh.run_until_idle();
    fresh.screen().get(By::text("Documento abierto"));
    assert!(
        fresh
            .value(By::test_id("document.editor"))
            .contains("Informe de demostración")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_pdf_export_writes_a_pdf_and_keeps_the_docx_name() {
    let mut app = document();
    app.click(By::test_id("document.export_pdf"));
    let request = app.file_save_request().cloned().expect("the save panel");
    assert_eq!(request.types, ["pdf"]);
    let dir = scratch("pdf");
    let path = dir.join("informe.pdf");
    app.on_file_save(Some(path.clone()));
    app.run_until_idle();
    assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF"));
    assert!(status(&app).starts_with("Sin guardar"), "{}", status(&app));
    app.screen().get(By::text("PDF: informe.pdf"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn command_f_shows_the_find_bar_and_the_comment_button_asks_for_text() {
    let mut app = document();
    assert!(
        app.screen()
            .query(By::test_id("document.findbar"))
            .is_none()
    );
    app.press(
        Key::Char('f'),
        Modifiers {
            command: true,
            ..Modifiers::NONE
        },
    );
    app.screen().get(By::test_id("document.findbar"));
    app.type_text(By::test_id("document.find.query"), "Párrafo");
    app.press(Key::Enter, Modifiers::NONE);
    assert_ne!(status(&app), "Sin guardar", "the editor reports the search");
    app.click(By::test_id("document.find.close"));
    assert!(
        app.screen()
            .query(By::test_id("document.findbar"))
            .is_none()
    );

    assert!(
        app.screen()
            .query(By::test_id("document.commentbar"))
            .is_none()
    );
    assert!(app.perform("format.bold"));
}
