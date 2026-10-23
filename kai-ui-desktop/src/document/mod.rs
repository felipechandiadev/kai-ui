//! Widget [`document`]: mini procesador de texto (guía
//! [`docs/dev/16-widget-document.md`](../../docs/dev/16-widget-document.md)).
//!
//! **Fase 3 — archivo:** abrir/guardar `.docx` (`wordcraft-docx`) y
//! exportar PDF (`wordcraft-pdf`) vía `Effects::pick_file` /
//! `save_file` + `perform` (fuera del paint). El modelo WordCraft se
//! clona al snapshot (`DocumentCmd::TakeDocument`) y se encodea off-thread.
//! Ribbon Kai (catálogo WordCraft) en `ribbon`. Atlas glifos (`glyph_atlas`) +
//! clipboard texto plano; markdown rich tablas/imágenes / keytips = deuda.
//!
//! ```ignore
//! document()
//!     .label("Informe")
//!     .markdown(&self.doc_md)
//!     .on_change(Msg::Doc)
//!     .pending_cmd(cmd, serial)
//!     .class("flex-1 min-h-0")
//!     .test_id("documento.editor");
//! ```

use std::rc::Rc;

use crate::element::{Element, Kind, Role, WidgetId};
use crate::geometry::{Point, Rect};
use crate::render::{Color, DrawList, Quad};
use crate::theme::Theme;

#[cfg(feature = "document")]
mod editor;
#[cfg(feature = "document")]
mod ribbon;
#[cfg(feature = "document")]
mod glyph_atlas;
#[cfg(feature = "document")]
pub(crate) use editor::DocumentEditor;
#[cfg(feature = "document")]
pub(crate) use glyph_atlas::DocumentGlyphAtlas;
#[cfg(feature = "document")]
pub use ribbon::{
    cmd_disabled, default_run_params, document_cmd_for, document_ribbon, icon_for_cmd, localize_cmd,
    parse_location, ribbon_catalog, DocumentRibbon, RibbonGroup, RibbonItem, RibbonMode, RibbonTab,
};

/// Márgenes del stack de páginas respecto al padding box (viewport).
pub(crate) const PAGE_INSET: f32 = 16.0;
/// Separación entre páginas en puntos WordCraft (antes del scale a viewport).
pub(crate) const PAGE_GAP_PT: f32 = 24.0;
/// Radio de cada hoja.
const PAGE_RADIUS: f32 = 2.0;
/// Sombra suave bajo cada hoja (blur en puntos lógicos).
const PAGE_SHADOW_BLUR: f32 = 8.0;
const PAGE_SHADOW_Y: f32 = 2.0;

/// Ids de comando WordCraft que la toolbar / atajos empujan al editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentCmd {
    Bold,
    Italic,
    Bullets,
    Undo,
    Redo,
    /// `insert.table` 2×2 (WordCraft decide la caja).
    InsertTable,
    /// `insert.picture` con PNG embebido de demo (o bytes vía params en Session).
    InsertPicture,
    /// Clona el modelo WordCraft en el próximo [`DocumentChange::export`]
    /// (guardar / exportar PDF off-thread). No es un id de WordCraft.
    TakeDocument,
    /// Id crudo de WordCraft (`format.underline`, …).
    Raw(String),
}

impl DocumentCmd {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Bold => "format.bold",
            Self::Italic => "format.italic",
            Self::Bullets => "para.bullets",
            Self::Undo => "edit.undo",
            Self::Redo => "edit.redo",
            Self::InsertTable => "insert.table",
            Self::InsertPicture => "insert.picture",
            Self::TakeDocument => "kai.document.take",
            Self::Raw(id) => id.as_str(),
        }
    }
}

/// Pedido de comando desde la app (serial como `FitRequest` del mapa).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentCmdRequest {
    pub id: DocumentCmd,
    pub serial: u64,
}

/// Bytes `.docx` a cargar en el editor (serial como [`DocumentCmdRequest`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DocumentLoadRequest {
    pub serial: u64,
    pub bytes: std::sync::Arc<[u8]>,
}

/// Clone del modelo WordCraft listo para encode off-thread (Eq por `Arc`).
#[cfg(feature = "document")]
#[derive(Clone)]
pub(crate) struct ExportDoc(pub(crate) std::sync::Arc<wordcraft_doc::Document>);

#[cfg(feature = "document")]
impl PartialEq for ExportDoc {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(feature = "document")]
impl Eq for ExportDoc {}

