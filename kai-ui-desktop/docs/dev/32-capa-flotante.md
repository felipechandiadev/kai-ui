# Capa flotante: popovers anclados

Estado: **implementado** (parche 06). Código: `src/overlay.rs` (API y
posicionamiento), `src/layout.rs` (`place_floating`, orden de pintura),
`src/render/paint.rs`, `src/session/floating.rs` (foco y descarte) y
`pointer_target` en `src/runtime.rs`. Pruebas: `overlay::tests` (layout,
hit-test, flip) y `session::floating_tests` (foco, descarte, pintura).

## 1. Problema

Un popover (calendario de un campo de fecha, lista de un combobox, menú de
un botón, tarjeta al pasar el mouse) tiene que:

- dibujarse **encima** del resto de la ventana, no sólo de sus hermanos;
- no quedar **recortado** por un `overflow-hidden` o un contenedor con
  scroll de algún ancestro (una tarjeta, un formulario con scroll);
- ubicarse junto al rectángulo **ya calculado** de su ancla, y cambiar de
  lado o deslizarse para no salirse de la ventana;
- cerrarse con Escape y con un clic afuera, y decidir qué pasa con el foco.

### 1.1. Qué había (análisis de los `z-50` existentes)

| Pieza | Cómo se monta | Limitación |
|---|---|---|
| Panel del `DateField` (`src/components/date.rs`) | `absolute top-1 left-0 z-50` dentro de un `relative z-50 h-0` dentro de un envoltorio `relative z-50`; `.modal(on_toggle)` | El `z-index` sólo ordena **entre hermanos** (ver el comentario en `build`): hace falta subir el envoltorio y aun así un hermano posterior del formulario con `z-*` mayor, o cualquier ancestro con `overflow-hidden` o scroll, lo tapa o lo recorta. `.modal()` además bloquea los clics afuera en vez de cerrar. |
| Lista de un pop-up button (`Popup` en `session.rs`) | La sesión la agrega como **hijo de la raíz** (`absolute inset-0 z-50`) y calcula su marco con `menu_frame` desde el rectángulo del dueño | Funciona, pero es exclusiva de la sesión: la app no puede poner su propio contenido ahí. Clic afuera cierra y se consume. |
| Paleta y diálogo de confirmación | Hijos de la raíz que agrega la sesión, `z-50` | Igual: capas de la sesión. |
| Tooltip | Hijo de la raíz, `z-60`, `tooltip_origin` estima su tamaño por el texto | Sólo texto. |
| Diálogos de la app (`Dialog`) | `absolute inset-0 z-50`; la demo los pone como última capa de la raíz (`overlay()`, docs/dev/27) | Correcto mientras estén en la raíz. |
| Toasts (`toast_stack`) | `absolute right-4 bottom-4 z-40` en el ancestro `relative` más cercano | No anclados a un elemento; no los toca este parche. |

Conclusión: el patrón «hijo de la raíz» ya es la capa flotante de facto,
pero sólo la sesión podía usarlo. La primitiva nueva le da a cualquier
elemento de la app esa misma capa sin moverlo de su lugar en el árbol.

## 2. Diseño

### 2.1. El elemento se queda donde está

`Element::floating(Floating)` marca un hijo; su **padre es el ancla**. En el
árbol de elementos no se mueve (a diferencia de un *portal* que lo
trasladara a la raíz), así que:

- sus `WidgetId` posicionales y sus `test_id` derivados no cambian;
- el orden de Tab lo pone **justo después de su ancla**;
- los mensajes, `on_action`, la cadena de respuesta (Escape sube por sus
  ancestros) y el árbol de accesibilidad son los de ese lugar;
- el camino de hit-test pasa por el ancla, así que un panel modal que
  contenga al ancla sigue conteniendo al popover.

### 2.2. Layout

En `Layout::compute`:

1. El `Builder` construye el nodo flotante pero **no** lo cuelga de los
   hijos Taffy de su padre: no ocupa lugar en el flujo ni cambia el tamaño
   del ancla. Hereda el estilo de texto de la raíz (como cualquier capa
   superior) salvo `hidden` y el tamaño de control, que vienen del ancla.
2. Tras ubicar el árbol, `place_floating` (en orden de documento, así un
   flotante dentro de otro va después) calcula su tamaño como un `position:
   fixed`: absoluto dentro de una caja del tamaño de la ventana. Con
   `match_width` su `min-width` (o `min-height` a los lados) es la del
   ancla, salvo que la clase ya fije una.
3. `overlay::position(ancla, tamaño, ventana, placement)` elige el lugar
   (ver 2.3) y el nodo se ubica ahí **sin clip** heredado. Su
   `LayoutNode::floating` guarda el lado final.
4. El ancla se toma ya desplazada por el scroll de sus contenedores, así que
   el popover acompaña al ancla al hacer scroll.

