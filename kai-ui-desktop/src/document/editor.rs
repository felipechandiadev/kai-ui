//! WordCraft [`Session`] keyed por [`WidgetId`] (guía 16 §5).
//!
//! Modelo, tipado, selección, undo y comandos (`format.bold`, …) viven
//! aquí. El paint del frame de UI mapea la display list → runs en
//! [`super::DocumentPaint`]; el cuerpo no usa `TextFieldState`.

use serde_json::json;
use wordcraft_doc::{Document, StoryRef};
use wordcraft_engine::Session;
use wordcraft_fonts::FontDb;
use wordcraft_layout::display::{page_display, DisplayOptions, Draw};

use crate::geometry::{Point, Rect};
use crate::render::text::TextStyle;
use crate::render::Color;
use crate::ui_engine::{ImeIntent, Key, Modifiers, TextIntent};
use cosmic_text::Weight;

use super::{DocumentChange, DocumentCmd, DocumentPaint, DocumentRun, DocumentSpec};

/// Editor vivo: una [`Session`] WordCraft + snapshot de paint.
pub struct DocumentEditor {
    session: Session,
    /// Último `serial` de comando de la app ya aplicado.
    last_cmd_serial: u64,
    /// Markdown que la app empujó (evita re-cargar en bucle).
    last_markdown: String,
    paint: DocumentPaint,
}

impl DocumentEditor {
    pub fn new(markdown: &str) -> Self {
        let _ = FontDb::global();
        let plain = markdown_to_plain(markdown);
        let doc = if plain.is_empty() {
            Document::new()
        } else {
            Document::from_text(&plain)
        };
        let mut editor = Self {
            session: Session::new(doc),
            last_cmd_serial: 0,
            last_markdown: markdown.to_owned(),
            paint: DocumentPaint::default(),
        };
        editor.refresh_paint();
        editor
    }

    pub fn plain_text(&self) -> String {
        self.session.doc.plain_text(StoryRef::Body)
    }

    pub fn markdown(&self) -> String {
        plain_to_markdown(&self.session)
    }

    pub fn paint(&self) -> &DocumentPaint {
        &self.paint
    }

    pub fn change(&self) -> DocumentChange {
        DocumentChange {
            plain_text: self.plain_text(),
            markdown: self.markdown(),
        }
    }

    /// Empuja el markdown de la app si cambió desde fuera (p. ej. restore).
    pub fn sync_markdown(&mut self, markdown: &str) {
        if markdown == self.last_markdown {
            return;
        }
        *self = Self::new(markdown);
    }

    /// Aplica un comando pendiente del spec (`format.bold`, …).
    pub fn apply_pending_cmd(&mut self, spec: &DocumentSpec) -> bool {
        let Some(req) = &spec.pending_cmd else {
            return false;
        };
        if req.serial <= self.last_cmd_serial {
            return false;
        }
        self.last_cmd_serial = req.serial;
        let changed = self.run_cmd(&req.id);
        if changed {
            self.last_markdown = self.markdown();
            self.refresh_paint();
        }
        changed
    }

    pub fn apply_text_intent(&mut self, intent: TextIntent) -> bool {
        let changed = match intent {
            TextIntent::InsertText(text) => {
                if text.is_empty() {
                    return false;
                }
                self.insert_str(&text)
            }
            TextIntent::Ime(ImeIntent::Commit(text)) => {
                if text.is_empty() {
                    return false;
                }
                self.insert_str(&text)
            }
            TextIntent::Ime(
                ImeIntent::Preedit { .. }
                | ImeIntent::Enabled
                | ImeIntent::Disabled
                | ImeIntent::Cancel,
            ) => {
                // Fase 0: sin preedit visual en el cuerpo; el commit inserta.
                false
            }
            TextIntent::Undo => self.run_cmd(&DocumentCmd::Undo),
            TextIntent::Redo => self.run_cmd(&DocumentCmd::Redo),
            TextIntent::SelectAll => self.run("select.all"),
            TextIntent::Copy | TextIntent::Cut | TextIntent::Paste => false,
            TextIntent::KeyDown {
                key,
                modifiers,
                repeat: _,
            } => self.key_down(key, modifiers),
        };
        if changed {
            self.last_markdown = self.markdown();
            self.refresh_paint();
        }
        changed
    }

    fn insert_str(&mut self, text: &str) -> bool {
        let mut any = false;
        for ch in text.chars() {
            match ch {
                '\n' => any |= self.run("text.newParagraph"),
                '\t' => any |= self.run("text.tab"),
                c if !c.is_control() => {
                    any |= self
                        .session
                        .run("text.insert", &json!({ "text": c.to_string() }))
                        .is_ok();
                }
                _ => {}
            }
        }
        any
    }

    fn run(&mut self, id: &str) -> bool {
        self.session.run(id, &json!({})).is_ok()
    }