#[cfg(feature = "document")]
impl std::fmt::Debug for ExportDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExportDoc(..)")
    }
}

/// Snapshot de paint que la Session rellena cada frame (como tiles/labels).
///
/// Coordenadas de runs / selection / caret están en **canvas** WordCraft:
/// página `i` empieza en `y = i * (page_h + PAGE_GAP_PT)`. El paint aplica
/// scale (`min` ancho/alto para una hoja) y resta [`DocumentPaint::scroll_y`].
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentPaint {
    pub plain_text: String,
    /// Glifos rasterizados (atlas WordCraft → Bitmap), coords canvas.
    pub glyphs: Vec<DocumentGlyph>,
    /// Fondos de celda / párrafo (coords canvas).
    pub fills: Vec<DocumentFill>,
    /// Bordes de tabla y reglas (coords canvas).
    pub lines: Vec<DocumentLine>,
    /// Imágenes inline (bits en atlas Kai).
    pub images: Vec<DocumentImage>,
    pub selection: Vec<Rect>,
    /// Caret en coords canvas (página apilada).
    pub caret: Option<Rect>,
    /// Tamaño de una página WordCraft en puntos (letter ≈ 612×792).
    pub page_size: (f32, f32),
    pub page_count: usize,
    /// Scroll vertical del viewport en puntos lógicos (rueda).
    pub scroll_y: f32,
    pub scroll_max: f32,
}

impl Default for DocumentPaint {
    fn default() -> Self {
        Self {
            plain_text: String::new(),
            glyphs: Vec::new(),
            fills: Vec::new(),
            lines: Vec::new(),
            images: Vec::new(),
            selection: Vec::new(),
            caret: None,
            page_size: (612.0, 792.0),
            page_count: 1,
            scroll_y: 0.0,
            scroll_max: 0.0,
        }
    }
}

/// Un glifo rasterizado del cuerpo (atlas WordCraft → [`crate::render::Bitmap`]).
///
/// Coordenadas en canvas WordCraft (misma base que fills/images). El paint
/// escala al viewport y emite [`crate::render::ImageDraw`] — sin `TextStore`.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentGlyph {
    pub rect: Rect,
    pub bitmap: crate::render::Bitmap,
}

/// Rectángulo relleno de la display list (celda, sombreado).
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentFill {
    pub rect: Rect,
    pub color: Color,
}

/// Segmento de borde (tabla) en coords canvas.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentLine {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub width: f32,
    pub color: Color,
}

/// Imagen de la display list con bitmap Kai (atlas).
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentImage {
    pub rect: Rect,
    pub bitmap: crate::render::Bitmap,
    pub opacity: f32,
}

/// Spec del nodo: markdown de la app + pedido de comando.
/// El snapshot de paint vive en la Session (como la cámara del mapa), no en el Kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentSpec {
    /// Texto de reserva (a11y / vacío).
    pub placeholder: String,
    /// Markdown ligero que la app persiste (`on_change` lo actualiza).
    pub markdown: String,
    /// Comando pendiente (toolbar / Action); la Session lo aplica una vez.
    pub pending_cmd: Option<DocumentCmdRequest>,
    /// Carga `.docx` pendiente (Abrir); la Session la aplica una vez.
    pub(crate) pending_load: Option<DocumentLoadRequest>,
}

impl DocumentSpec {
    pub fn new() -> Self {
        Self::default()
    }

    /// Caja inset del viewport (`area` = padding box). El stack de páginas
    /// vive dentro; el scroll es interno ([`DocumentPaint::scroll_y`]).
    pub fn page_rect(&self, area: Rect) -> Rect {
        let inset = PAGE_INSET.min(area.width / 4.0).min(area.height / 4.0);
        Rect {
            x: area.x + inset,
            y: area.y + inset,
            width: (area.width - inset * 2.0).max(0.0),
            height: (area.height - inset * 2.0).max(0.0),
        }
    }
}

/// Cambio del documento hacia el `update` de la app.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentChange {
    pub plain_text: String,
    pub markdown: String,
    /// Tras [`DocumentCmd::TakeDocument`]: modelo para encode off-thread.
    #[cfg(feature = "document")]
    pub(crate) export: Option<ExportDoc>,
}

/// Editor de documento. Dale tamaño con clases y un nombre accesible con
/// [`Element::label`].
pub fn document<Msg>() -> Element<Msg> {
    Element::new(Kind::Document(Box::new(DocumentSpec::new())))
        .role(Role::TextArea)
        .parts_root()
        .class(
            "relative overflow-hidden rounded-md bg-muted/40 \
             border border-border/60 appkit:border-separator",
        )
}

