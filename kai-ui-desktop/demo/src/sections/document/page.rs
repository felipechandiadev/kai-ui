//! /document: a small word processor. The ribbon with the file commands,
//! find and replace (⌘F), the comment bar and a status bar; .docx and PDF
//! through file panels, encoded off the UI thread.

use std::path::PathBuf;
use std::sync::Arc;

use kai::commands::{Availability, CommandRegistry, Key, PageAction, Shortcut};
use kai::components::{
    AlertVariant, ButtonVariant, Toast, button, search_field, toolbar_item,
};
use kai::document::{
    DocumentChange, DocumentCmd, ExportDoc, RibbonMode, document, document_cmd_for,
    document_ribbon,
};
use kai::element::{Element, div, text};
use kai::icons::Icon;
use kai::router::{Context, HasPage, Page};
use kai::runtime::{Effects, PickFile, SaveFile};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::State;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Docx,
    Pdf,
}

/// Enough text for several Letter pages.
fn sample_markdown() -> String {
    let mut parts = vec![
        "# Informe de demostración".to_owned(),
        "Varias páginas Letter, scroll del viewport y edición WordCraft.".to_owned(),
    ];
    for i in 0..52 {
        parts.push(format!(
            "Párrafo {i}. Texto de relleno para forzar paginación en la demo."
        ));
    }
    parts.push("Fin del documento.".to_owned());
    parts.join("\n\n")
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

pub struct Screen {
    markdown: String,
    ribbon_tab: String,
    ribbon_pressed: Vec<String>,
    ribbon_disabled: Vec<String>,
    ribbon_contexts: Vec<String>,
    /// The last command for the editor, with a serial so a repeat counts.
    cmd: Option<(DocumentCmd, u64)>,
    /// A .docx to load, with its serial.
    load: Option<(u64, Arc<[u8]>)>,
    serial: u64,
    /// Where the next taken document goes.
    pending_export: Option<(PathBuf, ExportKind)>,
    file_busy: bool,
    file_name: Option<String>,
    /// What the editor last said (matches, a comment inserted…).
    edit_status: String,
    find_open: bool,
    find: String,
    replace: String,
    comment_open: bool,
    comment: String,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Changed(DocumentChange),
    Cmd(DocumentCmd),
    RibbonTab(String),
    ToggleFind,
    ToggleComment,
    Find(String),
    Replace(String),
    Comment(String),
    Open,
    OpenChosen(Option<PathBuf>),
    BytesRead(Result<Vec<u8>, String>),
    Export(ExportKind),
    ExportTo(ExportKind, Option<PathBuf>),
    FileReady(Result<String, String>),
}

impl Screen {
    fn run(&mut self, cmd: DocumentCmd) {
        self.serial = self.serial.saturating_add(1);
        self.cmd = Some((cmd, self.serial));
    }
}

/// Encode and write off the UI thread.
fn write_file(doc: ExportDoc, path: PathBuf, kind: ExportKind) -> Result<String, String> {
    let (bytes, label) = match kind {
        ExportKind::Docx => (doc.docx_bytes()?, "DOCX"),
        ExportKind::Pdf => (doc.pdf_bytes()?, "PDF"),
    };
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(format!("{label}: {}", file_name(&path)))
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        let screen = Self {
            markdown: sample_markdown(),
            ribbon_tab: "home".into(),
            ribbon_pressed: Vec::new(),
            ribbon_disabled: Vec::new(),
            ribbon_contexts: Vec::new(),
            cmd: None,
            load: None,
            serial: 0,
            pending_export: None,
            file_busy: false,
            file_name: None,
            edit_status: String::new(),
            find_open: false,
            find: String::new(),
            replace: String::new(),
            comment_open: false,
            comment: String::new(),
        };
        (screen, Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Changed(change) => {
                self.markdown = change.markdown;
                self.ribbon_pressed = change.ribbon_pressed;
                self.ribbon_disabled = change.ribbon_disabled;
                self.ribbon_contexts = change.ribbon_contexts;
                if !change.status.is_empty() {
                    self.edit_status = change.status;
                }
                if let Some(doc) = change.export
                    && let Some((path, kind)) = self.pending_export.take()
                {
                    self.file_busy = true;
                    return Effects::perform(move || write_file(doc, path, kind), Msg::FileReady);
                }
            }
            Msg::Cmd(cmd) => self.run(cmd),
            Msg::RibbonTab(id) => self.ribbon_tab = id,
            Msg::ToggleFind => self.find_open = !self.find_open,
            Msg::ToggleComment => self.comment_open = !self.comment_open,
            Msg::Find(text) => self.find = text,
            Msg::Replace(text) => self.replace = text,
            Msg::Comment(text) => self.comment = text,
            Msg::Open => {
                let request = PickFile::new("Abrir documento").types(&["docx"]);
                return Effects::pick_file(request, Msg::OpenChosen);
            }
            Msg::OpenChosen(None) | Msg::ExportTo(_, None) => {}
            Msg::OpenChosen(Some(path)) => {
                self.file_busy = true;
                self.file_name = Some(file_name(&path));
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
                        cx.toast(
                            Toast::new(0, AlertVariant::Success, "Documento abierto")
                                .message("Se cargó el archivo .docx"),
                        );
                    }
                    Err(error) => {
                        cx.toast(Toast::new(0, AlertVariant::Error, "No se abrió").message(error));
                    }
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
                // A PDF does not rename the open .docx.
                if kind == ExportKind::Docx {
                    self.file_name = Some(file_name(&path));
                }
                self.pending_export = Some((path, kind));
                self.run(DocumentCmd::TakeDocument);
            }
            Msg::FileReady(result) => {
                self.file_busy = false;
                cx.toast(match result {
                    Ok(name) => Toast::new(0, AlertVariant::Success, "Archivo listo").message(name),
                    Err(error) => {
                        Toast::new(0, AlertVariant::Error, "No se pudo escribir").message(error)
                    }
                });
            }
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let mut editor = document()
            .label("Informe de demostración")
            .placeholder_text("Escribe aquí…")
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
        let mut chrome = vec![self.ribbon()];
        if self.find_open {
            chrome.push(self.find_bar());
        }
        if self.comment_open {
            chrome.push(self.comment_bar());
        }
        let file = self.file_name.as_deref().unwrap_or("Sin guardar");
        let status = if self.edit_status.is_empty() {
            file.to_owned()
        } else {
            format!("{file} · {}", self.edit_status)
        };
        // The Pages metaphor: ribbon, a full-height canvas and a status
        // bar, without a card or a scrolling page.
        div()
            .class("flex flex-col flex-1 min-h-0 w-full bg-under-page")
            .test_id("document")
            .children([
                div()
                    .class(
                        "flex-none flex flex-col px-3 pt-2 pb-1 bg-window border-b \
                         border-separator gap-1",
                    )
                    .children(chrome),
                editor,
                div()
                    .class(
                        "flex-none flex flex-row items-center px-3 py-1 bg-window border-t \
                         border-separator",
                    )
                    .child(
                        text(status)
                            .class("text-xs text-muted-foreground")
                            .test_id("document.status"),
                    ),
            ])
    }

    fn title(&self, _: &State) -> String {
        match &self.file_name {
            Some(name) => format!("Documento — {name}"),
            None => "Documento".into(),
        }
    }

    fn commands<A: HasPage<Self>>(registry: &mut CommandRegistry<A>) {
        registry.page_action::<Bold>().menu("Documento").expose();
        registry.page_action::<Italic>().menu("Documento").expose();
        registry.page_action::<Bullets>().menu("Documento").expose();
        registry.page_action::<TypeText>().expose();
        registry.page_action::<ClearDocument>().menu("Documento").expose();
        registry.page_action::<InsertTable>().menu("Documento").expose();
        registry.page_action::<InsertPicture>().menu("Documento").expose();
        registry.page_action::<Open>().menu("Documento").expose();
        registry.page_action::<Save>().menu("Documento").expose();
        registry.page_action::<ExportPdf>().menu("Documento").expose();
        registry
            .page_action::<Find>()
            .shortcut(Shortcut::cmd(Key::Char('f')))
            .menu("Documento")
            .expose();
        registry.page_action::<Undo>().menu("Documento").expose();
        registry.page_action::<Redo>().menu("Documento").expose();
    }
}