`reveal` no hace scroll en los contenedores del ancla para mostrar algo de
dentro del popover, y la rueda sobre el popover no hace scroll de lo que
está debajo (`scroll_target` se corta en el nodo flotante).

### 2.3. Posición: lado, alineación, flip y shift

`Placement { side, align, gap, margin, flip, shift, match_width }`:

- `side`: `Below` (por defecto), `Above`, `Right`, `Left`; `align`:
  `Start`, `Center`, `End` en el eje cruzado.
- **flip**: si no cabe en `side` va al lado opuesto si allí cabe, o si allí
  hay más espacio.
- **shift**: se desliza en el eje cruzado para quedar a `margin` (8 pt) de
  los bordes; si no cabe en ningún lado, también en el eje principal (queda
  sobre el ancla en lugar de salirse). Más alto que la ventana: desde el
  margen superior.
- `gap`: 4 pt entre ancla y popover (lo mismo que `menu_frame`).

### 2.4. Capas, pintura y hit-test

La raíz pinta sus hijos ordenados por `z-index` (como antes). Cada
flotante pertenece al **nivel** del hijo de la raíz que lo contiene (su
`z-index`), y se pinta **después del último hijo de la raíz de ese nivel**
(`Layout::floats_after`). El hit-test usa el mismo orden al revés.

| Orden (de abajo arriba) | Ejemplo |
|---|---|
| Hijos de la raíz con `z` 0 (la app: barra lateral, contenido, toolbar) | — |
| **Flotantes de esos hijos**, en orden de documento | combobox del formulario, menú de la barra lateral |
| Hijos de la raíz con `z-40` | toasts puestos en la raíz |
| Flotantes de esos hijos | — |
| Hijos de la raíz con `z-50` (diálogo de la app, lista de pop-up y paleta de la sesión) | — |
| Flotantes de esos hijos | calendario dentro de un diálogo |
| `z-60` tooltip, `z-100` inspector | — |

Por eso un popover del contenido queda sobre cualquier hermano con `z-*`
dentro del contenido y sobre la barra lateral vecina, y debajo de un
diálogo de la app o de las capas de la sesión. Regla para las apps: los
diálogos y capas propias van como **hijos de la raíz** con `z-*` (como ya
hace `overlay()` de la demo).

`pointer_target` se corta en el nodo flotante: un clic en el fondo del
popover no activa el control del ancla (p. ej. el botón que lo abrió).

### 2.5. Foco (`OverlayFocus`)

- `Anchor` (por defecto; combobox, tarjeta): el foco no se mueve. Se puede
  seguir escribiendo en el campo; Tab sigue el orden del documento y entra
  al popover justo después del ancla.
- `Contain` (menú, calendario): al aparecer el foco va al primer control de
  dentro, Tab queda dentro, los atajos de la app se apagan (igual que con
  las listas de la sesión) y al desaparecer el foco vuelve a donde estaba,
  salvo que ya se haya ido a otro elemento vivo. Usa el mismo
  `modal_focus` (`ModalFocus::First` / `Restore`) que `Element::modal`.

En `rebuild`, la cadena que elige qué subárbol da el orden de foco queda:
paleta/confirmación → lista de pop-up → **flotante `Contain` más interno**
→ panel modal de la app.

### 2.6. Descarte

`Floating::on_dismiss(msg)`; la app guarda si está abierto y lo cierra al
recibirlo (mismo patrón que `Dialog`/`DateField::calendar`).

- **Escape / ⌘.** con el foco dentro del popover: sube por la cadena de
  respuesta (`Element::floating` envuelve `on_action` como `modal`), así un
  control de dentro que lo use (un buscador con texto) va primero.
- **Escape / ⌘.** con el foco fuera: la sesión (`float_key`) lo envía al
  flotante más interno cuyo ancla tiene el foco (modo `Anchor`: el campo de
  un combobox) o que contiene el foco sin tener nada enfocable (`Contain`).
- **Clic afuera** (fuera del ancla y por tanto del popover): se envía
  `on_dismiss` de todos los flotantes abiertos afectados, del más interno al
  más externo. En modo `Anchor` el clic **sigue** hacia lo que está debajo;
  en modo `Contain` se **consume** (como las listas de la sesión). Un clic
  en el ancla no lo descarta: el ancla decide (p. ej. alternar).
- Un panel modal de la app o la confirmación siguen bloqueando antes.

### 2.7. Accesibilidad

El popover no recibe un rol automático: se le da el que corresponde
(`Role::Menu`, `Role::List`, `Role::Dialog`) y un nombre (`label`), y al
control que lo abre `expanded(open)`. Como está en su lugar del árbol, el
lector de pantalla lo encuentra junto a su ancla y `By::role(...)` lo
encuentra en las pruebas.

### 2.8. Combobox (`Autocomplete`, parche 07)

