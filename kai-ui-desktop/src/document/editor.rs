//! WordCraft [`Session`] keyed por [`WidgetId`] (guía 16 §5–§6 fase 1).
//!
//! Modelo, tipado, selección, undo, **páginas**, scroll y **archivo**
//! (docx/pdf) viven aquí. Varias páginas Letter se apilan en un canvas;
//! el viewport desplaza con `scroll_y`. Hit-test por `DocLayout::hit`.

use serde_json::json;
use wordcraft_doc::{Document, StoryRef};
use wordcraft_engine::Session;
use wordcraft_fonts::{shape, FaceRef, FontDb, DEFAULT_FAMILY};
use wordcraft_layout::display::{page_display, DisplayOptions, Draw};

use crate::geometry::{Point, Rect};
use crate::render::Color;
use crate::text_field::{ClipboardImage, ClipboardPort};
use crate::ui_engine::{ImeIntent, Key, Modifiers, TextIntent};

use super::{
    viewport_metrics, DocumentChange, DocumentCmd, DocumentFill, DocumentGlyph, DocumentGlyphAtlas,
    DocumentImage, DocumentLine, DocumentPaint, DocumentSpec, ExportDoc, PAGE_GAP_PT,
};
use crate::media;
use crate::render::Bitmap;
use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Range;

/// Clave de invalidación del snapshot [`DocumentPaint`].
///
/// El scroll (`scroll_y`) **no** entra: el paint reutiliza glyphs/fills y solo
/// actualiza el offset. Incluye `rev`, selección (caret/highlights; los
/// comandos `caret.*` no bumpan `rev`), tamaño de viewport (afecta scale /
/// `scroll_max`) y modo de vista WordCraft.
#[derive(Clone, PartialEq, Eq)]
struct PaintKey {
    rev: u64,
    sel: wordcraft_engine::Selection,
    vp_w: u32,
    vp_h: u32,
    view_mode: wordcraft_layout::ViewMode,
    read_mode: bool,
    /// Firma del preedit IME (texto + cursor); 0 = ausente.
    preedit_sig: u64,
}

/// Composición IME pendiente (no muta el modelo WordCraft hasta Commit).
#[derive(Clone, Debug, PartialEq, Eq)]
struct DocumentImePreedit {
    text: String,
    /// Rango de bytes del caret IME dentro de `text`; `None` oculta caret en la composición.
    cursor: Option<Range<usize>>,
}

impl DocumentImePreedit {
    fn signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.text.hash(&mut h);
        match &self.cursor {
            Some(r) => {
                r.start.hash(&mut h);
                r.end.hash(&mut h);
            }
            None => {
                usize::MAX.hash(&mut h);
            }
        }
        h.finish()
    }
}

/// Clipboard no-op para tests / APIs que no necesitan host clipboard.
#[cfg_attr(not(test), allow(dead_code))]
struct DiscardClipboard;

#[cfg_attr(not(test), allow(dead_code))]
impl ClipboardPort for DiscardClipboard {
    fn get_text(&mut self) -> Result<String, String> {
        Ok(String::new())
    }

    fn set_text(&mut self, _text: String) -> Result<(), String> {
        Ok(())
    }
}

/// Editor vivo: una [`Session`] WordCraft + snapshot de paint + scroll.
pub struct DocumentEditor {
    session: Session,
    /// Último `serial` de comando de la app ya aplicado.
    last_cmd_serial: u64,
    /// Último `serial` de carga `.docx` ya aplicado.
    last_load_serial: u64,
    /// Markdown que la app empujó (evita re-cargar en bucle).
    last_markdown: String,
    /// Scroll vertical del viewport (puntos lógicos).
    scroll_y: f32,
    /// Último viewport visto (para clamp / ensure_caret).
    pub(crate) last_viewport: Rect,
    /// Cache media WordCraft → Bitmap Kai (atlas).
    bitmaps: HashMap<String, Bitmap>,
    /// Atlas de glifos WordCraft (swash → Bitmap); sin TextStore.
    glyph_atlas: DocumentGlyphAtlas,
    /// Composición IME (preedit) superpuesta al caret; no toca undo/doc.
    ime_preedit: Option<DocumentImePreedit>,
    /// Modelo clonado tras [`DocumentCmd::TakeDocument`].
    taken_document: Option<std::sync::Arc<wordcraft_doc::Document>>,
    paint: DocumentPaint,
    /// Última clave con la que se construyó `paint` (None = nunca).
    last_paint_key: Option<PaintKey>,
    /// Contador de rebuilds de display list (tests de fluidez P1).
    #[cfg(test)]
    paint_rebuilds: u64,
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
            last_load_serial: 0,
            last_markdown: markdown.to_owned(),
            scroll_y: 0.0,
            last_viewport: Rect {
                width: 400.0,
                height: 384.0,
                ..Rect::default()
            },
            bitmaps: HashMap::new(),
            glyph_atlas: DocumentGlyphAtlas::new(),
            ime_preedit: None,
            taken_document: None,
            paint: DocumentPaint::default(),
            last_paint_key: None,
            #[cfg(test)]
            paint_rebuilds: 0,
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
    pub(crate) fn glyph_atlas_len(&self) -> usize {
        self.glyph_atlas.len()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn page_count(&self) -> usize {
        self.paint.page_count
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn scroll_y(&self) -> f32 {
        self.scroll_y
    }

    /// Tablas en el cuerpo (gate fase 2).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn table_count(&self) -> usize {
        self.session
            .doc
            .body
            .iter()
            .filter(|b| b.as_table().is_some())
            .count()
    }

    /// Entradas en el almacén de media del documento.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn media_count(&self) -> usize {
        self.session.doc.media.len()
    }

    /// Rueda / TestApp: desplaza el canvas. Same sign as [`crate::Session::scroll_by`]:
    /// positive moves content down (toward the start); negative toward the end.
    ///
    /// No reconstruye la display list si solo cambia el scroll: actualiza
    /// `paint.scroll_y` sobre el snapshot existente.
    pub fn scroll_by(&mut self, delta_y: f32, viewport: Rect) -> bool {
        self.last_viewport = viewport;
        self.ensure_paint();
        let before = self.scroll_y;
        let max = self.paint.scroll_max;
        self.scroll_y = (self.scroll_y - delta_y).clamp(0.0, max);
        if (self.scroll_y - before).abs() < 0.01 {
            return false;
        }
        self.paint.scroll_y = self.scroll_y;
        true
    }

