//! The Document widget in a running app: typing, selection, the
//! clipboard, pages, the ribbon and .docx/.pdf through file panels. On
//! [`Writer`], not the demo (docs/dev/27).

use crate::testing::document_app::{Writer, long_markdown};
use crate::testing::{By, Key, MemoryClipboard, Modifiers, TestApp};

const CMD: Modifiers = Modifiers {
    command: true,
    ..Modifiers::NONE
};

const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::NONE
};

fn editor() -> By {
    By::test_id("document.editor")
}

fn empty() -> TestApp<Writer> {
    TestApp::new(Writer::with_markdown(""))
}

/// A fresh directory for files the test writes.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kai-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn document_types_bold_and_undo() {
    let mut app = empty();
    assert_eq!(
        app.screen().get(editor()).name().as_deref(),
        Some("Informe de prueba")
    );
    app.type_text(editor(), "Hola");
    assert_eq!(app.value(editor()), "Hola");
    app.click(By::test_id("document.ribbon.cmd.select.all"));
    assert!(app.perform("format.bold"));
    assert_eq!(app.value(editor()), "Hola");
    assert!(app.perform("edit.undo"));
    assert_eq!(app.value(editor()), "Hola");
    assert!(app.perform("edit.undo"));
    assert_ne!(app.value(editor()), "Hola");
}

#[test]
fn document_pointer_drag_selects_text() {
    use crate::geometry::Point;
    let mut app = empty();
    app.type_text(editor(), "Hola mundo seleccionable");
    assert_eq!(app.value(editor()), "Hola mundo seleccionable");

    // The body starts after the 72 pt top and left margins.
    let start = app.document_page_to_local(editor(), Point { x: 72.0, y: 72.0 });
    let end = app.document_page_to_local(editor(), Point { x: 180.0, y: 72.0 });
    app.drag_at(editor(), start.x, start.y, end.x - start.x, 0.0);
    assert!(
        !app.document_selection_collapsed(editor()),
        "expected non-empty selection after pointer drag across text"
    );
}

#[test]
fn document_shift_arrows_extend_selection() {
    let mut app = empty();
    app.type_text(editor(), "abcdef");
    for _ in 0..3 {
        app.press(Key::ArrowLeft, Modifiers::NONE);
    }
    assert!(app.document_selection_collapsed(editor()));
    app.press(Key::ArrowRight, SHIFT);
    app.press(Key::ArrowRight, SHIFT);
    assert!(
        !app.document_selection_collapsed(editor()),
        "Shift+Arrow should extend Document selection"
    );
}

#[test]
fn document_clipboard_copy_cut_paste_plain() {
    use crate::render::DrawOp;
    let clipboard = MemoryClipboard::new();
    let mut app = TestApp::builder(Writer::with_markdown(""))
        .clipboard(clipboard.clone())
        .build();
    app.type_text(editor(), "ClipDoc");
    assert_eq!(app.value(editor()), "ClipDoc");
    app.click(By::test_id("document.ribbon.cmd.select.all"));
    app.press(Key::C, CMD);
    assert_eq!(clipboard.text().as_deref(), Some("ClipDoc"));
    app.press(Key::X, CMD);
    assert!(
        !app.value(editor()).contains("ClipDoc"),
        "cut should clear: {:?}",
        app.value(editor())
    );
    assert_eq!(clipboard.text().as_deref(), Some("ClipDoc"));
    app.press(Key::V, CMD);
    assert!(
        app.value(editor()).contains("ClipDoc"),
        "paste plain: {:?}",
        app.value(editor())
    );
    // The body paints glyphs as images (atlas), not DrawOp::Text runs.
    let list = app.draw_list();
    let images = list
        .ops
        .iter()
        .filter(|op| matches!(op, DrawOp::Image(_)))
        .count();
    assert!(images > 0, "expected glyph/image draws after paste");
}

#[test]
fn document_clipboard_copy_preserves_bold_via_html() {
    let clipboard = MemoryClipboard::new();
    let mut app = TestApp::builder(Writer::with_markdown(""))
        .clipboard(clipboard.clone())
        .build();
    app.type_text(editor(), "RichDoc");
    app.click(By::test_id("document.ribbon.cmd.select.all"));
    assert!(app.perform("format.bold"));
    app.press(Key::C, CMD);
    assert_eq!(clipboard.text().as_deref(), Some("RichDoc"));
    assert!(
        clipboard.html().is_some_and(|h| h.contains("RichDoc")),
        "⌘C must publish HTML for rich interop: {:?}",
        clipboard.html()
    );
    // Cut and paste: the markdown keeps the bold.
    app.press(Key::X, CMD);
    app.press(Key::V, CMD);
    let md = app.app().markdown.clone();
    assert!(
        md.contains("**") && md.contains("RichDoc"),
        "paste after cut should keep bold in persisted markdown: {md:?}"
    );
}

