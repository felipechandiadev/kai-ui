//! A small word processor the Document tests share: the editor, its ribbon
//! with the file commands, the comment bar and .docx/.pdf through file
//! panels. It replaces the demo's «Documento» section as the test bench
//! (docs/dev/27).

use schemars::JsonSchema;
use serde::Deserialize;

use crate::commands::{Action, Availability, CommandRegistry};
use crate::components::{search_field, toolbar_item};
use crate::document::{
    DocumentChange, DocumentCmd, RibbonMode, document, document_cmd_for, document_ribbon,
};
use crate::element::{Element, div, text};
use crate::icons::Icon;
use crate::runtime::{App, Effects, PickFile, SaveFile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExportKind {
    Docx,
    Pdf,
}

/// Enough text for several Letter pages.
pub(crate) fn long_markdown() -> String {
    let mut parts = vec!["# Informe de prueba".to_owned()];
    for i in 0..52 {
        parts.push(format!(
            "Párrafo {i}. Texto de relleno para forzar paginación."
        ));
    }
    parts.join("\n\n")
}

#[derive(Default)]
pub(crate) struct Writer {
    pub markdown: String,
    pub file_busy: bool,
    pub ribbon_pressed: Vec<String>,
    pub ribbon_disabled: Vec<String>,
    ribbon_contexts: Vec<String>,
    ribbon_tab: String,
    cmd: Option<(DocumentCmd, u64)>,
    load: Option<(u64, std::sync::Arc<[u8]>)>,
    serial: u64,
    pending_export: Option<(std::path::PathBuf, ExportKind)>,
    comment_open: bool,
    comment_draft: String,
    /// The last file outcome, as the person reads it.
    notice: String,
}

impl Writer {
    pub(crate) fn with_markdown(markdown: impl Into<String>) -> Self {
        Self {
            markdown: markdown.into(),
            ribbon_tab: "home".into(),
            ..Self::default()
        }
    }

    fn run(&mut self, cmd: DocumentCmd) {
        self.serial = self.serial.saturating_add(1);
        self.cmd = Some((cmd, self.serial));
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Msg {
    Changed(DocumentChange),
    Cmd(DocumentCmd),
    RibbonTab(String),
    ToggleComment,
    CommentDraft(String),
    Open,
    OpenChosen(Option<std::path::PathBuf>),
    BytesRead(Result<Vec<u8>, String>),
    Export(ExportKind),
    ExportTo(ExportKind, Option<std::path::PathBuf>),
    FileReady(Result<String, String>),
}

/// Encode and write off the UI thread.
fn write_file(
    doc: wordcraft_doc::Document,
    path: std::path::PathBuf,
    kind: ExportKind,
) -> Result<String, String> {
    let bytes = match kind {
        ExportKind::Docx => wordcraft_docx::write(&doc).map_err(|e| e.to_string())?,
        ExportKind::Pdf => wordcraft_pdf::export(&doc, &wordcraft_pdf::PdfOptions::default())
            .map_err(|e| e.to_string())?,
    };
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

macro_rules! doc_action {
    ($name:ident, $id:literal, $title:literal, $msg:expr) => {
        #[derive(Clone, Deserialize, JsonSchema)]
        pub(crate) struct $name;

        impl Action for $name {
            type App = Writer;
            const ID: &'static str = $id;
            const TITLE: &'static str = $title;
            fn into_msg(self) -> Msg {
                $msg
            }
        }
    };
}

doc_action!(Bold, "format.bold", "Negrita", Msg::Cmd(DocumentCmd::Bold));
doc_action!(Undo, "edit.undo", "Deshacer", Msg::Cmd(DocumentCmd::Undo));
doc_action!(
    InsertTable,
    "insert.table",
    "Insertar tabla",
    Msg::Cmd(DocumentCmd::InsertTable)
);
doc_action!(
    InsertPicture,
    "insert.picture",
    "Insertar imagen",
    Msg::Cmd(DocumentCmd::InsertPicture)
);

/// File commands wait while another file operation runs.
macro_rules! file_action {
    ($name:ident, $id:literal, $title:literal, $msg:expr) => {
        #[derive(Clone, Deserialize, JsonSchema)]
        pub(crate) struct $name;

        impl Action for $name {
            type App = Writer;
            const ID: &'static str = $id;
            const TITLE: &'static str = $title;
            fn available(app: &Writer) -> Availability {
                if app.file_busy {
                    Availability::disabled("Ya hay una operación de archivo en curso")
                } else {
                    Availability::Enabled
                }
            }
            fn into_msg(self) -> Msg {
                $msg
            }
        }
    };
}

file_action!(Open, "document.open", "Abrir documento", Msg::Open);
file_action!(
    Save,
    "document.save",
    "Guardar documento",
    Msg::Export(ExportKind::Docx)
);
file_action!(
    ExportPdf,
    "document.export_pdf",
    "Exportar PDF",
    Msg::Export(ExportKind::Pdf)
);

impl App for Writer {
    type Msg = Msg;

    fn commands(registry: &mut CommandRegistry<Self>) {
        registry.action::<Bold>().menu("Documento");
        registry.action::<Undo>().menu("Documento");
        registry.action::<InsertTable>().menu("Documento");
        registry.action::<InsertPicture>().menu("Documento");
        registry.action::<Open>().menu("Documento");
        registry.action::<Save>().menu("Documento");
        registry.action::<ExportPdf>().menu("Documento");
    }

    fn title(&self) -> String {
        "Documento".into()
    }

    fn update(&mut self, msg: Msg) -> Effects<Msg> {
        match msg {
            Msg::Changed(change) => {
                self.markdown = change.markdown;
                self.ribbon_pressed = change.ribbon_pressed;
                self.ribbon_disabled = change.ribbon_disabled;
                self.ribbon_contexts = change.ribbon_contexts;
                if let Some(export) = change.export
                    && let Some((path, kind)) = self.pending_export.take()
                {
                    let doc = (*export.0).clone();
                    return Effects::perform(move || write_file(doc, path, kind), Msg::FileReady);
                }
            }
            Msg::Cmd(cmd) => self.run(cmd),
            Msg::RibbonTab(id) => self.ribbon_tab = id,
            Msg::ToggleComment => self.comment_open = !self.comment_open,
            Msg::CommentDraft(draft) => self.comment_draft = draft,
            Msg::Open => {
                let request = PickFile::new("Abrir documento").types(&["docx"]);
                return Effects::pick_file(request, Msg::OpenChosen);
            }
            Msg::OpenChosen(None) | Msg::ExportTo(_, None) => {}
            Msg::OpenChosen(Some(path)) => {
                self.file_busy = true;
                return Effects::perform(
                    move || std::fs::read(&path).map_err(|e| e.to_string()),
                    Msg::BytesRead,
                );
            }
            Msg::BytesRead(result) => {
                self.file_busy = false;
                match result {
                    Ok(bytes) => {
                        self.serial = self.serial.saturating_add(1);
                        self.load = Some((self.serial, bytes.into()));
                        self.notice = "Documento abierto".into();
                    }
                    Err(error) => self.notice = error,
                }
            }
            Msg::Export(kind) => {
                let request = match kind {
                    ExportKind::Docx => {
                        SaveFile::new("Guardar documento", "informe.docx").types(&["docx"])
                    }
                    ExportKind::Pdf => SaveFile::new("Exportar PDF", "informe.pdf").types(&["pdf"]),
                };
                return Effects::save_file(request, move |path| Msg::ExportTo(kind, path));
            }
            Msg::ExportTo(kind, Some(path)) => {
                self.file_busy = true;
                self.pending_export = Some((path, kind));
                self.run(DocumentCmd::TakeDocument);
            }
            Msg::FileReady(result) => {
                self.file_busy = false;
                self.notice = match result {
                    Ok(_) => "Archivo listo".into(),
                    Err(error) => error,
                };
            }
        }
        Effects::none()
    }

    fn view(&self) -> Element<Msg> {
        let mut editor = document()
            .label("Informe de prueba")
            .markdown(&self.markdown)
            .on_change(Msg::Changed)
            .class("flex-1 min-h-0 w-full")
            .test_id("document.editor");
        if let Some((cmd, serial)) = &self.cmd {
            editor = editor.pending_cmd(cmd.clone(), *serial);
        }
        if let Some((serial, bytes)) = &self.load {
            editor = editor.pending_load(bytes.clone(), *serial);
        }
        let ribbon = document_ribbon()
            .mode(RibbonMode::Full)
            .active_tab(self.ribbon_tab.clone())
            .pressed_ids(self.ribbon_pressed.clone())
            .disabled_ids(self.ribbon_disabled.clone())
            .contexts(self.ribbon_contexts.clone())
            .on_tab(Msg::RibbonTab)
            // A comment needs its text: the button opens the bar that asks.
            .on_command(|id| match id.as_str() {
                "review.newComment" => Msg::ToggleComment,
                _ => Msg::Cmd(document_cmd_for(&id)),
            })
            .leading([
                toolbar_item(Icon::FolderOpen, "Abrir…")
                    .on_command(Open)
                    .test_id("document.open"),
                toolbar_item(Icon::Save, "Guardar…")
                    .on_command(Save)
                    .test_id("document.save"),
            ])
            .test_id("document.ribbon")
            .build();
        let mut chrome = vec![ribbon];
        if self.comment_open {
            chrome.push(
                div()
                    .class("flex flex-row items-center gap-1")
                    .test_id("document.commentbar")
                    .children([
                        search_field(&self.comment_draft, Msg::CommentDraft)
                            .label("Comentario")
                            .test_id("document.comment.draft")
                            .build(),
                        toolbar_item(Icon::X, "Cerrar comentario")
                            .on_click(Msg::ToggleComment)
                            .test_id("document.comment.close"),
                    ]),
            );
        }
        div().class("flex flex-col w-full h-full").children([
            div().class("flex-none flex flex-col").children(chrome),
            editor,
            text(self.notice.clone()).test_id("document.notice"),
        ])
    }
}
