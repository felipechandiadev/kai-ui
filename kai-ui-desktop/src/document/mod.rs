//! Widget [`document`]: mini procesador de texto (guía
//! [`docs/dev/16-widget-document.md`](../../docs/dev/16-widget-document.md)).
//!
//! Este corte es el **esqueleto**: `Kind::Document`, builder público y paint
//! de una página en blanco (`DrawOp::Quad`). Sin tipear, IME, ni display list
//! de WordCraft en el frame de UI (fase 0).
//!
//! Con `feature = "document"` se enlazan los crates WordCraft (`doc` /
//! `fonts` / `layout` / `engine`) para el trabajo posterior. La
//! `wordcraft_engine::Session` vivirá en la [`crate::session::Session`] de
//! Kai **keyed por [`WidgetId`]**, como la cámara del mapa — no en el modelo
//! de la app ni clonada en `view` (ver §5 de la guía 16).
//!
//! ```ignore
//! document()
//!     .label("Informe")
//!     .on_change(Msg::Doc)
//!     .class("h-96 w-full")
//!     .test_id("documento.editor");
//! ```

use std::rc::Rc;

use crate::element::{Element, Kind, Role, WidgetId};
use crate::geometry::Rect;
use crate::render::{Color, DrawList, Quad};
use crate::theme::Theme;

/// Márgenes internos de la “hoja” respecto al padding box del widget.
const PAGE_INSET: f32 = 16.0;
/// Radio de la hoja placeholder.
const PAGE_RADIUS: f32 = 2.0;
/// Sombra suave bajo la hoja (blur en puntos lógicos).
const PAGE_SHADOW_BLUR: f32 = 8.0;
const PAGE_SHADOW_Y: f32 = 2.0;

/// Qué muestra un [`document`] en este corte: solo metadatos de UI.
/// El cuerpo real y la `wordcraft_engine::Session` llegan en fase 0;
/// entonces la Session de Kai las guarda por [`WidgetId`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentSpec {
    /// Texto inicial opcional (aún no se pinta; reserva API / tests).
    pub placeholder: String,
}

impl DocumentSpec {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Cambio del documento hacia el `update` de la app.
///
/// Esqueleto: variante vacía. Fase 0 rellenará runs / selección / comandos
/// (`format.bold`, …) sin clonar el doc entero en cada frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentChange {}

/// Editor de documento (página en blanco). Dale tamaño con clases
/// (`h-96`, `flex-1 min-h-0`) y un nombre accesible con [`Element::label`].
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

impl<Msg> Element<Msg> {
    /// Documentos: mensaje cuando el contenido cambie (fase 0+). En el
    /// esqueleto el handler se guarda y no se dispara.
    pub fn on_change(mut self, handler: impl Fn(DocumentChange) -> Msg + 'static) -> Self {
        if !matches!(self.kind, Kind::Document(_)) {
            return self;
        }
        self.handlers.on_document_change = Some(Rc::new(handler));
        self
    }

    /// Documentos: texto de reserva (aún no se pinta en el cuerpo).
    pub fn placeholder_text(mut self, text: impl Into<String>) -> Self {
        if let Kind::Document(spec) = &mut self.kind {
            spec.placeholder = text.into();
        }
        self
    }
}

/// Paint esqueleto: fondo del widget + hoja blanca con sombra.
/// Sin cosmic-text en el cuerpo. TODO(fase 0): mapear
/// `wordcraft_layout::display::Draw` → `DrawOp` (ver guía 16 §10).
pub(crate) fn paint(list: &mut DrawList, area: Rect, spec: &DocumentSpec, theme: &Theme) {
    let _ = spec; // placeholder aún no dibujado
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
    }
    list.pop_clip();
}

/// Id estable para tests / session futura (keyed por widget).
#[allow(dead_code)]
pub(crate) fn session_key(id: WidgetId) -> WidgetId {
    id
}

#[cfg(feature = "document")]
pub mod wordcraft {
    //! Reexportes mínimos detrás de la feature (fase 0 enganchará Session).
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
        // Solo comprueba que los símbolos de la feature resuelven.
        let _ = std::any::type_name::<wordcraft::doc::Document>();
    }
}