impl DocumentSpec {
    pub(crate) fn session_key(id: WidgetId) -> WidgetId {
        id
    }
}

impl<Msg> Element<Msg> {
    /// Documentos: mensaje cuando el contenido cambie.
    pub fn on_change(mut self, handler: impl Fn(DocumentChange) -> Msg + 'static) -> Self {
        if !matches!(self.kind, Kind::Document(_)) {
            return self;
        }
        self.handlers.on_document_change = Some(Rc::new(handler));
        self
    }

    /// Documentos: texto de reserva.
    pub fn placeholder_text(mut self, text: impl Into<String>) -> Self {
        if let Kind::Document(spec) = &mut self.kind {
            spec.placeholder = text.into();
        }
        self
    }

    /// Documentos: markdown que la Session carga / sincroniza.
    pub fn markdown(mut self, text: impl Into<String>) -> Self {
        if let Kind::Document(spec) = &mut self.kind {
            spec.markdown = text.into();
        }
        self
    }

    /// Documentos: pide un comando WordCraft una vez (`serial` nuevo).
    pub fn pending_cmd(mut self, id: DocumentCmd, serial: u64) -> Self {
        if let Kind::Document(spec) = &mut self.kind {
            spec.pending_cmd = Some(DocumentCmdRequest { id, serial });
        }
        self
    }

    /// Documentos: carga bytes `.docx` una vez (`serial` nuevo).
    pub fn pending_load(mut self, bytes: std::sync::Arc<[u8]>, serial: u64) -> Self {
        if let Kind::Document(spec) = &mut self.kind {
            spec.pending_load = Some(DocumentLoadRequest { serial, bytes });
        }
        self
    }
}

/// Paint sin id de widget (tests del esqueleto / hoja).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn paint(list: &mut DrawList, area: Rect, spec: &DocumentSpec, theme: &Theme) {
    paint_document(
        list,
        area,
        spec,
        theme,
        WidgetId(0),
        &DocumentPaint::default(),
        false,
        true,
        true,
    );
}


/// Paint con id de widget (usado desde `render/paint.rs`).
pub(crate) fn paint_document(
    list: &mut DrawList,
    area: Rect,
    spec: &DocumentSpec,
    theme: &Theme,
    widget: WidgetId,
    paint: &DocumentPaint,
    focused: bool,
    caret_visible: bool,
    window_active: bool,
) {
    if area.width <= 0.0 || area.height <= 0.0 {
        return;
    }
    list.push_clip(area);
    let _ = spec;
    let metrics = viewport_metrics(area, paint.page_size);
    if metrics.scale <= 0.0 || metrics.inner_w <= 0.0 {
        list.pop_clip();
        return;
    }
    let (pw, ph) = paint.page_size;
    let page_count = paint.page_count.max(1);
    let shadow = theme
        .color("foreground")
        .unwrap_or(Color::rgb(0, 0, 0))
        .with_alpha(28);
    let paper = theme.color("background").unwrap_or(Color::rgb(255, 255, 255));
    let gap = PAGE_GAP_PT * metrics.scale;
    let page_h = ph * metrics.scale;

    for i in 0..page_count {
        let page = page_viewport_rect(&metrics, i, paint.scroll_y, page_h, gap);
        if page.y + page.height < area.y || page.y > area.y + area.height {
            continue;
        }
        list.quad(Quad {
            blur: PAGE_SHADOW_BLUR,
            radius: PAGE_RADIUS,
            ..Quad::fill(
                Rect {
                    x: page.x,
                    y: page.y + PAGE_SHADOW_Y,
                    width: page.width,
                    height: page.height,
                },
                shadow,
            )
        });
        list.quad(Quad {
            radius: PAGE_RADIUS,
            ..Quad::fill(page, paper)
        });
    }

    let origin = canvas_origin(&metrics, paint.scroll_y);
    let sel_color = theme
        .color("primary")
        .unwrap_or(Color::rgb(59, 130, 246))
        .with_alpha(64);
    for r in &paint.selection {
        list.fill_rect(
            Rect {
                x: origin.x + r.x * metrics.scale,
                y: origin.y + r.y * metrics.scale,
                width: (r.width * metrics.scale).max(1.0),
                height: (r.height * metrics.scale).max(1.0),
            },
            sel_color,
        );
    }

    paint_body_canvas(list, origin, paint, widget, metrics.scale);

    if focused
        && window_active
        && caret_visible
        && let Some(c) = paint.caret
    {
        // AppKit `textInsertionPointColor` / semantic.caret (parity with TextField).
        let caret_color = theme.semantic.caret;
        list.fill_rect(
            Rect {
                x: origin.x + c.x * metrics.scale,
                y: origin.y + c.y * metrics.scale,
                width: (c.width * metrics.scale).max(1.0),
                height: (c.height * metrics.scale).max(1.0),
            },
            caret_color,
        );
    }
    let _ = pw;
    list.pop_clip();
}

