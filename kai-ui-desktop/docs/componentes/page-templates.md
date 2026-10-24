# Plantillas de página

Estructuras de página listas: encabezado, lista + detalle, formulario y vista
de datos, sobre [PageGrid](page.md).

Código: `src/components/page_templates.rs`. Guía: `docs/dev/22-page-templates.md`.

## PageHeader

```rust
PageHeader::new("Clientes")
    .breadcrumbs(["Demo", "Clientes"])
    .subtitle("1 024 activos")
    .action(button("Nuevo", ButtonVariant::Primary).on_click(Msg::Nuevo))
```

| Método | Qué hace |
|---|---|
| `new(title)`, `subtitle(text)` | Título y subtítulo. |
| `breadcrumbs(iter)` | Rastro sobre el título (`Demo › Clientes`), solo visual. |
| `action(el)` / `actions(iter)` | Botones a la derecha. |
| `test_id`, `build()` | — |

`breadcrumb_trail(&[String])` dibuja solo el rastro.

## MasterDetailPage (lista + detalle)

```rust
MasterDetailPage::new(lista, detalle)
    .header(PageHeader::new("Clientes"))
    .list_span(4)
    .build()
```

`list_span` (1–11, por defecto `MASTER_LIST_SPAN` = 4) es el ancho de la
lista en 12 columnas; el detalle toma el resto. Bajo `md` se apilan.

## FormPage (formulario)

```rust
FormPage::new()
    .header(PageHeader::new("Nuevo cliente"))
    .field(form_field_cell(campo_nombre))
    .field(form_field_cell(campo_correo))
    .field(form_full_cell(campo_notas))
    .actions([button("Guardar", ButtonVariant::Primary).on_click(Msg::Guardar)])
    .build()
```

Densidad cómoda. `form_field_cell` = media fila (`FORM_FIELD_SPAN` = 6),
`form_full_cell` = fila completa (`FORM_FULL_SPAN` = 12).

## DataPage (vista de datos)

```rust
DataPage::new().header(PageHeader::new("Ventas")).children([grafico, tabla]).build()
```

Densidad compacta, para tablas y gráficos.

## PageDensity

`Comfortable` (formularios) o `Compact` (datos). `theme_density()` lo pasa a
`Density`; `stack_gap_class()` da el espacio vertical entre bloques.