#[test]
fn document_multipage_scrolls_and_keeps_hit_target() {
    let mut app = TestApp::new(Writer::with_markdown(long_markdown()));
    let before = app.value(editor());
    assert!(before.contains("Párrafo 40"), "len={}", before.len());
    app.scroll_wheel(editor(), -480.0);
    app.click(editor());
    app.type_text(editor(), "¶");
    let after = app.value(editor());
    assert!(after.contains('¶'), "typing after scroll should insert");
    assert_ne!(after, before);
}

#[test]
fn document_inserts_table_and_picture_from_toolbar() {
    let mut app = empty();
    app.click(editor());
    assert!(app.perform("insert.table"));
    assert!(app.perform("insert.picture"));
    // Undo picture then table keeps the editor responsive.
    assert!(app.perform("edit.undo"));
    assert!(app.perform("edit.undo"));
    app.type_text(editor(), "ok");
    assert!(app.value(editor()).contains("ok"));
}

#[test]
fn document_ribbon_bold_and_insert_table() {
    let mut app = empty();
    app.type_text(editor(), "Ribbon");
    assert_eq!(app.value(editor()), "Ribbon");
    app.click(By::test_id("document.ribbon.cmd.select.all"));
    app.click(By::test_id("document.ribbon.cmd.format.bold"));
    assert_eq!(app.value(editor()), "Ribbon");
    app.click(By::test_id("document.ribbon.tab.insert"));
    app.click(By::test_id("document.ribbon.cmd.insert.table"));
    assert!(app.perform("edit.undo"));
    app.type_text(editor(), "!");
    assert!(app.value(editor()).contains('!'));
}

#[test]
fn document_contextual_tabs_follow_table_and_picture() {
    let mut app = empty();
    let tab = |id: &str| By::test_id(format!("document.ribbon.tab.{id}"));
    app.type_text(editor(), "Texto");
    assert!(app.screen().try_get(tab("table")).is_err());
    assert!(app.screen().try_get(tab("picture")).is_err());

    app.click(tab("insert"));
    app.click(By::test_id("document.ribbon.cmd.insert.table"));
    assert!(
        app.screen().try_get(tab("table")).is_ok(),
        "caret en la tabla"
    );
    app.click(tab("table"));
    app.click(By::test_id("document.ribbon.cmd.table.style:TableGrid"));

    app.click(tab("insert"));
    app.click(By::test_id("document.ribbon.cmd.insert.picture"));
    assert!(
        app.screen().try_get(tab("picture")).is_ok(),
        "imagen seleccionada"
    );
    app.click(tab("picture"));
    app.click(By::test_id("document.ribbon.cmd.arrange.wrap:square"));
}

#[test]
fn document_ribbon_collapses_with_the_window_width() {
    let at = |width: f32| {
        TestApp::builder(Writer::with_markdown(""))
            .viewport(width, 800.0)
            .build()
    };
    let shown = |app: &TestApp<Writer>, by: By| app.screen().query(by).is_some();
    let gallery = || By::test_id("document.ribbon.cmd.para.heading2");
    let styles = || By::test_id("document.ribbon.styles");
    let paste_label = || By::text("Pegar");
    let group_label = || By::text("Portapapeles");

    let wide = at(1400.0);
    assert!(shown(&wide, gallery()) && !shown(&wide, styles()));
    assert!(shown(&wide, paste_label()) && shown(&wide, group_label()));

    // < xl: the gallery becomes a pop-up.
    let large = at(1100.0);
    assert!(!shown(&large, gallery()) && shown(&large, styles()));
    assert!(shown(&large, paste_label()));

    // < lg: the large buttons keep only their icon.
    let medium = at(900.0);
    assert!(!shown(&medium, paste_label()) && shown(&medium, group_label()));
    assert!(shown(
        &medium,
        By::test_id("document.ribbon.cmd.edit.paste")
    ));

    // < md: no group labels.
    let narrow = at(700.0);
    assert!(!shown(&narrow, group_label()));
}

#[test]
fn document_new_comment_asks_for_its_text() {
    let mut app = empty();
    let bar = || By::test_id("document.commentbar");
    assert!(app.screen().try_get(bar()).is_err());
    app.click(By::test_id("document.ribbon.tab.review"));
    app.click(By::test_id("document.ribbon.cmd.review.newComment"));
    assert!(app.screen().try_get(bar()).is_ok());
    app.click(By::test_id("document.comment.close"));
    assert!(app.screen().try_get(bar()).is_err());
}

