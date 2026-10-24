# Sidebar

Barra lateral de secciones (*source list*). Se puede plegar a un riel de
iconos.

Código: `src/components/sidebar.rs`.

## Uso

```rust
use kai::components::{Sidebar, SidebarGroup, SidebarItem};
use kai::icons::Icon;

let grupos = [
    SidebarGroup::untitled([SidebarItem::new("Inicio").icon(Icon::House)]),
    SidebarGroup::titled("Datos", [
        SidebarItem::new("Clientes").icon(Icon::Users).badge("12"),
        SidebarItem::new("Mapa").icon(Icon::Map),
    ]),
];
Sidebar::new(&grupos, self.seccion, Msg::IrA)
    .collapsed(self.plegada)
    .on_toggle(Msg::AlternarSidebar)
    .label("Secciones")
    .test_id("sidebar")
    .build()
    .class("w-64")
```

## API

| Elemento | Qué hace |
|---|---|
| `SidebarItem::new(label)` | Fila; `.icon(Icon)` y `.badge(texto)` opcionales. |
| `SidebarGroup::titled(title, items)` / `untitled(items)` | Grupos. |
| `Sidebar::new(groups, selected, on_select)` | `selected` cuenta las filas de todos los grupos, en orden. |
| `collapsed(bool)` | Riel de 48 pt con iconos centrados. |
| `on_toggle(msg)` | Botón de plegar al pie (`x.toggle`). |
| `label(text)`, `test_id`, `build()` | — |
| `sidebar(groups, selected, on_select)` | Versión expandida sin botón de plegar. |

Expandida, dale ancho con `.class(..)` sobre el elemento construido.

## Plegada

Los títulos de grupo pasan a separadores, un badge pasa a punto sobre el
icono y cada fila muestra su título en un tooltip. Nombres, selección y
hover se mantienen.

## Teclado

Una parada de Tab que entra por la fila elegida; ↑ y ↓ cambian la selección
(envían `on_select` como un clic).

## Temas

La fila elegida es vidrio teñido (`choice`, cyan); en Glass el texto va en
semibold.

## Partes

Una por fila, por índice: `x.0`, `x.1`…; `x.toggle`.