impl Screen {
    fn ribbon(&self) -> Element<Msg> {
        document_ribbon()
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
                    .on_page_command(Open)
                    .test_id("document.open"),
                toolbar_item(Icon::Save, "Guardar…")
                    .on_page_command(Save)
                    .test_id("document.save"),
                toolbar_item(Icon::FileDown, "Exportar PDF…")
                    .on_page_command(ExportPdf)
                    .test_id("document.export_pdf"),
                toolbar_item(Icon::Search, "Buscar y reemplazar")
                    .on_page_command(Find)
                    .test_id("document.find"),
            ])
            .test_id("document.ribbon")
            .build()
    }

    /// Find and replace in one row, hidden until ⌘F (as in Pages).
    fn find_bar(&self) -> Element<Msg> {
        let (find, replace) = (self.find.clone(), self.replace.clone());
        div()
            .class("flex flex-row items-center gap-1 py-1")
            .test_id("document.findbar")
            .children([
                search_field(&self.find, Msg::Find)
                    .label("Buscar en el documento")
                    .on_submit(Msg::Cmd(DocumentCmd::Find { text: find.clone() }))
                    .test_id("document.find.query")
                    .build(),
                search_field(&self.replace, Msg::Replace)
                    .placeholder("Reemplazar con")
                    .label("Reemplazar con")
                    .test_id("document.find.replace")
                    .build(),
                toolbar_item(Icon::ChevronUp, "Anterior")
                    .on_click(Msg::Cmd(DocumentCmd::FindPrevious))
                    .test_id("document.find.prev"),
                toolbar_item(Icon::ChevronDown, "Siguiente")
                    .on_click(Msg::Cmd(DocumentCmd::FindNext))
                    .test_id("document.find.next"),
                button("Reemplazar", ButtonVariant::Text)
                    .on_click(Msg::Cmd(DocumentCmd::Replace {
                        text: find.clone(),
                        with: replace.clone(),
                    }))
                    .test_id("document.find.replace_one"),
                button("Todo", ButtonVariant::Text)
                    .on_click(Msg::Cmd(DocumentCmd::ReplaceAll {
                        text: find,
                        with: replace,
                    }))
                    .test_id("document.find.replace_all"),
                toolbar_item(Icon::X, "Cerrar búsqueda")
                    .on_click(Msg::ToggleFind)
                    .test_id("document.find.close"),
            ])
    }

    fn comment_bar(&self) -> Element<Msg> {
        let insert = Msg::Cmd(DocumentCmd::NewComment {
            text: self.comment.clone(),
        });
        div()
            .class("flex flex-row items-center gap-1 py-1")
            .test_id("document.commentbar")
            .children([
                search_field(&self.comment, Msg::Comment)
                    .placeholder("Escribe un comentario")
                    .label("Comentario")
                    .on_submit(insert.clone())
                    .test_id("document.comment.draft")
                    .build(),
                button("Comentar", ButtonVariant::Text)
                    .on_click(insert)
                    .test_id("document.comment.new"),
                toolbar_item(Icon::X, "Cerrar comentario")
                    .on_click(Msg::ToggleComment)
                    .test_id("document.comment.close"),
            ])
    }
}