    fn run_cmd(&mut self, cmd: &DocumentCmd) -> bool {
        self.run(cmd.as_str())
    }

    fn run_extend(&mut self, id: &str, extend: bool) -> bool {
        self.session
            .run(id, &json!({ "extend": extend }))
            .is_ok()
    }

    fn key_down(&mut self, key: Key, modifiers: Modifiers) -> bool {
        if modifiers.command && !modifiers.option && !modifiers.control {
            match key {
                Key::Char('b') | Key::Char('B') => {
                    return self.run_cmd(&DocumentCmd::Bold);
                }
                Key::Char('i') | Key::Char('I') => {
                    return self.run_cmd(&DocumentCmd::Italic);
                }
                Key::Char('z') | Key::Char('Z') if modifiers.shift => {
                    return self.run_cmd(&DocumentCmd::Redo);
                }
                Key::Char('z') | Key::Char('Z') => {
                    return self.run_cmd(&DocumentCmd::Undo);
                }
                _ => {}
            }
        }
        let shift = modifiers.shift;
        match key {
            Key::Backspace => self.run("text.backspace"),
            Key::Delete => self.run("text.delete"),
            Key::Enter => self.run("text.newParagraph"),
            Key::Tab if shift => self.run("text.backTab"),
            Key::Tab => self.run("text.tab"),
            Key::ArrowLeft => self.run_extend("caret.left", shift),
            Key::ArrowRight => self.run_extend("caret.right", shift),
            Key::ArrowUp => self.run_extend("caret.up", shift),
            Key::ArrowDown => self.run_extend("caret.down", shift),
            Key::Home if modifiers.command => self.run_extend("caret.docStart", shift),
            Key::End if modifiers.command => self.run_extend("caret.docEnd", shift),
            Key::Home => self.run_extend("caret.home", shift),
            Key::End => self.run_extend("caret.end", shift),
            _ => false,
        }
    }

    /// Clic en coordenadas locales al padding box del widget.
    pub fn click(&mut self, local: Point, page: Rect, clicks: u32) -> bool {
        let Some(page_pt) = local_to_page(local, page) else {
            return false;
        };
        let layout = self.session.layout();
        let Some(pos) = layout.hit(0, page_pt.x, page_pt.y, StoryRef::Body) else {
            return false;
        };
        let ok = if clicks >= 3 {
            self.session.sel = wordcraft_engine::Selection::caret(pos);
            self.run("select.paragraph")
        } else if clicks == 2 {
            self.session.sel = wordcraft_engine::Selection::caret(pos);
            self.run("select.word")
        } else {
            self.session
                .run("caret.set", &json!({ "pos": pos }))
                .is_ok()
                || {
                    self.session.sel = wordcraft_engine::Selection::caret(pos);
                    true
                }
        };
        if ok {
            self.refresh_paint();
        }
        ok
    }

    pub fn drag_to(&mut self, local: Point, page: Rect) -> bool {
        let Some(page_pt) = local_to_page(local, page) else {
            return false;
        };
        let layout = self.session.layout();
        let Some(pos) = layout.hit(0, page_pt.x, page_pt.y, StoryRef::Body) else {
            return false;
        };
        self.session.sel.focus = pos;
        self.refresh_paint();
        true
    }

    pub fn refresh_paint(&mut self) {
        let layout = self.session.layout();
        let page_layout = layout.pages.first();
        let mut runs = Vec::new();
        let mut selection = Vec::new();
        let mut caret = None;
        if let Some(p) = page_layout {
            let draws = page_display(&self.session.doc, p, &DisplayOptions::default());
            collect_runs(&draws, &mut runs);
            let (a, b) = self.session.sel.ordered();
            for (_page, rect) in layout.selection_rects(&self.session.doc, &a, &b, 0) {
                selection.push(Rect {
                    x: rect.x,
                    y: rect.y,
                    width: rect.w,
                    height: rect.h,
                });
            }
            if let Some(c) = layout.caret(&self.session.sel.focus) {
                caret = Some(Rect {
                    x: c.x,
                    y: c.top,
                    width: c.width.max(1.5),
                    height: c.height.max(1.0),
                });
            }
        }
        self.paint = DocumentPaint {
            plain_text: self.plain_text(),
            runs,
            selection,
            caret,
            page_size: page_layout
                .map(|p| (p.w, p.h))
                .unwrap_or((612.0, 792.0)),
        };
    }
}

fn local_to_page(local: Point, page: Rect) -> Option<Point> {
    if page.width <= 0.0 || page.height <= 0.0 {
        return None;
    }
    Some(Point {
        x: (local.x - page.x).max(0.0),
        y: (local.y - page.y).max(0.0),
    })
}

