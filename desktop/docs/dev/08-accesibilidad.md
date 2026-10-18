# 08 — Accesibilidad (spike AccessKit)

## Decisión

Kai Desktop expone su UI dibujada a mano a las tecnologías de asistencia mediante **AccessKit**. El spike integra el adaptador oficial de winit y deja la construcción del árbol como función pura, testeable sin ventana.

| Crate | Versión | MSRV | Licencia | Notas |
|---|---|---|---|---|
| `accesskit` | 0.25.1 | ≤ 1.87 | MIT OR Apache-2.0 | Tipos del árbol (`Node`, `Role`, `TreeUpdate`, `ActionRequest`). |
| `accesskit_winit` | 0.34.1 | 1.87 | Apache-2.0 | Compatible con winit 0.30. Se usa con `default-features = false` y `rwh_06`; en macOS arrastra `accesskit_macos` 0.27. |

El MSRV del crate sigue siendo 1.89 (lo impone `cosmic-text` 0.19).

## Integración

- `desktop/src/a11y.rs`:
  - `a11y_tree(&A11ySnapshot) -> TreeUpdate` genera el árbol completo desde el `Layout` del árbol `Element`: `Window` > un nodo por elemento visible. El rol sale de `Element::role` (`Main`, `Banner` para `Header`, `Group`, `Heading`, `Paragraph`, `Label`, `Status`, `Button`, `TextInput`) o se infiere: input → `TextInput`, control con `on_click` → `Button`, texto → `Label`, caja → `GenericContainer`. El id de nodo es `WidgetId + 1`; el 0 es la ventana.
  - Botones e inputs son hojas. La etiqueta de un botón es su texto; la de un input, su `label` (la etiqueta del `text_field`) o el placeholder.
  - Los controles deshabilitados se publican con `disabled` y sin acciones. Los nodos con `display: none` se omiten.
  - `intents_for_action(&ActionRequest, &FocusOrder) -> Vec<Intent>` traduce acciones de asistencia a intents de Kai solo para elementos habilitados y focusables: `Focus` → `SetFocus`, `Click` → `Intent::Activate`, `ReplaceSelectedText` en un input → foco + `InsertText`.
- `Intent::Activate(id)` en `ui_engine.rs` enfoca el elemento y, si es un botón, lo activa con `ActivationSource::Accessibility`; el runtime lo convierte en su mensaje `on_click`.
- `runtime.rs`:
  - La ventana se crea con `with_visible(false)`; el `Adapter` se crea con `with_event_loop_proxy` y solo después se muestra la ventana (requisito del adaptador, que hace panic si la ventana ya es visible).
  - Cada `WindowEvent` pasa primero por `Adapter::process_event`.
  - `InitialTreeRequested` y cada frame renderizado llaman `update_if_active` con el árbol completo; `ActionRequested` se despacha como intents normales.

## Coordenadas

AccessKit en macOS espera píxeles físicos (divide por `backingScaleFactor`). El layout de Kai está en puntos lógicos, así que el nodo raíz lleva `transform = Affine::scale(scale_factor)` y todos los bounds se expresan en puntos lógicos relativos a la ventana. Los bounds son los visibles (`LayoutNode::visible_bounds`): se recortan a los clips de los contenedores con overflow, y un nodo totalmente fuera de la vista no tiene bounds.

## Cobertura automática

Tests en `a11y.rs` y `ui_engine.rs`:

- Roles, nombres, valor y placeholder del campo, acciones soportadas por el botón, estado `disabled`.
- Transform de escala en la raíz y jerarquía que replica el árbol `Element`.
- Foco reportado según `UiModel` y bounds que siguen al layout, al scroll y al clip.
- Traducción de `Click`, `Focus` y `ReplaceSelectedText` a intents; las acciones sobre nodos no interactivos o deshabilitados se ignoran.

## Limitaciones conocidas

- El árbol se reconstruye completo en cada frame; no hay actualizaciones incrementales.
- El `TextInput` no publica todavía selección de texto ni runs de caracteres (`set_text_selection`, `Role::TextRun`); los lectores de pantalla leen el valor completo pero no la posición del caret.
- No se implementan `SetValue`, `ScrollIntoView` ni acciones de scroll de los contenedores.
- Los landmarks dependen de que la app asigne `Role::Main`/`Header`/`Group`; la demo lo hace.

El árbol AccessKit se deriva del mismo `Layout` que paint y hit-testing (ver [02](02-geometria-y-layout.md) y [05](05-widgets-y-tema.md)), con ids estables por ruta o `key`.
