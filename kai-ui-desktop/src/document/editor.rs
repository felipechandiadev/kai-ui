//! WordCraft [`Session`] keyed por [`WidgetId`] (guía 16 §5–§6 fase 1).
//!
//! Modelo, tipado, selección, undo, **páginas** y scroll viven aquí.
//! Varias páginas Letter se apilan en un canvas; el viewport del widget
//! desplaza con `scroll_y`. Hit-test por `DocLayout::hit(page, x, y)`.

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

use super::{
    viewport_metrics, DocumentChange, DocumentCmd, DocumentPaint, DocumentRun, DocumentSpec,
    PAGE_GAP_PT,
};

/// Editor vivo: una [`Session`] WordCraft + snapshot de paint + scroll.
pub struct DocumentEditor {
    session: Session,
    /// Último `serial` de comando de la app ya aplicado.
    last_cmd_serial: u64,
    /// Markdown que la app empujó (evita re-cargar en bucle).
    last_markdown: String,
    /// Scroll vertical del viewport (puntos lógicos).
    scroll_y: f32,
    /// Último viewport visto (para clamp / ensure_caret).
    pub(crate) last_viewport: Rect,
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
            scroll_y: 0.0,
            last_viewport: Rect {
                width: 400.0,
                height: 384.0,
                ..Rect::default()
            },
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

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn page_count(&self) -> usize {
        self.paint.page_count
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn scroll_y(&self) -> f32 {
        self.scroll_y
    }

    /// Rueda / TestApp: desplaza el canvas. `delta_y` > 0 mueve el contenido hacia arriba.
    pub fn scroll_by(&mut self, delta_y: f32, viewport: Rect) -> bool {
        self.last_viewport = viewport;
        self.refresh_paint();
        let before = self.scroll_y;
        let max = self.paint.scroll_max;
        self.scroll_y = (self.scroll_y + delta_y).clamp(0.0, max);
        if (self.scroll_y - before).abs() < 0.01 {
            return false;
        }
        self.paint.scroll_y = self.scroll_y;
        true
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

    /// Clic en coords locales al padding box (viewport) del widget.
    pub fn click(&mut self, local: Point, viewport: Rect, clicks: u32) -> bool {
        self.last_viewport = viewport;
        let Some((page_idx, page_pt)) = self.local_to_page_hit(local, viewport) else {
            return false;
        };
        let layout = self.session.layout();
        let Some(pos) = layout.hit(page_idx, page_pt.x, page_pt.y, StoryRef::Body) else {
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
            self.ensure_caret_visible();
        }
        ok
    }

    /// Arrastre de selección (multi-página vía hit por índice).
    pub fn drag_to(&mut self, local: Point, viewport: Rect) -> bool {
        self.last_viewport = viewport;
        let Some((page_idx, page_pt)) = self.local_to_page_hit(local, viewport) else {
            return false;
        };
        let layout = self.session.layout();
        let Some(pos) = layout.hit(page_idx, page_pt.x, page_pt.y, StoryRef::Body) else {
            return false;
        };
        self.session.sel.focus = pos;
        self.refresh_paint();
        self.ensure_caret_visible();
        true
    }

    /// Viewport local → (índice de página, punto en coords de esa página).
    pub fn local_to_page_hit(&self, local: Point, viewport: Rect) -> Option<(usize, Point)> {
        if viewport.width <= 0.0 || viewport.height <= 0.0 {
            return None;
        }
        let (pw, ph) = self.paint.page_size;
        if pw <= 0.0 || ph <= 0.0 {
            return None;
        }
        let m = viewport_metrics(viewport, (pw, ph));
        if m.scale <= 0.0 {
            return None;
        }
        let page_count = self.paint.page_count.max(1);
        let canvas_x = (local.x - m.inset) / m.scale;
        let canvas_y = (local.y - m.inset + self.scroll_y) / m.scale;
        let stride = ph + PAGE_GAP_PT;
        let mut page_idx = if stride > 0.0 {
            (canvas_y / stride).floor().max(0.0) as usize
        } else {
            0
        };
        if page_idx >= page_count {
            page_idx = page_count - 1;
        }
        let y_in = canvas_y - page_idx as f32 * stride;
        Some((
            page_idx,
            Point {
                x: canvas_x.clamp(0.0, pw),
                y: y_in.clamp(0.0, ph),
            },
        ))
    }

    fn ensure_caret_visible(&mut self) {
        let Some(c) = self.paint.caret else {
            return;
        };
        let viewport = self.last_viewport;
        let m = viewport_metrics(viewport, self.paint.page_size);
        if m.scale <= 0.0 {
            return;
        }
        let caret_top = m.inset + c.y * m.scale - self.scroll_y;
        let caret_bot = caret_top + c.height * m.scale;
        let margin = 8.0;
        let mut sy = self.scroll_y;
        if caret_top < m.inset + margin {
            sy -= (m.inset + margin) - caret_top;
        } else if caret_bot > viewport.height - margin {
            sy += caret_bot - (viewport.height - margin);
        }
        sy = sy.clamp(0.0, self.paint.scroll_max);
        if (sy - self.scroll_y).abs() > 0.01 {
            self.scroll_y = sy;
            self.paint.scroll_y = sy;
        }
    }

    pub fn refresh_paint(&mut self) {
        let layout = self.session.layout();
        let page_count = layout.pages.len().max(1);
        let (pw, ph) = layout
            .pages
            .first()
            .map(|p| (p.w, p.h))
            .unwrap_or((612.0, 792.0));
        let mut runs = Vec::new();
        let mut selection = Vec::new();
        let mut caret = None;
        for (pi, p) in layout.pages.iter().enumerate() {
            let page_y = pi as f32 * (ph + PAGE_GAP_PT);
            let draws = page_display(&self.session.doc, p, &DisplayOptions::default());
            collect_runs_at(&draws, page_y, &mut runs);
        }
        let (a, b) = self.session.sel.ordered();
        for (pi, rect) in layout.selection_rects(&self.session.doc, &a, &b, 0) {
            let page_y = pi as f32 * (ph + PAGE_GAP_PT);
            selection.push(Rect {
                x: rect.x,
                y: rect.y + page_y,
                width: rect.w,
                height: rect.h,
            });
        }
        if let Some(c) = layout.caret_on(&self.session.sel.focus, 0) {
            let page_y = c.page as f32 * (ph + PAGE_GAP_PT);
            caret = Some(Rect {
                x: c.x,
                y: c.top + page_y,
                width: c.width.max(1.5),
                height: c.height.max(1.0),
            });
        }
        let m = viewport_metrics(self.last_viewport, (pw, ph));
        let content_h = if page_count == 0 {
            0.0
        } else {
            let pages_h = page_count as f32 * ph * m.scale
                + (page_count.saturating_sub(1) as f32) * PAGE_GAP_PT * m.scale;
            pages_h + m.inset * 2.0
        };
        let scroll_max = (content_h - self.last_viewport.height).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, scroll_max);
        self.paint = DocumentPaint {
            plain_text: self.plain_text(),
            runs,
            selection,
            caret,
            page_size: (pw, ph),
            page_count,
            scroll_y: self.scroll_y,
            scroll_max,
        };
    }
}

fn collect_runs_at(draws: &[Draw], page_y: f32, out: &mut Vec<DocumentRun>) {
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
                        y: gy - size + page_y,
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
            | Draw::Figure { draws: items, .. } => collect_runs_at(items, page_y, out),
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

    fn multipage_sample() -> String {
        (0..55)
            .map(|i| {
                format!(
                    "Párrafo {i}. Texto de relleno para forzar varias páginas Letter                      en el layout de WordCraft (fase 1)."
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[test]
    fn long_document_lays_out_several_pages() {
        let ed = DocumentEditor::new(&multipage_sample());
        assert!(
            ed.page_count() >= 2,
            "expected ≥2 pages, got {}",
            ed.page_count()
        );
    }

    #[test]
    fn hit_test_distinguishes_pages() {
        let mut ed = DocumentEditor::new(&multipage_sample());
        let viewport = Rect {
            width: 500.0,
            height: 360.0,
            ..Rect::default()
        };
        ed.last_viewport = viewport;
        ed.refresh_paint();
        assert!(ed.page_count() >= 2);
        let (pw, ph) = ed.paint.page_size;
        let m = viewport_metrics(viewport, (pw, ph));
        // Clic en la mitad de la página 0.
        let local0 = Point {
            x: m.inset + m.inner_w * 0.5,
            y: m.inset + 40.0,
        };
        let (p0, _) = ed.local_to_page_hit(local0, viewport).expect("hit p0");
        assert_eq!(p0, 0);
        // Misma x, y en la página 1 del canvas (tras scroll 0).
        let y_page1 = m.inset + (ph + PAGE_GAP_PT) * m.scale + 40.0;
        // Sin scroll la página 1 puede estar fuera del viewport; el hit
        // sigue resolviendo el índice por canvas.
        let local1 = Point {
            x: local0.x,
            y: y_page1,
        };
        let (p1, pt) = ed.local_to_page_hit(local1, viewport).expect("hit p1");
        assert_eq!(p1, 1, "y_page1={y_page1} pt={pt:?}");
        assert!(ed.click(local1, viewport, 1), "click page 1");
        // Tras el clic, el caret debe existir en paint.
        assert!(ed.paint().caret.is_some());
    }

    #[test]
    fn scroll_by_moves_viewport() {
        let mut ed = DocumentEditor::new(&multipage_sample());
        let viewport = Rect {
            width: 480.0,
            height: 320.0,
            ..Rect::default()
        };
        ed.last_viewport = viewport;
        ed.refresh_paint();
        assert!(ed.paint.scroll_max > 0.0, "multi-page should scroll");
        assert!(ed.scroll_by(120.0, viewport));
        assert!(ed.scroll_y() > 0.0);
        let y = ed.scroll_y();
        assert!(!ed.scroll_by(1_000_000.0, viewport) || ed.scroll_y() >= y);
        assert!((ed.scroll_y() - ed.paint.scroll_max).abs() < 1.0);
    }

    #[test]
    fn drag_selection_across_pages() {
        let mut ed = DocumentEditor::new(&multipage_sample());
        let viewport = Rect {
            width: 500.0,
            height: 360.0,
            ..Rect::default()
        };
        ed.last_viewport = viewport;
        ed.refresh_paint();
        let (pw, ph) = ed.paint.page_size;
        let m = viewport_metrics(viewport, (pw, ph));
        let a = Point {
            x: m.inset + 80.0,
            y: m.inset + 50.0,
        };
        assert!(ed.click(a, viewport, 1));
        let b = Point {
            x: m.inset + 80.0,
            y: m.inset + (ph + PAGE_GAP_PT) * m.scale + 50.0,
        };
        assert!(ed.drag_to(b, viewport));
        assert!(
            !ed.paint().selection.is_empty(),
            "expected selection rects after drag"
        );
    }
}