El [Autocomplete](../componentes/autocomplete.md) es el primer usuario del
modo `Anchor` con `match_width`. Un campo de una línea consume ↑/↓ (inicio y
fin del texto) y Tab mueve el foco, así que la sesión los enruta antes:
`float_key` llama a `combobox_key` cuando el foco está en un `Input` con
`Role::ComboBox`. ↑, ↓ y Retorno sin modificadores van como
`Action::MoveUp`/`MoveDown`/`InsertNewline` por la cadena de responders
(`action_message`) y se consumen solo si alguien los maneja; Tab con la
lista abierta envía su `on_dismiss`, reconstruye y deja que el foco siga
al próximo control (nunca entra a las opciones). El `active_descendant` del
snapshot de accesibilidad sale de la opción `ListBoxOption` presionada del
flotante del campo enfocado.

## 3. API

```rust
use kai::overlay::{Align, Floating, OverlayFocus, Side};

// Menú: el foco entra y vuelve. El ancla es el padre del popover.
let mut anchor = div().class("flex").child(
    button("Opciones", ButtonVariant::Secondary)
        .expanded(open)
        .on_click(Msg::ToggleMenu),
);
if open {
    anchor = anchor.child(
        div()
            .class("w-56 p-1 flex flex-col rounded-md border bg-background shadow-lg")
            .role(Role::Menu)
            .label("Opciones")
            .children(items)
            .floating(
                Floating::below()
                    .align(Align::End)
                    .contain_focus()
                    .on_dismiss(Msg::CloseMenu),
            ),
    );
}

// Combobox: el foco se queda en el campo y la lista es al menos igual de ancha.
let list = div()
    .class("max-h-60 overflow-y-auto p-1 border bg-background shadow-lg")
    .role(Role::List)
    .label("Sugerencias")
    .children(rows)
    .floating(Floating::below().match_width().on_dismiss(Msg::CloseList));
```

| Constructor / método | Efecto |
|---|---|
| `Floating::new(side)`, `below()`, `above()`, `right()`, `left()` | Lado preferido; empieza alineado al inicio, con flip y shift |
| `.align(Align)` | Alineación en el eje cruzado |
| `.gap(pt)`, `.margin(pt)` | Separación del ancla (4) y de los bordes (8) |
| `.no_flip()`, `.no_shift()` | Quedarse en el lado o en la posición pedida |
| `.match_width()` | Al menos tan ancho (o alto, a los lados) como el ancla |
| `.focus(OverlayFocus)`, `.contain_focus()` | Regla de foco (2.5) |
| `.on_dismiss(msg)` | Mensaje de Escape y clic afuera (2.6) |
| `overlay::position(anchor, size, viewport, placement)` | La función pura de 2.3, por si se quiere reutilizar (p. ej. en `menu_frame`) |

## 4. Limitaciones conocidas

- Si el ancla sale de la vista por scroll, el popover la sigue fuera del
  contenedor (no se oculta). Los menús y combobox suelen cerrarse antes; se
  podría ocultar cuando el ancla no tenga `visible_bounds`.
- El tooltip y los `group-hover` del ancla se pueden activar mientras el
  mouse está sobre el popover (el camino de hover pasa por el ancla).
- La opacidad o el `moved` de un ancestro del ancla no se aplican al
  popover (se pinta desde la raíz).
- Un diálogo de la app que **no** sea hijo de la raíz queda debajo de los
  popovers de su nivel.
- No hay animación de entrada ni flecha.
- Las listas de la sesión (`Popup`), la paleta, el tooltip y los toasts
  siguen con su mecanismo propio; podrían adoptar `position` más adelante.

## 5. Plan de migración del `DateField`

Este parche **no** toca `src/components/date.rs`. Pasos propuestos (parche
opcional 06b, aplicable cuando convenga):

1. En `DateField::build`, quitar `absolute top-1 left-0 z-50` de las clases
   del panel y reemplazar `.modal(popover.on_toggle)` por
   `.floating(Floating::below().contain_focus().on_dismiss(popover.on_toggle))`.
2. Colgar el panel directamente del campo (`field.child(panel)`) y borrar
   los dos envoltorios `relative z-50` y su comentario. Como el flotante no
   ocupa lugar, el campo mide lo mismo abierto o cerrado y sus ids no
   cambian al abrir.
3. Cambio de comportamiento a revisar: hoy un clic afuera **no hace nada**
   (modal); con la capa flotante **cierra** el calendario y se consume.
   Escape, el foco inicial (primer control del calendario), Tab dentro y la
   vuelta del foco siguen igual.
4. Volver a correr las pruebas del calendario (`components::date`,
   `catalog` de la demo) y revisar a mano un `DateField` dentro de una
   tarjeta con `overflow-hidden` y al pie de la ventana (debe abrir hacia
   arriba).
5. Después: los futuros combobox y menús de `components` (hoy
   `src/datetime.rs` y `src/forms/` no arman paneles propios).