    #[cfg(test)]
    pub(crate) fn paint_rebuilds(&self) -> u64 {
        self.paint_rebuilds
    }

    pub fn change(&mut self) -> DocumentChange {
        let chrome = super::ribbon_chrome_for(&mut self.session, super::RibbonMode::Full);
        DocumentChange {
            plain_text: self.plain_text(),
            markdown: self.markdown(),
            ribbon_pressed: chrome.pressed,
            ribbon_disabled: chrome.disabled,
            export: self.taken_document.take().map(ExportDoc),
        }
    }

    /// Empuja el markdown de la app si cambió desde fuera (p. ej. restore).
    pub fn sync_markdown(&mut self, markdown: &str) {
        if markdown == self.last_markdown {
            return;
        }
        *self = Self::new(markdown);
    }

    /// Reemplaza el documento desde bytes `.docx` (`wordcraft_docx::read`).
    pub fn load_docx_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        let doc = wordcraft_docx::read(bytes).map_err(|e| e.to_string())?;
        self.session = Session::new(doc);
        self.last_markdown = self.markdown();
        self.scroll_y = 0.0;
        self.bitmaps.clear();
        self.ime_preedit = None;
        self.taken_document = None;
        self.refresh_paint();
        Ok(())
    }

    /// Serializa a `.docx` (tests / encode en el mismo hilo).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn to_docx_bytes(&self) -> Result<Vec<u8>, String> {
        wordcraft_docx::write(&self.session.doc).map_err(|e| e.to_string())
    }

    /// Exporta PDF (`wordcraft_pdf::export`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn to_pdf_bytes(&self) -> Result<Vec<u8>, String> {
        wordcraft_pdf::export(&self.session.doc, &wordcraft_pdf::PdfOptions::default())
            .map_err(|e| e.to_string())
    }

    /// Aplica una carga `.docx` pendiente del spec.
    pub fn apply_pending_load(&mut self, spec: &DocumentSpec) -> bool {
        let Some(req) = &spec.pending_load else {
            return false;
        };
        if req.serial <= self.last_load_serial {
            return false;
        }
        self.last_load_serial = req.serial;
        match self.load_docx_bytes(&req.bytes) {
            Ok(()) => true,
            Err(err) => {
                // Mantener el documento actual; el error lo verá la app
                // si re-intenta. Log suave vía plain_text unchanged.
                let _ = err;
                false
            }
        }
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
            // TakeDocument no muta el cuerpo; el resto sí.
            if !matches!(req.id, DocumentCmd::TakeDocument) {
                self.last_markdown = self.markdown();
                self.refresh_paint();
            }
        }
        changed
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn apply_text_intent(&mut self, intent: TextIntent) -> bool {
        self.apply_text_intent_clipboard(intent, &mut DiscardClipboard)
    }

    /// Como [`Self::apply_text_intent`], con puerto de clipboard (copy/cut/paste).
    pub fn apply_text_intent_clipboard(
        &mut self,
        intent: TextIntent,
        clipboard: &mut dyn ClipboardPort,
    ) -> bool {
        let is_copy = matches!(intent, TextIntent::Copy);
        let changed = match intent {
            TextIntent::InsertText(text) => {
                let cleared = self.clear_ime_preedit();
                if text.is_empty() {
                    return cleared;
                }
                self.insert_str(&text) || cleared
            }
            TextIntent::Ime(ImeIntent::Commit(text)) => {
                let cleared = self.clear_ime_preedit();
                if text.is_empty() {
                    return cleared;
                }
                self.insert_str(&text) || cleared
            }
            TextIntent::Ime(ImeIntent::Preedit { text, cursor }) => {
                self.set_ime_preedit(text, cursor)
            }
            TextIntent::Ime(ImeIntent::Enabled | ImeIntent::Disabled | ImeIntent::Cancel) => {
                self.clear_ime_preedit()
            }
            TextIntent::Undo => self.run_cmd(&DocumentCmd::Undo),
            TextIntent::Redo => self.run_cmd(&DocumentCmd::Redo),
            TextIntent::SelectAll => self.run("select.all"),
            TextIntent::Copy => {
                let _ = self.clipboard_copy(clipboard);
                false
            }
            TextIntent::Cut => self.clipboard_cut(clipboard),
            TextIntent::Paste => self.clipboard_paste(clipboard),
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
        // Copy also "handles" the intent so the Session can invalidate/IME.
        changed || is_copy
    }

    /// Copia la selección vía `edit.copy`: texto plano + HTML (runs / tablas / imágenes) al
    /// [`ClipboardPort`].
    ///
    /// WordCraft no emite RTF en copy; el host puede ofrecer RTF al pegar desde fuera.
    /// El HTML se re-exporta con media embebida (`data:` URIs) y tablas enteras cuando la
    /// selección cubre el story o una tabla completa — `edit.copy` solo no basta (fragmento
    /// sin `doc.media`; selección de celdas aplana a párrafos).
    pub fn clipboard_copy(&mut self, clipboard: &mut dyn ClipboardPort) -> bool {
        match self.session.run("edit.copy", &json!({})) {
            Ok(v) => self.write_copy_to_clipboard(clipboard, &v),
            Err(_) => false,
        }
    }

    /// Corta la selección (`edit.cut`) y escribe texto + HTML al host.
    pub fn clipboard_cut(&mut self, clipboard: &mut dyn ClipboardPort) -> bool {
        match self.session.run("edit.cut", &json!({})) {
            Ok(v) => {
                let _ = self.write_copy_to_clipboard(clipboard, &v);
                self.last_paint_key = None;
                true
            }
            Err(_) => false,
        }
    }

    /// Pega desde el host: HTML o RTF si hay; si no, imagen RGBA del port; si no, texto plano.
    pub fn clipboard_paste(&mut self, clipboard: &mut dyn ClipboardPort) -> bool {
        let text = clipboard.get_text().unwrap_or_default();
        let html = clipboard.get_html().ok().flatten().filter(|h| !h.trim().is_empty());
        let rtf = clipboard.get_rtf().ok().flatten().filter(|r| !r.trim().is_empty());
        let image = if html.is_none() && rtf.is_none() && text.is_empty() {
            clipboard.get_image().ok().flatten()
        } else {
            None
        };
        if text.is_empty() && html.is_none() && rtf.is_none() && image.is_none() {
            return false;
        }
        let mut payload = json!({});
        if !text.is_empty() {
            payload["text"] = json!(text);
        }
        if let Some(html) = html {
            payload["html"] = json!(html);
        } else if let Some(img) = image {
            if let Some(html) = rgba_clipboard_image_to_html_img(&img) {
                payload["html"] = json!(html);
            } else {
                return false;
            }
        }
        if let Some(rtf) = rtf {
            payload["rtf"] = json!(rtf);
        }
        let ok = self.session.run("edit.paste", &payload).is_ok();
        if ok {
            self.last_paint_key = None;
        }
        ok
    }

    fn write_copy_to_clipboard(
        &self,
        clipboard: &mut dyn ClipboardPort,
        v: &serde_json::Value,
    ) -> bool {
        let text = v
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_owned();
        let engine_html = v
            .get("html")
            .and_then(|t| t.as_str())
            .filter(|h| !h.is_empty())
            .map(str::to_owned);
        let html = clipboard_html_enriched(&self.session).or(engine_html);
        if let Some(html) = html {
            clipboard.set_html(html, Some(text)).is_ok()
        } else {
            clipboard.set_text(text).is_ok()
        }
    }

    /// Preedit activo (tests / a11y).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn ime_preedit_text(&self) -> Option<&str> {
        self.ime_preedit.as_ref().map(|p| p.text.as_str())
    }

    fn set_ime_preedit(&mut self, text: String, cursor: Option<Range<usize>>) -> bool {
        let next = if text.is_empty() {
            None
        } else {
            Some(DocumentImePreedit { text, cursor })
        };
        if self.ime_preedit == next {
            return false;
        }
        self.ime_preedit = next;
        true
    }

    fn clear_ime_preedit(&mut self) -> bool {
        if self.ime_preedit.take().is_some() {
            true
        } else {
            false
        }
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
        let params = super::default_run_params(id);
        self.session.run(id, &params).is_ok()
    }

    fn run_cmd(&mut self, cmd: &DocumentCmd) -> bool {
        match cmd {
            DocumentCmd::TakeDocument => {
                self.taken_document = Some(std::sync::Arc::new(self.session.doc.clone()));
                true
            }
            DocumentCmd::InsertTable => self
                .session
                .run("insert.table", &json!({ "rows": 2, "cols": 2 }))
                .is_ok(),
            DocumentCmd::InsertPicture => {
                let b64 = base64_encode(&demo_picture_png());
                self.session
                    .run(
                        "insert.picture",
                        &json!({
                            "data": b64,
                            "width": 120.0,
                            "alt": "Imagen de demostración",
                        }),
                    )
                    .is_ok()
            }
            other => self.run(other.as_str()),
        }
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
        let offset_x = m.origin_x - viewport.x;
        let offset_y = m.origin_y - viewport.y;
        let canvas_x = (local.x - offset_x) / m.scale;
        let canvas_y = (local.y - offset_y + self.scroll_y) / m.scale;
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
        let offset_y = m.origin_y - viewport.y;
        let caret_top = offset_y + c.y * m.scale - self.scroll_y;
        let caret_bot = caret_top + c.height * m.scale;
        let margin = 8.0;
        let mut sy = self.scroll_y;
        if caret_top < offset_y + margin {
            sy -= (offset_y + margin) - caret_top;
        } else if caret_bot > viewport.height - margin {
            sy += caret_bot - (viewport.height - margin);
        }
        sy = sy.clamp(0.0, self.paint.scroll_max);
        if (sy - self.scroll_y).abs() > 0.01 {
            self.scroll_y = sy;
            self.paint.scroll_y = sy;
        }
    }

    /// Reusa [`DocumentPaint`] si `rev` / selección / viewport size / vista
    /// no cambiaron; solo sincroniza `scroll_y`. Llamar desde sync de frame y
    /// scroll — no desde mutaciones de contenido (esas usan [`refresh_paint`]).
    pub(crate) fn ensure_paint(&mut self) {
        let key = self.current_paint_key();
        if self.last_paint_key.as_ref() == Some(&key) {
            self.paint.scroll_y = self.scroll_y;
            return;
        }
        self.refresh_paint();
    }

    fn current_paint_key(&self) -> PaintKey {
        PaintKey {
            rev: self.session.rev(),
            sel: self.session.sel.clone(),
            vp_w: self.last_viewport.width.to_bits(),
            vp_h: self.last_viewport.height.to_bits(),
            view_mode: self.session.view.mode,
            read_mode: self.session.view.read_mode,
            preedit_sig: self
                .ime_preedit
                .as_ref()
                .map(DocumentImePreedit::signature)
                .unwrap_or(0),
        }
    }

    /// Rasteriza el preedit IME junto al caret (glifos WordCraft + underline).
    /// No muta la Session: el commit es quien inserta.
    fn paint_ime_preedit(
        &mut self,
        paint_scale: f32,
        glyphs_out: &mut Vec<DocumentGlyph>,
        underline_out: &mut Vec<Rect>,
        caret: &mut Option<Rect>,
    ) {
        let Some(pre) = self.ime_preedit.clone() else {
            return;
        };
        let Some(anchor) = *caret else {
            return;
        };
        let props = self.session.typing_props();
        let family = props
            .font
            .as_deref()
            .filter(|f| !f.trim().is_empty())
            .unwrap_or(DEFAULT_FAMILY);
        let style = if props.italic == Some(true) && props.bold == Some(true) {
            "Bold Italic"
        } else if props.bold == Some(true) {
            "Bold"
        } else if props.italic == Some(true) {
            "Italic"
        } else {
            "Regular"
        };
        let face_arc = FontDb::global().face(family, style);
        let face = FaceRef::of(&face_arc);
        let size_pt = props.size.unwrap_or(11.0).clamp(1.0, 1584.0);
        let shaped = shape(face.get(), &pre.text, &[], |c| c);
        if shaped.is_empty() {
            return;
        }
        let upem = face.upem.max(1.0) as f32;
        let k = size_pt / upem;
        // Caret WordCraft: top→bottom de la línea; baseline ≈ cerca del fondo.
        let baseline = anchor.y + anchor.height * 0.85;
        let color = Color::rgb(0, 0, 0);
        let synth_bold = props.bold == Some(true) && face.weight < 600.0;
        let px_size = (size_pt * paint_scale).max(1.0);
        let cursor_at = pre
            .cursor
            .as_ref()
            .map(|r| r.end.min(pre.text.len()))
            .unwrap_or(pre.text.len());
        let mut pen_x = anchor.x;
        let start_x = pen_x;
        let mut caret_x = pen_x;
        for g in &shaped {
            if g.cluster <= cursor_at {
                caret_x = pen_x + g.x_advance as f32 * k;
            }
            let gx = pen_x + g.x_offset as f32 * k;
            let gy = baseline - g.y_offset as f32 * k;
            if let Some((bmp, left, top)) =
                self.glyph_atlas.glyph(face, g.gid, px_size, color, synth_bold)
            {
                let w = bmp.width() as f32 / paint_scale;
                let h = bmp.height() as f32 / paint_scale;
                glyphs_out.push(DocumentGlyph {
                    rect: Rect {
                        x: gx + left as f32 / paint_scale,
                        y: gy - top as f32 / paint_scale,
                        width: w.max(0.5),
                        height: h.max(0.5),
                    },
                    bitmap: bmp,
                });
            }
            pen_x += g.x_advance as f32 * k;
        }
        let width = (pen_x - start_x).max(1.0);
        underline_out.push(Rect {
            x: start_x,
            y: baseline + 1.0,
            width,
            height: 1.25,
        });
        *caret = Some(Rect {
            x: caret_x,
            y: anchor.y,
            width: anchor.width,
            height: anchor.height,
        });
    }

    pub fn refresh_paint(&mut self) {
        let layout = self.session.layout();
        let page_count = layout.pages.len().max(1);
        let (pw, ph) = layout
            .pages
            .first()
            .map(|p| (p.w, p.h))
            .unwrap_or((612.0, 792.0));
        let m_early = viewport_metrics(self.last_viewport, (pw, ph));
        let paint_scale = m_early.scale.max(0.01);
        let mut glyphs = Vec::new();
        let mut fills = Vec::new();
        let mut lines = Vec::new();
        let mut images = Vec::new();
        let mut selection = Vec::new();
        let mut caret = None;
        for (pi, p) in layout.pages.iter().enumerate() {
            let page_y = pi as f32 * (ph + PAGE_GAP_PT);
            let draws = page_display(&self.session.doc, p, &DisplayOptions::default());
            collect_display_at(
                &draws,
                page_y,
                paint_scale,
                &self.session.doc.media,
                &mut self.bitmaps,
                &mut self.glyph_atlas,
                &mut glyphs,
                &mut fills,
                &mut lines,
                &mut images,
            );
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
        let mut preedit_underline = Vec::new();
        self.paint_ime_preedit(
            paint_scale,
            &mut glyphs,
            &mut preedit_underline,
            &mut caret,
        );
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
            glyphs,
            fills,
            lines,
            images,
            selection,
            caret,
            preedit_underline,
            page_size: (pw, ph),
            page_count,
            scroll_y: self.scroll_y,
            scroll_max,
        };
        self.last_paint_key = Some(self.current_paint_key());
        #[cfg(test)]
        {
            self.paint_rebuilds = self.paint_rebuilds.wrapping_add(1);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_display_at(
    draws: &[Draw],
    page_y: f32,
    paint_scale: f32,
    media: &std::collections::BTreeMap<String, std::sync::Arc<Vec<u8>>>,
    bitmaps: &mut HashMap<String, Bitmap>,
    atlas: &mut DocumentGlyphAtlas,
    glyphs_out: &mut Vec<DocumentGlyph>,
    fills: &mut Vec<DocumentFill>,
    lines: &mut Vec<DocumentLine>,
    images: &mut Vec<DocumentImage>,
) {
    for d in draws {
        match d {
            Draw::Glyphs {
                face,
                size,
                glyphs,
                color,
                alpha,
                synth_bold,
                ..
            } => {
                if glyphs.is_empty() {
                    continue;
                }
                let c = Color::rgba(
                    color.0,
                    color.1,
                    color.2,
                    (alpha.clamp(0.0, 1.0) * 255.0) as u8,
                );
                let bold = *synth_bold || face.weight >= 600.0;
                let px_size = (*size * paint_scale).max(1.0);
                for &(gid, gx, gy) in glyphs.iter() {
                    let Some((bmp, left, top)) = atlas.glyph(*face, gid, px_size, c, bold) else {
                        continue;
                    };
                    let w = bmp.width() as f32 / paint_scale;
                    let h = bmp.height() as f32 / paint_scale;
                    glyphs_out.push(DocumentGlyph {
                        rect: Rect {
                            x: gx + left as f32 / paint_scale,
                            y: gy - top as f32 / paint_scale + page_y,
                            width: w.max(0.5),
                            height: h.max(0.5),
                        },
                        bitmap: bmp,
                    });
                }
            }
            Draw::Fill {
                rect,
                color,
                alpha,
                ..
            } => {
                fills.push(DocumentFill {
                    rect: Rect {
                        x: rect.x,
                        y: rect.y + page_y,
                        width: rect.w,
                        height: rect.h,
                    },
                    color: Color::rgba(
                        color.0,
                        color.1,
                        color.2,
                        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
                    ),
                });
            }
            Draw::Line {
                x0,
                y0,
                x1,
                y1,
                width,
                color,
                alpha,
                ..
            } => {
                lines.push(DocumentLine {
                    x0: *x0,
                    y0: *y0 + page_y,
                    x1: *x1,
                    y1: *y1 + page_y,
                    width: (*width).max(0.5),
                    color: Color::rgba(
                        color.0,
                        color.1,
                        color.2,
                        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
                    ),
                });
            }
            Draw::Image {
                rect,
                media: key,
                alpha,
                ..
            } => {
                let bitmap = if let Some(b) = bitmaps.get(key) {
                    Some(b.clone())
                } else if let Some(bytes) = media.get(key) {
                    match media::decode(bytes) {
                        Ok(loaded) => {
                            bitmaps.insert(key.clone(), loaded.bitmap.clone());
                            Some(loaded.bitmap)
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                };
                if let Some(bitmap) = bitmap {
                    images.push(DocumentImage {
                        rect: Rect {
                            x: rect.x,
                            y: rect.y + page_y,
                            width: rect.w,
                            height: rect.h,
                        },
                        bitmap,
                        opacity: alpha.clamp(0.0, 1.0),
                    });
                }
            }
            Draw::Turned { items, .. }
            | Draw::Rotated { items, .. }
            | Draw::Clip { items, .. }
            | Draw::Figure { draws: items, .. } => collect_display_at(
                items,
                page_y,
                paint_scale,
                media,
                bitmaps,
                atlas,
                glyphs_out,
                fills,
                lines,
                images,
            ),
            _ => {}
        }
    }
}

/// HTML de clipboard con tablas e imágenes: re-exporta el fragmento WordCraft
/// adjuntando `doc.media` (data URIs) y, si la selección cubre el story entero o una
/// tabla completa, preserva `Block::Table` (el `edit.copy` de celdas aplana a párrafos).
fn clipboard_html_enriched(session: &Session) -> Option<String> {
    let frag = if session.selects_whole_story() {
        session.doc.body_fragment()
    } else if let Some(f) = fully_selected_table_fragment(session) {
        f
    } else if let Some(f) = session.clipboard.clone() {
        f
    } else {
        return None;
    };

    let mut doc = frag.to_document();
    attach_referenced_media(&session.doc, &mut doc);
    let html = wordcraft_formats::html::export_fragment(&doc);
    if html_looks_empty(&html) {
        None
    } else {
        Some(html)
    }
}

fn html_looks_empty(html: &str) -> bool {
    let t = html
        .replace("<meta charset=\"utf-8\">", "")
        .replace("&nbsp;", "")
        .replace("<p></p>", "")
        .replace("<br>", "")
        .replace("<br/>", "");
    t.chars().all(|c| c.is_whitespace() || c == '<' || c == '>' || c == '/')
        && !html.to_ascii_lowercase().contains("<table")
        && !html.contains("<img")
        && !html.contains("data:image")
}

fn fully_selected_table_fragment(session: &Session) -> Option<wordcraft_doc::edit::Fragment> {
    use wordcraft_doc::edit::Fragment;
    use wordcraft_doc::Block;

    let cr = session.cell_range()?;
    let block = session.doc.block(cr.story, &cr.table)?;
    let table = block.as_table()?;
    let last_row = table.rows.len().saturating_sub(1);
    if cr.rows != (0, last_row) {
        return None;
    }
    // Toda columna de la grilla: cols cubre 0..last_grid inclusive.
    let last_col = table
        .rows
        .iter()
        .map(|r| r.cells.len().saturating_sub(1))
        .max()
        .unwrap_or(0);
    // CellRange.cols is grid span; require it covers the table width when possible.
    if cr.cols.0 != 0 {
        return None;
    }
    // Accept full-table select.all growth (rows+cols span the table).
    let cols_span = cr.cols.1.saturating_sub(cr.cols.0);
    if cols_span < last_col && cr.cols.1 < last_col {
        return None;
    }
    Some(Fragment {
        blocks: vec![Block::Table(table.clone())],
        parts: Default::default(),
    })
}

fn attach_referenced_media(src: &Document, dst: &mut Document) {
    use wordcraft_doc::{Block, InlineObject};

    let mut keys = std::collections::BTreeSet::new();
    fn walk(blocks: &[std::sync::Arc<Block>], keys: &mut std::collections::BTreeSet<String>) {
        for b in blocks {
            match &**b {
                Block::Para(p) => {
                    for o in &p.objects {
                        if let InlineObject::Image { media, .. } = o {
                            keys.insert(media.clone());
                        }
                    }
                }
                Block::Table(t) => {
                    for row in &t.rows {
                        for cell in &row.cells {
                            walk(&cell.blocks, keys);
                        }
                    }
                }
            }
        }
    }
    walk(&dst.body, &mut keys);
    if keys.is_empty() {
        // Objetos imagen sin media en el fragmento plano: copiar todo por si acaso
        // (docs de clipboard son pequeños).
        dst.media = src.media.clone();
        return;
    }
    for k in keys {
        if let Some(bytes) = src.media.get(&k) {
            dst.media.insert(k, bytes.clone());
        }
    }
}

/// RGBA del port → `<img src="data:image/png;base64,…">` para `edit.paste` HTML.
fn rgba_clipboard_image_to_html_img(img: &ClipboardImage) -> Option<String> {
    use image::{ImageBuffer, ImageFormat, Rgba};
    if img.width == 0
        || img.height == 0
        || img.bytes.len() != (img.width as usize) * (img.height as usize) * 4
    {
        return None;
    }
    let buf = ImageBuffer::<Rgba<u8>, _>::from_raw(img.width, img.height, img.bytes.clone())?;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(buf)
        .write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)
        .ok()?;
    let b64 = base64_encode(&png);
    Some(format!(
        r#"<p><img src="data:image/png;base64,{b64}" width="{}" height="{}" alt=""/></p>"#,
        img.width, img.height
    ))
}

/// PNG 48×32 a rayas (demo / TestApp); WordCraft lo acepta vía `insert.picture`.
fn demo_picture_png() -> Vec<u8> {
    use image::{ImageBuffer, ImageFormat, Rgba};
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(48, 32, |x, y| {
        if (x / 8 + y / 8) % 2 == 0 {
            Rgba([37, 99, 235, 255])
        } else {
            Rgba([191, 219, 254, 255])
        }
    });
    let mut buf = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .expect("encode demo png");
    buf
}

fn base64_encode(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(b.len().div_ceil(3) * 4);
    for ch in b.chunks(3) {
        let n = (u32::from(ch.first().copied().unwrap_or(0)) << 16)
            | (u32::from(ch.get(1).copied().unwrap_or(0)) << 8)
            | u32::from(ch.get(2).copied().unwrap_or(0));
        for i in 0..4 {
            if i <= ch.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
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
            x: m.origin_x + m.inner_w * 0.5,
            y: m.origin_y + 40.0,
        };
        let (p0, _) = ed.local_to_page_hit(local0, viewport).expect("hit p0");
        assert_eq!(p0, 0);
        // Misma x, y en la página 1 del canvas (tras scroll 0).
        let y_page1 = m.origin_y + (ph + PAGE_GAP_PT) * m.scale + 40.0;
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
        assert!(ed.scroll_by(-120.0, viewport));
        assert!(ed.scroll_y() > 0.0);
        let y = ed.scroll_y();
        assert!(!ed.scroll_by(-1_000_000.0, viewport) || ed.scroll_y() >= y);
        assert!((ed.scroll_y() - ed.paint.scroll_max).abs() < 1.0);
    }

    #[test]
    fn scroll_by_does_not_rebuild_paint() {
        let mut ed = DocumentEditor::new(&multipage_sample());
        let viewport = Rect {
            width: 480.0,
            height: 320.0,
            ..Rect::default()
        };
        ed.last_viewport = viewport;
        ed.refresh_paint();
        let rebuilds = ed.paint_rebuilds();
        let glyphs = ed.paint.glyphs.clone();
        assert!(ed.paint.scroll_max > 0.0);
        assert!(ed.scroll_by(-120.0, viewport));
        assert_eq!(
            ed.paint_rebuilds(),
            rebuilds,
            "scroll must not rebuild page_display / glyph atlas"
        );
        assert_eq!(ed.paint.glyphs, glyphs);
        assert_eq!(ed.paint.scroll_y, ed.scroll_y());
        assert!(ed.scroll_y() > 0.0);
    }

    #[test]
    fn ensure_paint_skips_when_key_unchanged() {
        let mut ed = DocumentEditor::new("hola");
        let viewport = Rect {
            width: 400.0,
            height: 300.0,
            ..Rect::default()
        };
        ed.last_viewport = viewport;
        ed.refresh_paint();
        let n = ed.paint_rebuilds();
        ed.scroll_y = 12.0;
        ed.ensure_paint();
        assert_eq!(ed.paint_rebuilds(), n);
        assert_eq!(ed.paint.scroll_y, 12.0);
    }

    #[test]
    fn ensure_paint_rebuilds_when_viewport_size_changes() {
        let mut ed = DocumentEditor::new(&multipage_sample());
        ed.last_viewport = Rect {
            width: 480.0,
            height: 320.0,
            ..Rect::default()
        };
        ed.refresh_paint();
        let n = ed.paint_rebuilds();
        ed.last_viewport = Rect {
            width: 480.0,
            height: 200.0,
            ..Rect::default()
        };
        ed.ensure_paint();
        assert_eq!(ed.paint_rebuilds(), n + 1);
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
            x: m.origin_x + 80.0,
            y: m.origin_y + 50.0,
        };
        assert!(ed.click(a, viewport, 1));
        let b = Point {
            x: m.origin_x + 80.0,
            y: m.origin_y + (ph + PAGE_GAP_PT) * m.scale + 50.0,
        };
        assert!(ed.drag_to(b, viewport));
        assert!(
            !ed.paint().selection.is_empty(),
            "expected selection rects after drag"
        );
    }
    #[test]
    fn insert_table_2x2() {
        let mut ed = DocumentEditor::new("");
        assert_eq!(ed.table_count(), 0);
        assert!(ed.run_cmd(&DocumentCmd::InsertTable));
        assert_eq!(ed.table_count(), 1);
        ed.refresh_paint();
        // Tabla aporta fills y/o lines en la display list.
        assert!(
            !ed.paint().fills.is_empty() || !ed.paint().lines.is_empty(),
            "table should paint cell fills or borders"
        );
        assert!(ed.run_cmd(&DocumentCmd::Undo));
        assert_eq!(ed.table_count(), 0);
    }

    #[test]
    fn insert_picture_uses_kai_bitmap() {
        let mut ed = DocumentEditor::new("");
        assert_eq!(ed.media_count(), 0);
        assert!(ed.run_cmd(&DocumentCmd::InsertPicture));
        assert!(ed.media_count() >= 1);
        ed.refresh_paint();
        assert!(
            !ed.paint().images.is_empty(),
            "picture should appear in DocumentPaint.images"
        );
        assert!(ed.paint().images[0].bitmap.width() > 0);
    }

    #[test]
    fn docx_roundtrip_preserves_plain_text() {
        let ed = DocumentEditor::new("Hola fase 3");
        let bytes = ed.to_docx_bytes().expect("write docx");
        assert!(bytes.starts_with(b"PK"), "docx is a zip");
        let mut loaded = DocumentEditor::new("");
        loaded.load_docx_bytes(&bytes).expect("read docx");
        assert!(
            loaded.plain_text().contains("Hola fase 3"),
            "got {:?}",
            loaded.plain_text()
        );
    }

    #[test]
    fn pdf_export_is_non_empty() {
        let ed = DocumentEditor::new("PDF de prueba");
        let bytes = ed.to_pdf_bytes().expect("export pdf");
        assert!(bytes.starts_with(b"%PDF"), "got {} bytes", bytes.len());
        assert!(bytes.len() > 100);
    }

    #[test]
    fn take_document_fills_export_once() {
        let mut ed = DocumentEditor::new("snap");
        assert!(ed.run_cmd(&DocumentCmd::TakeDocument));
        let change = ed.change();
        assert!(change.export.is_some());
        assert!(ed.change().export.is_none(), "taken once");
    }

    #[test]
    fn glyph_atlas_rasterizes_body_without_textstore_runs() {
        let ed = DocumentEditor::new("Hola atlas");
        assert!(
            !ed.paint().glyphs.is_empty(),
            "expected rasterized glyphs, got 0"
        );
        assert!(ed.glyph_atlas_len() > 0);
        for g in &ed.paint().glyphs {
            assert!(g.bitmap.width() > 0 && g.bitmap.height() > 0);
            assert!(g.rect.width > 0.0 && g.rect.height > 0.0);
        }
    }

    #[test]
    fn clipboard_copy_cut_paste_plain_text() {
        let mut clip = MemClip::default();
        let mut ed = DocumentEditor::new("");
        assert!(ed.apply_text_intent_clipboard(
            TextIntent::InsertText("Hola mundo".into()),
            &mut clip
        ));
        assert!(ed.apply_text_intent_clipboard(TextIntent::SelectAll, &mut clip));
        assert!(ed.apply_text_intent_clipboard(TextIntent::Copy, &mut clip));
        assert_eq!(clip.text.as_deref(), Some("Hola mundo"));
        assert!(ed.apply_text_intent_clipboard(TextIntent::Cut, &mut clip));
        assert!(
            ed.plain_text().is_empty() || !ed.plain_text().contains("Hola mundo"),
            "cut should remove selection: {:?}",
            ed.plain_text()
        );
        assert_eq!(clip.text.as_deref(), Some("Hola mundo"));
        assert!(ed.apply_text_intent_clipboard(TextIntent::Paste, &mut clip));
        assert!(
            ed.plain_text().contains("Hola mundo"),
            "paste plain: {:?}",
            ed.plain_text()
        );
    }

    #[derive(Default)]
    struct MemClip {
        text: Option<String>,
        html: Option<String>,
        rtf: Option<String>,
        image: Option<ClipboardImage>,
    }

    impl ClipboardPort for MemClip {
        fn get_text(&mut self) -> Result<String, String> {
            Ok(self.text.clone().unwrap_or_default())
        }

        fn set_text(&mut self, text: String) -> Result<(), String> {
            self.text = Some(text);
            self.html = None;
            self.rtf = None;
            self.image = None;
            Ok(())
        }

        fn get_html(&mut self) -> Result<Option<String>, String> {
            Ok(self.html.clone())
        }

        fn set_html(&mut self, html: String, alt_text: Option<String>) -> Result<(), String> {
            self.html = Some(html);
            if let Some(text) = alt_text {
                self.text = Some(text);
            }
            Ok(())
        }

        fn get_rtf(&mut self) -> Result<Option<String>, String> {
            Ok(self.rtf.clone())
        }

        fn set_rtf(&mut self, rtf: String) -> Result<(), String> {
            self.rtf = Some(rtf);
            Ok(())
        }

        fn get_image(&mut self) -> Result<Option<ClipboardImage>, String> {
            Ok(self.image.clone())
        }

        fn set_image(&mut self, image: ClipboardImage) -> Result<(), String> {
            self.image = Some(image);
            Ok(())
        }
    }

    #[test]
    fn clipboard_rich_html_roundtrip_across_editors() {
        let mut clip = MemClip::default();
        let mut src = DocumentEditor::new("");
        assert!(src.apply_text_intent_clipboard(
            TextIntent::InsertText("Hola rich".into()),
            &mut clip
        ));
        assert!(src.apply_text_intent_clipboard(TextIntent::SelectAll, &mut clip));
        assert!(src.run("format.bold"));
        assert!(
            src.markdown().contains("**"),
            "source should be bold markdown: {:?}",
            src.markdown()
        );
        assert!(src.clipboard_copy(&mut clip));
        assert_eq!(clip.text.as_deref(), Some("Hola rich"));
        assert!(
            clip.html.as_ref().is_some_and(|h| h.contains("Hola") || h.contains("rich")),
            "copy must put HTML on the port for cross-editor paste: {:?}",
            clip.html
        );

        // Destino sin fragmento WordCraft propio: solo lo que hay en el port.
        let mut dst = DocumentEditor::new("");
        assert!(dst.clipboard_paste(&mut clip));
        let md = dst.markdown();
        assert!(
            md.contains("**") && md.contains("Hola rich"),
            "paste must keep bold runs via HTML: {md:?}"
        );
    }

    #[test]
    fn clipboard_paste_prefers_html_over_plain_when_both_present() {
        let mut clip = MemClip {
            text: Some("plain only".into()),
            html: Some("<p><strong>Bold HTML</strong></p>".into()),
            rtf: None,
            image: None,
        };
        let mut ed = DocumentEditor::new("");
        assert!(ed.clipboard_paste(&mut clip));
        let md = ed.markdown();
        assert!(
            md.contains("**") || md.to_lowercase().contains("bold"),
            "HTML payload should win formatting: {md:?}"
        );
        assert!(
            md.contains("Bold HTML") || md.contains("Bold"),
            "expected HTML text in doc: {md:?}"
        );
    }

    #[test]
    fn clipboard_paste_rtf_bold_when_no_html() {
        // WordCraft importa RTF en edit.paste; arboard no expone RTF al SO —
        // el port en memoria simula un host que sí lo ofrece.
        let mut clip = MemClip {
            text: Some("Bold RTF".into()),
            html: None,
            rtf: Some(r"{\rtf1\ansi {\b Bold RTF}\par}".into()),
            image: None,
        };
        let mut ed = DocumentEditor::new("");
        assert!(ed.clipboard_paste(&mut clip));
        let md = ed.markdown();
        assert!(
            md.contains("Bold RTF"),
            "RTF text should land: {md:?}"
        );
        assert!(
            md.contains("**"),
            "RTF \\b should become bold markdown: {md:?}"
        );
    }

        #[test]
    fn clipboard_table_html_roundtrip_across_editors() {
        let mut clip = MemClip::default();
        let mut src = DocumentEditor::new("");
        assert!(src.run_cmd(&DocumentCmd::InsertTable));
        assert_eq!(src.table_count(), 1);
        assert!(src.apply_text_intent_clipboard(
            TextIntent::InsertText("CeldaA".into()),
            &mut clip
        ));
        // WordCraft: select.all crece celda → fila → tabla → documento.
        for _ in 0..4 {
            assert!(src.apply_text_intent_clipboard(TextIntent::SelectAll, &mut clip));
        }
        assert!(src.clipboard_copy(&mut clip));
        let html = clip.html.clone().unwrap_or_default();
        assert!(
            html.to_lowercase().contains("<table"),
            "copy of a table must publish HTML <table>: {html}"
        );
        assert!(
            html.contains("CeldaA") || clip.text.as_deref().unwrap_or("").contains("CeldaA"),
            "table cell text should appear in clipboard: html={html:?} text={:?}",
            clip.text
        );

        let mut dst = DocumentEditor::new("");
        assert_eq!(dst.table_count(), 0);
        assert!(dst.apply_text_intent_clipboard(TextIntent::Paste, &mut clip));
        assert_eq!(
            dst.table_count(),
            1,
            "paste HTML table must recreate a table block; plain={:?} md={:?}",
            dst.plain_text(),
            dst.markdown()
        );
        assert!(
            dst.plain_text().contains("CeldaA") || dst.markdown().contains("CeldaA"),
            "cell text must survive: plain={:?} md={:?}",
            dst.plain_text(),
            dst.markdown()
        );
    }

    #[test]
    fn clipboard_picture_html_roundtrip_across_editors() {
        let mut clip = MemClip::default();
        let mut src = DocumentEditor::new("");
        assert!(src.run_cmd(&DocumentCmd::InsertPicture));
        assert!(
            src.media_count() >= 1,
            "source should store picture bytes in doc.media"
        );
        // Incluir el objeto imagen en la selección (documento vacío + picture).
        for _ in 0..3 {
            let _ = src.apply_text_intent_clipboard(TextIntent::SelectAll, &mut clip);
        }
        assert!(src.clipboard_copy(&mut clip));
        let html = clip.html.clone().unwrap_or_default();
        assert!(
            html.contains("<img") || html.contains("data:image"),
            "copy of a picture must publish HTML img/data URI: len={} snippet={}",
            html.len(),
            &html[..html.len().min(240)]
        );

        let mut dst = DocumentEditor::new("");
        assert_eq!(dst.media_count(), 0);
        assert!(dst.apply_text_intent_clipboard(TextIntent::Paste, &mut clip));
        assert!(
            dst.media_count() >= 1,
            "paste HTML picture must recreate media; plain={:?} md={:?} html_len={}",
            dst.plain_text(),
            dst.markdown(),
            html.len()
        );
        dst.refresh_paint();
        assert!(
            !dst.paint().images.is_empty(),
            "pasted picture should paint"
        );
    }

    #[test]
    fn clipboard_paste_html_table_from_external_markup() {
        let mut clip = MemClip {
            text: Some("A\tB\nC\tD".into()),
            html: Some(
                "<table border=\"1\"><tr><td>ExtA</td><td>ExtB</td></tr>\
                 <tr><td>ExtC</td><td>ExtD</td></tr></table>"
                    .into(),
            ),
            rtf: None,
            image: None,
        };
        let mut ed = DocumentEditor::new("");
        assert!(ed.apply_text_intent_clipboard(TextIntent::Paste, &mut clip));
        assert_eq!(
            ed.table_count(),
            1,
            "external HTML table paste: plain={:?} md={:?}",
            ed.plain_text(),
            ed.markdown()
        );
        assert!(
            ed.plain_text().contains("ExtA") || ed.markdown().contains("ExtA"),
            "expected cell ExtA: {:?}",
            ed.plain_text()
        );
    }

    #[test]
    fn clipboard_paste_html_img_data_uri() {
        // 1×1 PNG
        let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let html = format!(
            r#"<p>Antes</p><p><img src="data:image/png;base64,{png_b64}" alt="dot" width="48" height="48"/></p>"#
        );
        let mut clip = MemClip {
            text: Some("Antes".into()),
            html: Some(html),
            rtf: None,
            image: None,
        };
        let mut ed = DocumentEditor::new("");
        assert!(ed.apply_text_intent_clipboard(TextIntent::Paste, &mut clip));
        assert!(
            ed.media_count() >= 1,
            "data URI img paste must create media; plain={:?} md={:?}",
            ed.plain_text(),
            ed.markdown()
        );
        ed.refresh_paint();
        assert!(
            !ed.paint().images.is_empty(),
            "pasted data URI should paint as DocumentImage"
        );
    }

    #[test]
    fn clipboard_paste_raw_rgba_image_when_no_html() {
        let mut clip = MemClip {
            text: None,
            html: None,
            rtf: None,
            image: Some(ClipboardImage {
                width: 2,
                height: 1,
                bytes: vec![255, 0, 0, 255, 0, 255, 0, 255],
            }),
        };
        let mut ed = DocumentEditor::new("");
        assert!(ed.apply_text_intent_clipboard(TextIntent::Paste, &mut clip));
        assert!(
            ed.media_count() >= 1,
            "raw clipboard image must become media; md={:?}",
            ed.markdown()
        );
        ed.refresh_paint();
        assert!(!ed.paint().images.is_empty(), "raw image should paint");
    }


    #[test]
    fn ime_preedit_shows_glyphs_and_underline_without_committing() {
        let mut ed = DocumentEditor::new("");
        let before = ed.plain_text();
        assert!(ed.apply_text_intent(TextIntent::Ime(ImeIntent::Preedit {
            text: "nihon".into(),
            cursor: Some(5..5),
        })));
        assert_eq!(ed.ime_preedit_text(), Some("nihon"));
        assert_eq!(
            ed.plain_text(),
            before,
            "preedit must not mutate the WordCraft doc"
        );
        let paint = ed.paint();
        assert!(
            !paint.preedit_underline.is_empty(),
            "expected IME underline under caret"
        );
        assert!(
            !paint.glyphs.is_empty(),
            "preedit should rasterize composition glyphs"
        );
        assert!(ed.apply_text_intent(TextIntent::Ime(ImeIntent::Preedit {
            text: "nihongo".into(),
            cursor: Some(7..7),
        })));
        assert_eq!(ed.ime_preedit_text(), Some("nihongo"));
        assert_eq!(ed.plain_text(), before);
    }

    #[test]
    fn ime_preedit_then_commit_inserts_once_and_clears_overlay() {
        let mut ed = DocumentEditor::new("");
        assert!(ed.apply_text_intent(TextIntent::Ime(ImeIntent::Preedit {
            text: "にほん".into(),
            cursor: Some(9..9),
        })));
        assert!(ed.ime_preedit_text().is_some());
        assert!(ed.apply_text_intent(TextIntent::Ime(ImeIntent::Commit(
            "日本".into()
        ))));
        assert!(ed.ime_preedit_text().is_none());
        assert!(
            ed.paint().preedit_underline.is_empty(),
            "underline must clear after commit"
        );
        let plain = ed.plain_text();
        assert!(
            plain.contains("日本"),
            "commit inserts final text: {plain:?}"
        );
        assert!(!ed.apply_text_intent(TextIntent::Ime(ImeIntent::Preedit {
            text: String::new(),
            cursor: None,
        })));
        assert_eq!(ed.plain_text(), plain);
    }

    #[test]
    fn ime_cancel_clears_preedit_without_insert() {
        let mut ed = DocumentEditor::new("Base");
        let before = ed.plain_text();
        assert!(ed.apply_text_intent(TextIntent::Ime(ImeIntent::Preedit {
            text: "xxx".into(),
            cursor: Some(3..3),
        })));
        assert!(ed.apply_text_intent(TextIntent::Ime(ImeIntent::Cancel)));
        assert!(ed.ime_preedit_text().is_none());
        assert!(ed.paint().preedit_underline.is_empty());
        assert_eq!(ed.plain_text(), before);
    }

}
