# 22 — Plantillas de página (Fase C)

Tres plantillas reutilizables sobre el shell de [21 — Page layout](21-page-layout.md): **lista + detalle**, **formulario** y **vista de datos**. Chrome de ventana = Sidebar + Toolbar AppKit (sin cambios).

## Densidad por contexto

| Contexto | `PageDensity` | `Theme::Density` | Ritmo vertical |
|---|---|---|---|
| Formularios, ajustes, alta/edición | `Comfortable` | `Comfortable` | `gap-5` |
| Tablas, métricas, gráficos densos | `Compact` | `Compact` | `gap-3` |

La plantilla declara la intención (`FormPage::density` / `DataPage::density`); la app puede alinear el tema con `PageDensity::theme_density()`.

## API

```rust
use kai::components::{
    DataPage, FormPage, MasterDetailPage, PageHeader,
    form_field_cell, form_full_cell, FORM_FIELD_SPAN, FORM_FULL_SPAN,
};

// Cabecera AppKit: crumbs + título + actions
PageHeader::new("Clientes")
    .subtitle("12 activos")
    .breadcrumbs(["Demo", "Clientes"])
    .action(button("Nuevo", ButtonVariant::Primary))
    .build()

// Lista + detalle (lista span 4 / detalle 8 en md+; apila en estrecho)
MasterDetailPage::new(lista, detalle)
    .header(PageHeader::new("Clientes").breadcrumbs(["App", "Clientes"]))
    .list_span(4)
    .build()

// Formulario comfortable + PageGrid
FormPage::new()
    .header(PageHeader::new("Nuevo cliente"))
    .fields([
        form_field_cell(campo_a), // span 6
        form_field_cell(campo_b),
        form_full_cell(notas),    // span 12
    ])
    .actions([cancelar, guardar])
    .build()

// Datos compact
DataPage::new()
    .header(PageHeader::new("Operaciones"))
    .child(metricas_en_page_grid)
    .child(tabla_densa)
    .build()
```

### Master/detail y breakpoints

En `md+` la lista usa `list_span` (default 4) y el detalle el resto. Por debajo de `md`, ambas celdas usan `col-span-4 sm:col-span-8` (ancho completo de la grilla base/`sm`) para **apilar** lista arriba y detalle abajo — no el escalado proporcional de `page_grid_item`, que dejaría dos columnas en móvil.

## Demo

Grupo sidebar **Plantillas**:

| Sección | `test_id` | Qué muestra |
|---|---|---|
| Lista + detalle | `plantilla.lista` | MasterDetail + PageHeader + crumbs |
| Formulario | `plantilla.formulario` | FormPage comfortable, spans 6/6/12 |
| Vista de datos | `plantilla.datos` | DataPage compact, métricas + filas densas |

Cómo verlo: `cargo run` (tema AppKit) → sidebar **Plantillas**.

## Qué no es

- No clona Carbon 16 cols ni UI Shell web.
- No sustituye Sidebar / Toolbar.
- No define un sistema de spacing tipográfico completo (Fase D).

## Fase D (siguiente)

Spacing extendida (escala tipográfica / do & don’t de márgenes y gaps) documentada sobre estas plantillas; posiblemente tokens adicionales de ritmo vertical más allá de `gap-3` / `gap-5`.
