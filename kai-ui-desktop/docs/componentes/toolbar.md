# Toolbar

Barra a lo ancho de la parte superior de la ventana: 52 pt en Kai, 38 pt en
AppKit (toolbar de barra de título unificada).

Código: `src/components/toolbar.rs`.

## Uso

```rust
use kai::components::*;
use kai::icons::Icon;

toolbar([
    toolbar_item(Icon::PanelLeft, "Barra lateral").on_command(AlternarSidebar),
    toolbar_title("Kai UI Desktop", Some("v0.1.0")),
    toolbar_spacer(),
    toolbar_item(Icon::Plus, "Nuevo").on_click(Msg::Nuevo),
    toolbar_separator(),
    toolbar_search(search_field(&self.query, Msg::Query).build()),
])
```

## API

| Firma | Qué hace |
|---|---|
| `toolbar(items)` | La barra. Sus botones son una sola parada de Tab; ←/→ se mueven entre ellos. |
| `toolbar_item(icon, label)` | Botón de icono sin borde; el borde aparece bajo el puntero. `label` es nombre y tooltip. Encadena `.on_click`, `.on_command` y `.pressed(bool)` para un toggle. |
| `toolbar_title(title, subtitle)` | Título y subtítulo de la ventana. |
| `toolbar_separator()` | Línea vertical entre grupos. |
| `toolbar_spacer()` | Espacio flexible: empuja lo siguiente al final. |
| `toolbar_search(field)` | Hueco de ancho fijo para un campo; es su propia parada de Tab, porque las flechas editan su texto. |
| `toolbar_areas(leading, center, trailing)` | Tres zonas: inicio, centro (centrado en la ventana, los lados crecen igual) y final. Como único ítem de `toolbar`; las zonas suelen ser `slot`s que llenan los layouts internos (docs/dev/28 §3.3). |
| `section_tabs(tabs, path, on_select)` | Navegación por ruta entre las partes de una sección, para el centro de la barra: `tabs` son `(etiqueta, ruta)` y se marca la de prefijo más largo de `path` (`section_tab_index`). En Glass es una píldora: pista gris redondeada, la activa con tinte y contorno finos del color *choice* (cian). |
| `status_bar(items)` | Barra inferior delgada a todo el ancho (24 pt Kai, 22 pt AppKit), esquinas cuadradas, filete arriba y texto secundario pequeño; no toma foco. |
| `status_text(value)` | Texto de una línea para `status_bar`. |
| `status_item(icon, value, label)` | Icono pequeño y texto (solo el icono si `value` está vacío) para `status_bar`; `label` es nombre y tooltip. |

## Estados de `toolbar_item`

`hover` (borde y tinte), `active`, `selected` (`.pressed(true)`: estilo
*choice*), `disabled` (50 %), `busy` (cursor de espera). En AppKit mide
28×32 pt; en Kai, 36×36.

## Relacionados

[IconButton](icon-button.md) · [SearchField](search-field.md) ·
[Sidebar](sidebar.md)