#[test]
fn document_ribbon_shows_bold_pressed_and_cut_disabled() {
    let mut app = empty();
    let has = |ids: &[String], id: &str| ids.iter().any(|it| it == id);
    app.type_text(editor(), "Vivo");
    assert!(
        has(&app.app().ribbon_disabled, "edit.cut"),
        "cut disabled without selection: {:?}",
        app.app().ribbon_disabled
    );
    app.click(By::test_id("document.ribbon.cmd.select.all"));
    app.click(By::test_id("document.ribbon.cmd.format.bold"));
    assert!(
        has(&app.app().ribbon_pressed, "format.bold"),
        "bold must show pressed: {:?}",
        app.app().ribbon_pressed
    );
    assert!(
        !has(&app.app().ribbon_disabled, "edit.cut"),
        "cut enabled with selection: {:?}",
        app.app().ribbon_disabled
    );
    // The ribbon button is pressed in the tree.
    let tree = crate::runtime::build_view(app.app());
    let mut bold_pressed = false;
    tree.walk(&mut |el| {
        if el.test_id.as_deref() == Some("document.ribbon.cmd.format.bold") {
            bold_pressed = el.is_pressed();
        }
    });
    assert!(bold_pressed, "ribbon bold button should be pressed");
}

#[test]
fn document_ribbon_layout_review_view_tabs() {
    use crate::document::{RibbonMode, ribbon_catalog};
    let tabs = ribbon_catalog(RibbonMode::Full);
    for id in ["layout", "review", "view", "references"] {
        assert!(tabs.iter().any(|t| t.id == id), "{id}");
    }

    let mut app = empty();
    app.click(editor());
    app.type_text(editor(), "Tabs");
    for (tab, cmd) in [
        ("layout", "layout.orientation"),
        ("review", "review.wordCount"),
        ("view", "view.zoomIn"),
        ("references", "references.footnote"),
    ] {
        app.click(By::test_id(format!("document.ribbon.tab.{tab}")));
        app.click(By::test_id(format!("document.ribbon.cmd.{cmd}")));
    }
    assert!(
        app.value(editor()).contains("Tabs"),
        "editor should stay usable after ribbon cmds"
    );
}

#[test]
fn document_save_docx_via_file_panel() {
    let mut app = TestApp::new(Writer::with_markdown("Hola archivo"));
    app.click(By::test_id("document.save"));
    let request = app.file_save_request().cloned().expect("save panel");
    assert_eq!(request.name, "informe.docx");
    assert_eq!(request.types, ["docx"]);

    let dir = scratch("doc-save");
    let path = dir.join("informe.docx");
    app.on_file_save(Some(path.clone()));
    app.run_until_idle();
    assert!(!app.app().file_busy);
    let bytes = std::fs::read(&path).expect("docx written");
    assert!(bytes.starts_with(b"PK"), "docx zip");
    app.screen().get(By::text("Archivo listo"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn document_open_docx_via_file_panel() {
    // Write a .docx with «Guardar», then open it in a fresh app.
    let mut app = TestApp::new(Writer::with_markdown("Desde disco"));
    app.click(By::test_id("document.save"));
    let dir = scratch("doc-open");
    let path = dir.join("seed.docx");
    app.on_file_save(Some(path.clone()));
    app.run_until_idle();
    assert!(path.exists());

    let mut app = empty();
    app.click(By::test_id("document.open"));
    let request = app.file_pick_request().cloned().expect("open panel");
    assert_eq!(request.types, ["docx"]);
    app.on_file_pick(Some(path.clone()));
    app.run_until_idle();
    assert!(!app.app().file_busy);
    let value = app.value(editor());
    assert!(
        value.contains("Desde disco"),
        "opened docx should restore text; got {value:?}"
    );
    app.screen().get(By::text("Documento abierto"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn document_export_pdf_via_file_panel() {
    let mut app = TestApp::new(Writer::with_markdown("PDF de prueba"));
    assert!(app.perform("document.export_pdf"));
    let request = app.file_save_request().cloned().expect("pdf panel");
    assert_eq!(request.name, "informe.pdf");
    assert_eq!(request.types, ["pdf"]);
    let dir = scratch("doc-pdf");
    let path = dir.join("informe.pdf");
    app.on_file_save(Some(path.clone()));
    app.run_until_idle();
    let bytes = std::fs::read(&path).expect("pdf written");
    assert!(bytes.starts_with(b"%PDF"), "got {} bytes", bytes.len());
    app.screen().get(By::text("Archivo listo"));
    std::fs::remove_dir_all(&dir).unwrap();
}
