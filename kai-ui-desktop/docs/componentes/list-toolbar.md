# ListToolbar (barra de herramientas de lista)

Barra al pie de un panel de lista: información a la izquierda, acciones a
la derecha y una búsqueda que se despliega desde el extremo derecho.

```rust
list_toolbar(format!("{} de {} clientes", visibles, total))
    .action(ListAction::button(Icon::FileDown, "Descargar", Msg::Descargar))
    .action(ListAction::menu(Icon::ListOrdered, "Ordenar", &ORDENES, self.orden, Msg::Orden)
        .active(self.orden.is_some()))
    .action(ListAction::menu(Icon::Funnel, "Filtrar", &ETAPAS, Some(self.filtro), Msg::Filtro)
        .active(self.filtro != 0))
    .action(ListAction::button(Icon::Plus, "Añadir cliente", Msg::Nuevo))
    .search(self.texto.clone(), self.reveal, Msg::Texto, Msg::AbrirBusqueda, Msg::CerrarBusqueda)
    .test_id("clientes.barra")
    .build()
```

## Reglas del patrón

- **Info a la izquierda**, solo lectura: el total («8 clientes») o, con
  filtro o búsqueda, «N de M clientes».
- **Acciones a la derecha**, iconos compactos de 28 pt con tooltip, en
  barra de 40 pt (44 en Kai): el hover queda dentro.
- **Búsqueda al extremo derecho**, expandible: al pulsarla el campo crece
  hacia la izquierda (animado, 180 ms, ease-out) sobre las acciones, que se
  ocultan; la info sigue visible. Toma el foco al abrir. Escape borra el
  texto y, vacío, la cierra; también la X. Al perder el foco solo se cierra
  si está vacía. Cerrar borra el texto.
- **Indicadores**: Ordenar y Filtrar llevan un punto cuando están activos
  (y «(activo)» en su nombre accesible).
- **Overflow** (pendiente): en paneles angostos las acciones secundarias
  deberían pasar a un menú «más»; falta saber el ancho del panel en `view`.

## API

| Firma | Qué hace |
|---|---|
| `list_toolbar(info)` | La barra, con su texto informativo. Encadena `.action`, `.search`, `.placeholder`, `.test_id` y `.build()`. |
| `ListAction::button(icon, label, msg)` | Acción simple. |
| `ListAction::menu(icon, label, items, selected, on_select)` | Acción que abre un menú (el pop-up del framework). |
| `.active(bool)` | Punto indicador sobre la acción. |
| `SearchReveal` | Estado de la animación: `open(tick)`, `close(tick)`, `tick(tick)` devuelven `Effects::after` cada 16 ms hasta reposar; `progress` 0–1, `eased()`. |

Partes: `<id>.info`, `<id>.search`, `<id>.field` (y `.field.input`),
`<id>.close`.

## Relacionados

[Paneles](panes.md) · [SearchField](search-field.md) · [Toolbar](toolbar.md)
