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

## Reglas (normativas)

Patrón de referencia del laboratorio (demo «App real»). Toda barra de
lista nueva las cumple; cambiarlas exige cambiar este documento.

- **R1. Barra de 44 pt.** Deja 8 pt arriba y abajo de un control de 28 pt.
- **R2. Iconos de 28 pt** (`IconButtonSize::Sm`) con tooltip. Blanco
  táctil/clicable cómodo sin engordar la barra.
- **R3. Campo de búsqueda de 28 pt.** Misma línea que los iconos: abrir la
  búsqueda no mueve nada en vertical.
- **R4. Margen mínimo de 8 pt para hover y foco.** El relleno de hover y el
  anillo de foco nunca tocan ni se recortan con el borde de la barra.
- **R5. Anillo de foco de 2 pt** (token `focus_ring_width`, color
  `focus_ring`: cian en Glass). Fino y con contraste sobre la aurora; un
  halo de 3 pt pide más margen y ensucia.
- **R6. Una sola X para borrar:** la del campo. Dos X (borrar y cerrar)
  confunden; cerrar es Escape o salir.
- **R7. Info a la izquierda, acciones a la derecha.** Se lee primero el
  estado («8 clientes», «N de M clientes», solo lectura) y luego qué hacer.
- **R8. Orden de acciones:** Descargar, Ordenar, Filtrar, Añadir y la
  búsqueda al extremo derecho. De lo que no cambia la lista a lo que la
  cambia; la búsqueda crece hacia la izquierda sin saltar.
- **R9. Indicador de activo:** punto en Ordenar/Filtrar cuando alteran la
  lista, y «(activo)» en el nombre accesible. Explica por qué faltan filas.
- **R10. Cerrar la búsqueda:** Escape borra el texto y, vacío, cierra;
  perder el foco cierra solo si está vacío; cerrar borra el texto. Nunca se
  pierde texto escrito por un clic accidental.
- **R11. Despliegue animado** (180 ms, ease-out) desde el botón sobre las
  acciones, que se ocultan; la info queda visible y el campo toma el foco.
  Continuidad: el campo sale del icono que se pulsó.

Pendiente: overflow a un menú «más» en paneles angostos (falta el ancho
del panel en `view`).

## API

| Firma | Qué hace |
|---|---|
| `list_toolbar(info)` | La barra, con su texto informativo. Encadena `.action`, `.search`, `.placeholder`, `.test_id` y `.build()`. |
| `ListAction::button(icon, label, msg)` | Acción simple. |
| `ListAction::menu(icon, label, items, selected, on_select)` | Acción que abre un menú (el pop-up del framework). |
| `.active(bool)` | Punto indicador sobre la acción. |
| `SearchReveal` | Estado de la animación: `open(tick)`, `close(tick)`, `tick(tick)` devuelven `Effects::after` cada 16 ms hasta reposar; `progress` 0–1, `eased()`. |

Partes: `<id>.info`, `<id>.search`, `<id>.field` (y `.field.input`,
`.field.clear`).

## Relacionados

[Paneles](panes.md) · [SearchField](search-field.md) · [Toolbar](toolbar.md)
