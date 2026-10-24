# Table

Tabla de datos sobre una [VirtualList](virtual-list.md): encabezado fijo,
búsqueda global, filtros por columna, orden, columnas ocultables, columnas
fijas, filas expandibles, scroll horizontal y exportación a Excel. Soporta
decenas de miles de filas: solo construye las visibles.

Código: `src/components/table.rs`, `src/components/table/view.rs`.
Guía: `docs/dev/15-tablas-con-muchos-registros.md`.

## Modelo

La app guarda dos cosas: los **datos** (`Rc<[T]>`) y la **vista**
(`TableView`: búsqueda, filtros, orden, columnas ocultas, filas abiertas y
las filas que resultan). La tabla pide cambios con `ViewChange`; la app los
aplica y refresca.

```rust
use kai::components::*;

fn columnas() -> Vec<Column<Cliente, Msg>> {
    vec![
        Column::text("Nombre", |c: &Cliente| c.nombre.clone())
            .sortable().filterable().flex(2.0).pin(Pin::Start),
        Column::number("Saldo", |c: &Cliente| c.saldo).sortable().fixed(120.0),
        Column::date("Alta", |c: &Cliente| c.alta).sortable().hideable(),
        Column::boolean("Activo", |c: &Cliente| c.activo).filterable().hideable(),
        Column::actions(|c: &Cliente| {
            icon_button(Icon::Pencil, "Editar", IconButtonVariant::Text)
                .on_click(Msg::Editar(c.id))
        }),
    ]
}

// view
Table::new(Rc::clone(&self.clientes), columnas(), &self.vista)
    .on_view(Msg::Vista)
    .selected(self.elegido)
    .on_select(Msg::Elegir)
    .expandable(|c: &Cliente| detalle(c), 120.0)
    .label("Clientes")
    .test_id("clientes.tabla")
    .build()
    .class("flex-1 min-h-0")

// update
Msg::Vista(cambio) => {
    self.vista.apply(cambio);
    self.vista.refresh(&self.clientes, &columnas());
}
```

## `Column`

| Constructor | Valor |
|---|---|
| `Column::text(title, f)` | Texto; entra en la búsqueda global. |
| `Column::number(title, f)` | Número, alineado al final. |
| `Column::date(title, f)` | `Date::new(año, mes, día)`. |
| `Column::boolean(title, f)` | «Sí»/«No» salvo `format`. |
| `Column::actions(render)` | Botones por fila: sin valor, sin orden, filtro, búsqueda ni exportación; fija al final. |

| Modificador | Qué hace |
|---|---|
| `fixed(pt)` / `flex(share)` | Ancho fijo o parte del sobrante. |
| `min_width(pt)` | Mínimo de una flexible (80 por defecto); si no cabe, scroll horizontal. |
| `pin(Pin::Start \| Pin::End)` / `unpin()` | Fija al inicio o al final al desplazar en horizontal. |
| `sortable()` | El encabezado ordena. |
| `filterable()` | Campo en la fila de filtros. |
| `searchable(bool)` | Si la búsqueda global mira esta columna. |
| `hideable()` | Se puede ocultar (menú de columnas). |
| `format(f)` | Texto mostrado (y buscado) para su valor. |
| `render(f)` | Dibuja un componente en la celda; el valor sigue filtrando, ordenando y exportando, y nombra la celda. |

## `Table`

| Método | Qué hace |
|---|---|
| `Table::new(data, columns, &view)` | Crea la tabla. |
| `row_height(pt)` | 32 por defecto (24 es el de AppKit). |
| `on_view(f)` | Recibe los `ViewChange`. |
| `selected(Option<usize>)` / `on_select(f)` | Fila elegida por índice **en los datos** (sobrevive al orden). |
| `expandable(detail, height)` | Las filas abren un panel debajo. |
| `label(text)`, `test_id`, `build()` | Dale alto con `.class(..)`. |

## `TableView`

| Método | Qué hace |
|---|---|
| `TableView::new(&data, &columns)` | Sin filtros, en orden de datos. |
| `apply(ViewChange)` + `refresh(&data, &columns)` | Aplica un cambio y recalcula filas. |
| `rows()`, `len()`, `is_empty()`, `position(row)` | Filas mostradas. |
| `is_filtered()`, `has_filters()`, `can_hide(i)` | Estado. |
| `export(&data, &columns) -> Sheet` | Filas visibles como valores; `kai::xlsx::write` lo guarda en Excel (puede ir a otro hilo). |

`ViewChange`: `Sort(col, dir)`, `Filter(col, texto)`, `Filtering(bool)`,
`Hide(col, bool)`, `Expand(fila, bool)`.

## Menú de columnas (clic derecho)

Clic derecho (o Ctrl+clic) en el encabezado abre `column_menu`: una casilla
por columna ocultable; la última visible no se puede desmarcar. Para un botón
«Columnas»: `column_menu(boton, &columnas, &vista, Msg::Vista)`.

## Teclado

Con `on_select`, las filas son una parada de Tab: clic, ↑/↓, Inicio/Fin,
RePág/AvPág eligen. → y ← abren y cierran una fila expandible. En los
filtros, Escape limpia. Scroll horizontal con trackpad, Shift+rueda o al
mover el foco a una celda fuera de vista.

## Accesibilidad

Encabezados `ColumnHeader`, filas `Row`, celdas `Cell`; la columna de
acciones se llama «Acciones»; el panel abierto es una `Region` nombrada por
los valores de la fila. El menú de columnas responde a «mostrar menú» de
VoiceOver.

## Partes

`scroller`, `header`, `column.<i>`, `filters`, `filter.<i>`, `body`,
`row.<r>`, `row.<r>.toggle`, `detail.<r>` (`i` = columna, `r` = posición en
pantalla).

## Temas

AppKit alterna el fondo de las filas.
