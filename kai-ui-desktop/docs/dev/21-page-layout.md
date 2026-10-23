# 21 — Page layout: PageGrid 12 + shell de página

**Fase B** del layout de contenido de sección. AppKit canónico: Sidebar + Toolbar existentes; **no** UI Shell web ni bottom bar M3.

## Decisión: 12 columnas (no Carbon 16)

| Breakpoint | Columnas | Ventana |
|---|---|---|
| base (`< sm`) | **4** | stack útil en estrecho |
| `sm` (≥ 640 pt) | **8** | tablet / ventana media |
| `md`+ (≥ 768 pt) | **12** | canónico |

Carbon usa 16 columnas en desktop; Kai **no**. El span tipado es 1–12 sobre la grilla `md+`; en `sm`/base se escala con [`span_at`](../../src/components/page.rs).

## Tokens

Constantes en `kai::components::page_tokens` (pt):

| Token | Valor | Clases |
|---|---|---|
| `GUTTER_PT` | 16 | `gap-4` |
| `MARGIN_BASE_PT` | 16 | `px-4` |
| `MARGIN_SM_PT` | 24 | `sm:px-6` |
| `MARGIN_MD_PT` | 32 | `md:px-8` |
| `MAX_CONTENT_PT` | 1152 | `max-w-6xl` (opcional) |

`Theme::space.page_gap` (Kai 16 / AppKit 20) sigue siendo el margen HIG de ventana; el body de página usa los márgenes de la tabla.

## API

```rust
use kai::components::{page, page_body, page_grid, page_grid_item, PageBody, PageGridItem};

// Shell de sección (junto al Sidebar; no es la ventana):
page().child(
    page_body()
        .test_id("contenido")
        .children([/* cards */]),
)

// Con tope de ancho opcional:
PageBody::new()
    .max_content(true)
    .children([/* … */])
    .build()

// Grilla 4 / 8 / 12 + gutter 16 pt:
page_grid().children([
    page_grid_item(6).child(field_a), // mitad en md+
    page_grid_item(6).child(field_b),
    PageGridItem::new(12).child(full_width).build(),
])
```

Jerarquía: **`Page` → `PageBody` → `PageGrid` / `PageGridItem`**.

- `page` — shell scroll `flex-1 overflow-y-auto` de la sección (sin rol propio).
- `page_body` — columna + márgenes responsivos (dentro del scroll).
- `page_grid` — CSS grid (`grid-cols-4 sm:grid-cols-8 md:grid-cols-12 gap-4`).
- `page_grid_item(span)` — `col-span-*` responsivo (requiere utilidad `col-span-N` del motor).

## Motor: `col-span-N`

`Style::grid_column_span` + clase `col-span-N` (y variantes `sm:` / `md:`) se mapean a Taffy `grid_column: span(N)`.

## Demo

- Shell de todas las secciones (salvo Documento): `page` + `page_body` (`test_id` `contenido`).
- **Inicio** → card «Formulario controlado»: campos en `page_grid` con span 6.
- **Campos** → card «Variantes de TextField»: misma grilla.

Cómo verlo: `cargo run` (tema AppKit por defecto) → Inicio / Campos; estrechar la ventana para 4 → 8 → 12.

## Qué no es

- No es la grilla Carbon de **16** columnas.
- No es UI Shell de Carbon React ni bottom navigation M3.
- No sustituye Sidebar / Toolbar AppKit (chrome de ventana).
- Las plantillas de página están en Fase C ([22](22-page-templates.md)).

## Fase C

Hecha: plantillas lista+detalle / formulario / datos + `PageHeader` — ver [22-page-templates.md](22-page-templates.md).