/// An editing command of the document, from the menu, the palette or an
/// agent.
macro_rules! edit_action {
    ($name:ident, $id:literal, $title:literal, $description:literal, $cmd:expr) => {
        #[derive(Clone, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name;

        impl PageAction for $name {
            type Page = Screen;
            const ID: &'static str = $id;
            const TITLE: &'static str = $title;
            fn description() -> &'static str {
                $description
            }
            fn into_msg(self) -> Msg {
                Msg::Cmd($cmd)
            }
        }
    };
}

edit_action!(Bold, "format.bold", "Negrita", "Aplica o quita negrita en la selección", DocumentCmd::Bold);
edit_action!(Italic, "format.italic", "Cursiva", "Aplica o quita cursiva en la selección", DocumentCmd::Italic);
edit_action!(Bullets, "para.bullets", "Lista con viñetas", "Convierte el párrafo en una lista con viñetas", DocumentCmd::Bullets);
edit_action!(InsertTable, "insert.table", "Insertar tabla", "Inserta una tabla en el documento", DocumentCmd::InsertTable);
edit_action!(InsertPicture, "insert.picture", "Insertar imagen", "Inserta una imagen de demostración en el documento", DocumentCmd::InsertPicture);
edit_action!(Undo, "edit.undo", "Deshacer", "Deshace el último cambio del documento", DocumentCmd::Undo);
edit_action!(Redo, "edit.redo", "Rehacer", "Rehace el último cambio deshecho del documento", DocumentCmd::Redo);

/// A file command: it waits while another file operation runs.
macro_rules! file_action {
    ($name:ident, $id:literal, $title:literal, $description:literal, $msg:expr) => {
        #[derive(Clone, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name;

        impl PageAction for $name {
            type Page = Screen;
            const ID: &'static str = $id;
            const TITLE: &'static str = $title;
            fn description() -> &'static str {
                $description
            }
            fn available(page: &Screen) -> Availability {
                if page.file_busy {
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

file_action!(Open, "document.open", "Abrir documento", "Abre un archivo .docx en el editor", Msg::Open);
file_action!(Save, "document.save", "Guardar documento", "Guarda el documento como .docx donde elija la persona", Msg::Export(ExportKind::Docx));
file_action!(ExportPdf, "document.export_pdf", "Exportar PDF", "Exporta el documento a PDF donde elija la persona", Msg::Export(ExportKind::Pdf));

/// `document.find` (⌘F): shows or hides the find and replace bar.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Find;

impl PageAction for Find {
    type Page = Screen;
    const ID: &'static str = "document.find";
    const TITLE: &'static str = "Buscar y reemplazar";
    fn description() -> &'static str {
        "Muestra u oculta la barra Buscar y reemplazar del documento"
    }
    fn into_msg(self) -> Msg {
        Msg::ToggleFind
    }
}

/// `document.type_text`: writes at the caret, as if typed. Meant for agents.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TypeText {
    /// What to write; `\n` starts a new paragraph.
    text: String,
}

impl PageAction for TypeText {
    type Page = Screen;
    const ID: &'static str = "document.type_text";
    const TITLE: &'static str = "Escribir texto";
    fn description() -> &'static str {
        "Escribe un texto en el documento, en la posición del cursor"
    }
    fn into_msg(self) -> Msg {
        Msg::Cmd(DocumentCmd::InsertText { text: self.text })
    }
}

/// `document.clear`: removes all the content of the document.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClearDocument;

impl PageAction for ClearDocument {
    type Page = Screen;
    const ID: &'static str = "document.clear";
    const TITLE: &'static str = "Borrar todo el contenido";
    fn description() -> &'static str {
        "Borra todo el contenido del documento"
    }
    fn into_msg(self) -> Msg {
        Msg::Cmd(DocumentCmd::Clear)
    }
}
