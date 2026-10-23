//! Widget [`document`]: mini procesador de texto (guía
//! [`docs/dev/16-widget-document.md`](../../docs/dev/16-widget-document.md)).
//!
//! **Fase 0 — nota rica:** con `feature = "document"`, una
//! `wordcraft_engine::Session` por [`WidgetId`] en la Session Kai tipa,
//! selecciona, aplica `format.bold` / undo y pinta runs de la display list
//! como `DrawOp::{Text, Quad}`. Sin egui / vello_cpu. El bridge de glifos
//! usa `TextStore` solo para emitir `DrawOp::Text` (deuda fase 1: atlas
//! WordCraft → glifos nativos).
//!
//! ```ignore
//! document()
//!     .label("Informe")
//!     .markdown(&self.doc_md)
//!     .on_change(Msg::Doc)
//!     .pending_cmd(cmd, serial)
//!     .class("h-96 w-full")
//!     .test_id("documento.editor");
//! ```

use std::rc::Rc;

use crate::element::{Element, Kind, Role, WidgetId};
use crate::geometry::{Point, Rect};
use crate::render::{Color, DrawList, Quad, TextId, TextPart};
use crate::theme::Theme;

#[cfg(feature = "document")]
mod editor;
#[cfg(feature = "document")]
pub(crate) use editor::{run_text_style, DocumentEditor};

/// Márgenes internos de la “hoja” respecto al padding box del widget.
const PAGE_INSET: f32 = 16.0;
/// Radio de la hoja.
const PAGE_RADIUS: f32 = 2.0;
/// Sombra suave bajo la hoja (blur en puntos lógicos).
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

/// Snapshot de paint que la Session rellena cada frame (como tiles/labels).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentPaint {
    pub plain_text: String,
    pub runs: Vec<DocumentRun>,
    pub selection: Vec<Rect>,
    pub caret: Option<Rect>,
    /// Tamaño de página WordCraft en puntos (letter ≈ 612×792).
    pub page_size: (f32, f32),
}

/// Un run de glifos de la display list, listo para `DrawOp::Text`.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentRun {
    pub text: String,
    /// Origen (top-left) en coordenadas de página WordCraft.
    pub origin: Point,
    pub font_size: f32,
    pub bold: bool,
    pub color: Color,
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
}

impl DocumentSpec {
    pub fn new() -> Self {
        Self::default()
    }

    /// Caja de la hoja dentro de `area` (padding box del nodo).
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
}

impl TextId {
    /// Run `index` del cuerpo del documento `widget` (bridge TextStore).
    pub const fn document_run(widget: WidgetId, index: u32) -> Self {
        Self {
            widget,
            part: TextPart::DocumentRun(index),
        }
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
    );
}

/// Emite `DrawOp::Text` por run. `widget` es el id real del nodo.
pub(crate) fn paint_body(
    list: &mut DrawList,
    page: Rect,
    paint: &DocumentPaint,
    widget: WidgetId,
    scale: f32,
) {
    if widget.0 == 0 {
        return;
    }
    for (i, run) in paint.runs.iter().enumerate() {
        let origin = Point {
            x: page.x + run.origin.x * scale,
            y: page.y + run.origin.y * scale,
        };
        list.text(
            TextId::document_run(widget, i as u32),
            origin,
            run.color,
        );
    }
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
) {
    if area.width <= 0.0 || area.height <= 0.0 {
        return;
    }
    list.push_clip(area);
    let page = spec.page_rect(area);
    if page.width > 0.0 && page.height > 0.0 {
        let shadow = theme
            .color("foreground")
            .unwrap_or(Color::rgb(0, 0, 0))
            .with_alpha(28);
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
        let paper = theme.color("background").unwrap_or(Color::rgb(255, 255, 255));
        list.quad(Quad {
            radius: PAGE_RADIUS,
            ..Quad::fill(page, paper)
        });

        let (pw, ph) = paint.page_size;
        let scale = if pw > 0.0 && ph > 0.0 {
            (page.width / pw).min(page.height / ph)
        } else {
            1.0
        };

        let sel_color = theme
            .color("primary")
            .unwrap_or(Color::rgb(59, 130, 246))
            .with_alpha(64);
        for r in &paint.selection {
            list.fill_rect(
                Rect {
                    x: page.x + r.x * scale,
                    y: page.y + r.y * scale,
                    width: (r.width * scale).max(1.0),
                    height: (r.height * scale).max(1.0),
                },
                sel_color,
            );
        }

        paint_body(list, page, paint, widget, scale);

        if focused
            && let Some(c) = paint.caret
        {
            let caret_color = theme
                .color("foreground")
                .unwrap_or(Color::rgb(0, 0, 0));
            list.fill_rect(
                Rect {
                    x: page.x + c.x * scale,
                    y: page.y + c.y * scale,
                    width: (c.width * scale).max(1.0),
                    height: (c.height * scale).max(1.0),
                },
                caret_color,
            );
        }
    }
    list.pop_clip();
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
