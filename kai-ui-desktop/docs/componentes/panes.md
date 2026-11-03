# Paneles y sidebar flotante

Paneles fijos lado a lado sobre el fondo de la ventana, y una barra
lateral flotante de cristal para ellos: el layout Glass de una app
(docs/dev/28 §3.3). Los paneles son **sólidos** (contenido); la sidebar es
**cristal** (navegación), como pide la HIG. No se redimensionan: el
`PaneGroup` redimensionable queda para después.

```rust
panes([
    pane().class("w-[300px] shrink-0").child(lista),
    pane().class("flex-1 min-w-0").child(detalle),
    pane().class("w-[240px] shrink-0").child(inspector),
])
```

## API

| Firma | Qué hace |
|---|---|
| `panes(children)` | Fila de paneles con separación, ocupa lo que queda. El ancho lo da cada panel (`w-[300px] shrink-0` o `flex-1 min-w-0`). |
| `pane()` | Panel sólido redondeado: color de ventana opaco, filete y sombra suave (14 pt de radio en Glass). |
| `pane_footer(children)` | Fila delgada al pie de un panel: un conteo y acciones pequeñas. Mide 44 pt para contener el hover de un `icon_button_sized(.., IconButtonSize::Sm)` (28 pt) con 8 pt de margen. |
| `app_sidebar(rows, path, on_select)` | Sidebar flotante de cristal, estrecha y redondeada: filas `(icono, etiqueta, ruta)`, la activa elegida por ruta con tinte *choice* (cian); una ruta vacía la muestra deshabilitada. Un layout la devuelve en `Layout::sidebar` para reemplazar la sidebar generada. |

## Relacionados

[Toolbar](toolbar.md) · [Sidebar](sidebar.md) · [Page y PageGrid](page.md)