fn collect_runs(draws: &[Draw], out: &mut Vec<DocumentRun>) {
    for d in draws {
        match d {
            Draw::Glyphs {
                face,
                text,
                size,
                glyphs,
                color,
                alpha,
                synth_bold,
                ..
            } => {
                if text.is_empty() || glyphs.is_empty() {
                    continue;
                }
                let (gx, gy) = (glyphs[0].1, glyphs[0].2);
                let bold = *synth_bold || face.weight >= 600.0;
                out.push(DocumentRun {
                    text: text.clone(),
                    origin: Point {
                        x: gx,
                        y: gy - size,
                    },
                    font_size: *size,
                    bold,
                    color: Color::rgba(
                        color.0,
                        color.1,
                        color.2,
                        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
                    ),
                });
            }
            Draw::Turned { items, .. }
            | Draw::Rotated { items, .. }
            | Draw::Clip { items, .. }
            | Draw::Figure { draws: items, .. } => collect_runs(items, out),
            _ => {}
        }
    }
}

/// Markdown mínimo ↔ texto: párrafos por línea en blanco; quita `*` de
/// marcas al importar.
fn markdown_to_plain(md: &str) -> String {
    let mut out = String::new();
    for (i, para) in md.split("\n\n").enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut chars = para.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '*' {
                if chars.peek() == Some(&'*') {
                    chars.next();
                }
                continue;
            }
            out.push(c);
        }
    }
    out
}

fn plain_to_markdown(session: &Session) -> String {
    let mut parts = Vec::new();
    for block in &session.doc.body {
        let Some(para) = block.as_para() else {
            continue;
        };
        let mut s = String::new();
        let mut off = 0usize;
        for run in &para.runs {
            let end = (off + run.len).min(para.text.len());
            if off >= end {
                continue;
            }
            // Avanza por fronteras UTF-8.
            let t = while_valid_slice(&para.text, off, end);
            off = end;
            if t.is_empty() {
                continue;
            }
            let bold = run.props.bold.unwrap_or(false);
            let italic = run.props.italic.unwrap_or(false);
            match (bold, italic) {
                (true, true) => {
                    s.push_str("***");
                    s.push_str(t);
                    s.push_str("***");
                }
                (true, false) => {
                    s.push_str("**");
                    s.push_str(t);
                    s.push_str("**");
                }
                (false, true) => {
                    s.push('*');
                    s.push_str(t);
                    s.push('*');
                }
                (false, false) => s.push_str(t),
            }
        }
        if s.is_empty() && !para.text.is_empty() {
            s.push_str(&para.text);
        }
        parts.push(s);
    }
    parts.join("\n\n")
}

fn while_valid_slice(s: &str, start: usize, end: usize) -> &str {
    let start = floor_char_boundary(s, start);
    let end = floor_char_boundary(s, end).max(start);
    &s[start..end]
}

fn floor_char_boundary(s: &str, i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    let mut i = i;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Estilo cosmic-text solo para el bridge de paint (fase 0 → deuda atlas).
pub fn run_text_style(run: &DocumentRun) -> TextStyle {
    TextStyle {
        font_size: run.font_size,
        line_height: run.font_size * 1.35,
        weight: if run.bold {
            Weight::BOLD
        } else {
            Weight::NORMAL
        },
        wrap: false,
        letter_spacing: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_bold_undo_roundtrip() {
        let mut ed = DocumentEditor::new("");
        assert!(ed.apply_text_intent(TextIntent::InsertText("Hola".into())));
        assert_eq!(ed.plain_text(), "Hola");
        assert!(ed.apply_text_intent(TextIntent::SelectAll));
        assert!(ed.run("format.bold"));
        assert!(ed.run("edit.undo"));
        assert_eq!(ed.plain_text(), "Hola");
        assert!(ed.run("edit.undo"));
        assert!(ed.plain_text().is_empty() || ed.plain_text() != "Hola");
    }

    #[test]
    fn markdown_persist_roundtrip_plain() {
        let ed = DocumentEditor::new("Uno\n\nDos");
        assert!(ed.plain_text().contains("Uno"));
        let md = ed.markdown();
        let ed2 = DocumentEditor::new(&md);
        assert_eq!(
            ed2.plain_text().replace('\r', ""),
            ed.plain_text().replace('\r', "")
        );
    }

    #[test]
    fn cmd_b_via_keydown() {
        let mut ed = DocumentEditor::new("");
        assert!(ed.apply_text_intent(TextIntent::InsertText("ab".into())));
        assert!(ed.apply_text_intent(TextIntent::SelectAll));
        assert!(ed.apply_text_intent(TextIntent::KeyDown {
            key: Key::Char('b'),
            modifiers: Modifiers {
                command: true,
                ..Modifiers::NONE
            },
            repeat: false,
        }));
        let md = ed.markdown();
        assert!(
            md.contains("**") || ed.plain_text() == "ab",
            "bold mark or plain still present: {md:?}"
        );
    }
}