/// Métricas del viewport: scale para que quepa al menos una página
/// (`min(w/pw, h/ph)`), inset constante, hoja centrada en X.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ViewportMetrics {
    #[cfg_attr(not(feature = "document"), allow(dead_code))]
    pub inset: f32,
    pub scale: f32,
    /// Ancho de una hoja escalada (no el del viewport).
    pub inner_w: f32,
    pub origin_x: f32,
    pub origin_y: f32,
}

pub(crate) fn viewport_metrics(area: Rect, page_size: (f32, f32)) -> ViewportMetrics {
    let inset = PAGE_INSET.min(area.width / 4.0).min(area.height / 4.0);
    let avail_w = (area.width - inset * 2.0).max(0.0);
    let avail_h = (area.height - inset * 2.0).max(0.0);
    let (pw, ph) = page_size;
    let scale = match (pw > 0.0, ph > 0.0, avail_w > 0.0, avail_h > 0.0) {
        (true, true, true, true) => (avail_w / pw).min(avail_h / ph),
        (true, _, true, _) => avail_w / pw,
        _ => 1.0,
    };
    let page_w = pw * scale;
    let origin_x = area.x + (area.width - page_w) * 0.5;
    ViewportMetrics {
        inset,
        scale,
        inner_w: page_w,
        origin_x,
        origin_y: area.y + inset,
    }
}

fn page_viewport_rect(
    m: &ViewportMetrics,
    index: usize,
    scroll_y: f32,
    page_h_scaled: f32,
    gap_scaled: f32,
) -> Rect {
    Rect {
        x: m.origin_x,
        y: m.origin_y + index as f32 * (page_h_scaled + gap_scaled) - scroll_y,
        width: m.inner_w,
        height: page_h_scaled,
    }
}

fn canvas_origin(m: &ViewportMetrics, scroll_y: f32) -> Point {
    Point {
        x: m.origin_x,
        y: m.origin_y - scroll_y,
    }
}

/// Emite fills / imágenes / bordes / glifos (`ImageDraw`) en coords canvas escaladas.
pub(crate) fn paint_body_canvas(
    list: &mut DrawList,
    origin: Point,
    paint: &DocumentPaint,
    widget: WidgetId,
    scale: f32,
) {
    if widget.0 == 0 {
        return;
    }
    for f in &paint.fills {
        list.fill_rect(
            Rect {
                x: origin.x + f.rect.x * scale,
                y: origin.y + f.rect.y * scale,
                width: (f.rect.width * scale).max(0.5),
                height: (f.rect.height * scale).max(0.5),
            },
            f.color,
        );
    }
    for img in &paint.images {
        let rect = Rect {
            x: origin.x + img.rect.x * scale,
            y: origin.y + img.rect.y * scale,
            width: (img.rect.width * scale).max(1.0),
            height: (img.rect.height * scale).max(1.0),
        };
        list.image(crate::render::ImageDraw {
            rect,
            bitmap: img.bitmap.clone(),
            source: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            radius: 0.0,
            opacity: img.opacity.clamp(0.0, 1.0),
        });
    }
    for ln in &paint.lines {
        let dx = ln.x1 - ln.x0;
        let dy = ln.y1 - ln.y0;
        let len = (dx * dx + dy * dy).sqrt().max(0.5);
        // Eje-alineado: quad delgado; diagonal ≈ bounding box fino.
        let (rx, ry, rw, rh) = if dx.abs() >= dy.abs() {
            (
                origin.x + ln.x0.min(ln.x1) * scale,
                origin.y + (ln.y0 + ln.y1) * 0.5 * scale - ln.width * scale * 0.5,
                len * scale,
                (ln.width * scale).max(1.0),
            )
        } else {
            (
                origin.x + (ln.x0 + ln.x1) * 0.5 * scale - ln.width * scale * 0.5,
                origin.y + ln.y0.min(ln.y1) * scale,
                (ln.width * scale).max(1.0),
                len * scale,
            )
        };
        list.fill_rect(
            Rect {
                x: rx,
                y: ry,
                width: rw,
                height: rh,
            },
            ln.color,
        );
    }
    for g in &paint.glyphs {
        let rect = Rect {
            x: origin.x + g.rect.x * scale,
            y: origin.y + g.rect.y * scale,
            width: (g.rect.width * scale).max(1.0),
            height: (g.rect.height * scale).max(1.0),
        };
        list.image(crate::render::ImageDraw {
            rect,
            bitmap: g.bitmap.clone(),
            source: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            radius: 0.0,
            opacity: 1.0,
        });
    }
    let _ = widget;
}

