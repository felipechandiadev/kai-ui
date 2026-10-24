# SearchField

Campo de búsqueda con lupa y, mientras tiene texto, un botón para borrarlo.

Código: `src/components/search.rs`.

## Uso

```rust
use kai::components::{search_field, SearchVariant};

search_field(&self.query, Msg::Query)
    .placeholder("Filtrar clientes")
    .variant(SearchVariant::Toolbar)
    .test_id("clientes.buscar")
    .build()

// Buscar solo al presionar Return.
search_field(&self.query, Msg::Query).on_submit(Msg::Buscar).build()
```

## API

| Método | Qué hace |
|---|---|
| `search_field(value, on_input)` | Crea el campo. |
| `placeholder(text)` | — |
| `label(text)` | Nombre accesible (por defecto «Buscar»). |
| `on_submit(msg)` | Return; sin él, la búsqueda sigue al tipeo. |
| `variant(SearchVariant)` | `Toolbar` (cápsula con borde), `Field` (junto a campos de un formulario), `Inline` (en una celda o lista densa: sin ancho mínimo, relleno tenue). |
| `control_size(ControlSize)` | En AppKit también lo hereda de un ancestro. |
| `disabled`, `test_id`, `build()` | — |

## Teclado

Escape lo vacía; con el campo ya vacío, Escape sigue por la cadena de
responders (por ejemplo, cancela el diálogo). Borrar también pasa por
`on_input`, así la app conserva el valor.

## Partes

`x.icon`, `x.input`, `x.clear`.

## Relacionados

[Toolbar](toolbar.md) (`toolbar_search`) · [Table](table.md) (búsqueda global)
