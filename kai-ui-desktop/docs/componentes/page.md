# Page y PageGrid

Contenedor de una sección junto a la barra lateral y la toolbar, con márgenes
que crecen con el ancho, y una grilla de 4/8/12 columnas.

Código: `src/components/page.rs`. Guía: `docs/dev/21-page-layout.md`.

## Uso

```rust
use kai::components::*;

page().child(
    PageBody::new()
        .max_content(true)
        .child(
            page_grid().children([
                PageGridItem::new(6).child(card_ventas).build(),
                PageGridItem::new(6).child(card_clientes).build(),
                PageGridItem::new(12).child(tabla).build(),
            ]),
        )
        .build(),
)
```

## API

| Elemento | Qué hace |
|---|---|
| `page()` | Contenedor con scroll vertical de la sección (no de la ventana). |
| `page_body()` / `PageBody::new()` | Cuerpo con márgenes: 16 pt, 24 desde `sm`, 32 desde `md`. `.max_content(true)` limita a `max-w-6xl` (1152 pt). |
| `page_grid()` | Grilla de 4 columnas (base), 8 (`sm`) y 12 (`md+`), separación 16 pt. |
| `PageGridItem::new(span)` / `page_grid_item(span)` | Celda; `span` es sobre 12 columnas (1–12). |
| `col_span_classes(span12)`, `span_at(span12, cols)` | Clases y conversión de spans. |

Los spans de media fila o más (`≥ 6`) ocupan todo el ancho bajo `sm` y media
grilla en `sm`.

## Constantes

`GUTTER_PT`, `MARGIN_BASE_PT`, `MARGIN_SM_PT`, `MARGIN_MD_PT`,
`MAX_CONTENT_PT`, `COLS_BASE`, `COLS_SM`, `COLS_MD`, y las clases
`PAGE_CLASS`, `PAGE_BODY_CLASS`, `PAGE_GRID_CLASS`.

## Relacionados

[Plantillas de página](page-templates.md) · `docs/dev/23-spacing-density.md`