/// Id estable para session (keyed por widget).
#[allow(dead_code)]
pub(crate) fn session_key(id: WidgetId) -> WidgetId {
    DocumentSpec::session_key(id)
}

#[cfg(feature = "document")]
pub mod wordcraft {
    //! Reexportes mínimos detrás de la feature.
    pub use wordcraft_doc as doc;
    pub use wordcraft_engine as engine;
    pub use wordcraft_fonts as fonts;
    pub use wordcraft_layout as layout;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Role;
    use crate::render::DrawOp;
    use crate::theme::{Density, Theme};

    fn area() -> Rect {
        Rect {
            x: 10.0,
            y: 20.0,
            width: 400.0,
            height: 300.0,
        }
    }

    #[test]
    fn document_builder_sets_kind_role_and_label() {
        let el = document::<()>()
            .label("Informe")
            .placeholder_text("Hola")
            .markdown("**Hola**")
            .pending_cmd(DocumentCmd::Bold, 1)
            .on_change(|_| ())
            .class("h-96")
            .test_id("documento.editor");
        assert!(matches!(el.kind, Kind::Document(_)));
        assert_eq!(el.role, Some(Role::TextArea));
        assert_eq!(el.label.as_deref(), Some("Informe"));
        assert_eq!(el.test_id.as_deref(), Some("documento.editor"));
        let Kind::Document(spec) = &el.kind else {
            panic!("expected Document");
        };
        assert_eq!(spec.placeholder, "Hola");
        assert_eq!(spec.markdown, "**Hola**");
        assert_eq!(
            spec.pending_cmd.as_ref().map(|c| c.serial),
            Some(1)
        );
        assert!(el.handlers.on_document_change.is_some());
    }

    #[test]
    fn page_rect_insets_inside_area() {
        let page = DocumentSpec::new().page_rect(area());
        assert!(page.x > area().x);
        assert!(page.width < area().width);
        assert!(page.height < area().height);
    }

    #[test]
    fn paint_emits_shadow_and_page_quads() {
        let theme = Theme::kai(Density::Comfortable);
        let mut list = DrawList::new(theme.colors.background);
        paint(&mut list, area(), &DocumentSpec::new(), &theme);
        let quads: Vec<_> = list
            .ops
            .iter()
            .filter_map(|op| match op {
                DrawOp::Quad(q) => Some(*q),
                _ => None,
            })
            .collect();
        assert!(
            quads.len() >= 2,
            "shadow + page, got {}",
            quads.len()
        );
        assert!(
            quads.iter().any(|q| q.blur > 0.0),
            "expected soft shadow quad"
        );
        assert!(
            quads.iter().any(|q| q.blur == 0.0 && q.fill.a == 255),
            "expected opaque page quad"
        );
    }

    #[cfg(feature = "document")]
    #[test]
    fn wordcraft_crates_link_behind_feature() {
        let _ = std::any::type_name::<wordcraft::doc::Document>();
    }
}

#[cfg(test)]
mod metrics_tests {
    use super::{viewport_metrics, PAGE_INSET};
    use crate::geometry::Rect;

    #[test]
    fn viewport_metrics_fits_one_letter_page() {
        let area = Rect {
            width: 900.0,
            height: 500.0,
            ..Rect::default()
        };
        let m = viewport_metrics(area, (612.0, 792.0));
        let page_h = 792.0 * m.scale;
        assert!(
            page_h + m.inset * 2.0 <= area.height + 0.5,
            "scaled page {page_h} + insets must fit height {}",
            area.height
        );
        assert!(m.inner_w <= area.width);
        assert!(m.scale > 0.0);
        assert!(m.inset <= PAGE_INSET);
    }
}
