# 08 — Accesibilidad (spike AccessKit)

## Decisión

Kai Desktop expone su UI dibujada a mano a las tecnologías de asistencia mediante **AccessKit**. El spike integra el adaptador oficial de winit y deja la construcción del árbol como función pura, testeable sin ventana.

| Crate | Versión | MSRV | Licencia | Notas |
|---|---|---|---|---|
| `accesskit` | 0.25.1 | ≤ 1.87 | MIT OR Apache-2.0 | Tipos del árbol (`Node`, `Role`, `TreeUpdate`, `ActionRequest`). |
| `accesskit_winit` | 0.34.1 | 1.87 | Apache-2.0 | Compatible con winit 0.30. Se usa con `default-features = false` y `rwh_06`; en macOS arrastra `accesskit_macos` 0.27. |

El MSRV del crate sigue siendo 1.89 (lo impone `cosmic-text` 0.19).

## Integración

- `kai-ui-desktop/src/a11y.rs`:
  - `a11y_tree(&A11ySnapshot) -> TreeUpdate` genera el árbol completo desde el `Layout` del árbol `Element`: `Window` > un nodo por elemento visible. El rol sale de `Element::role` (`Main`, `Banner` para `Header`, `Group`, `Heading`, `Paragraph`, `Label`, `Status`, `Button`, `Switch`, `TextInput`, `Image`) o se infiere: input enmascarado → `PasswordInput` (siempre, aunque el rol explícito sea `TextInput`), input → `TextInput`, icono → `Image`, control con `on_click` → `Button`, texto → `Label`, caja → `GenericContainer`. El id de nodo es `WidgetId + 1`; el 0 es la ventana.
  - El rol de Kai y el nombre accesible salen de dos funciones públicas, `effective_role` y `accessible_name`, que también usan las consultas de `kai::testing`. Así un test encuentra cada elemento con el mismo nombre que lee VoiceOver. El nombre es el `label` explícito; si no hay, en un input es el placeholder, en un botón su texto y en un texto su contenido (que se publica como `value`).
  - Botones (también el botón por defecto, `DefaultButton`), switches e inputs son hojas: el texto de un botón queda dentro de su nombre y no aparece como hijo.
  - La pista de un switch tiene `Role::Switch`, que se publica como `Switch` de AccessKit con `Toggled` según su estado (`pressed`). En `switch_labeled` su nombre es el texto del label («Notificaciones»); un switch sin label se llama «Activado» o «Desactivado». La etiqueta de un input es su `label` (la etiqueta del `TextField`) o el placeholder.
  - `test_id` se publica como `author_id`, que en macOS es el `accessibilityIdentifier`. Esto incluye los ids derivados de las partes de un componente (`form.nombre.input`, ver [05](05-widgets-y-tema.md)).
  - `Alert` sigue publicándose como `Status`: `Role::Alert` de AccessKit es una región viva que interrumpe al lector, y el banner de la demo es informativo.
  - Un input publica `placeholder` y `value`, salvo si está enmascarado: un `PasswordInput` nunca expone su valor. `read_only` se publica con `set_read_only`, `required` con `set_required`, y `description` (el helper text del `TextField`) con `set_description`.
  - Los iconos sin `label` son decorativos (`aria-hidden`) y se omiten del árbol; con `label` se publican como `Image` con ese nombre. El botón de ojo del password se llama "Mostrar contraseña" u "Ocultar contraseña" según el estado.
  - Los controles deshabilitados se publican con `disabled` y sin acciones. Los nodos con `display: none` se omiten.
  - `A11ySnapshot::active_descendant` (input, fila) publica la fila que resaltan las flechas como `active_descendant` del input enfocado. La usa la paleta de comandos: el foco se queda en la búsqueda para seguir escribiendo, y VoiceOver lee la fila elegida (`Row` con `selected`) como foco efectivo.
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

- Roles, nombres (incluido el del botón por defecto), valor y placeholder del campo, acciones soportadas por el botón, estado `disabled`, `test_id` como `author_id`, y el switch con label como `Switch` con su nombre y `Toggled`.
- Iconos decorativos omitidos y etiquetados como `Image`; password como `PasswordInput` sin valor, con `required` y descripción; read-only con su valor.
- Transform de escala en la raíz y jerarquía que replica el árbol `Element`.
- Foco reportado según `UiModel` y bounds que siguen al layout, al scroll y al clip.
- Paleta (`demo.rs`): la búsqueda conserva el foco y su `active_descendant` sigue a la fila elegida con las flechas.
- Traducción de `Click`, `Focus` y `ReplaceSelectedText` a intents; las acciones sobre nodos no interactivos o deshabilitados se ignoran.

## Limitaciones conocidas

- El árbol se reconstruye completo en cada frame; no hay actualizaciones incrementales.
- El `TextInput` no publica todavía selección de texto ni runs de caracteres (`set_text_selection`, `Role::TextRun`); los lectores de pantalla leen el valor completo pero no la posición del caret.
- No se implementan `SetValue`, `ScrollIntoView` ni acciones de scroll de los contenedores.
- Los landmarks dependen de que la app asigne `Role::Main`/`Header`/`Group`; la demo lo hace.

El árbol AccessKit se deriva del mismo `Layout` que paint y hit-testing (ver [02](02-geometria-y-layout.md) y [05](05-widgets-y-tema.md)), con ids estables por ruta o `key`.
