# 05 — Widgets y tema Kai

**Fase:** L6 (frontend y árbol de widgets reactivos). No iniciar la migración de widgets hasta que estado/comandos, layout y paint tengan contratos verificables.

## Objetivo

Crear una API declarativa de componentes propios que se vean y se compongan como Kai UI web: un árbol tipo DOM, estilos con clases tipo Tailwind resueltas contra los tokens del tema, y componentes como funciones del modelo (arquitectura Elm, ver [03](03-eventos-y-estado.md)).

## Orden de implementación

1. Definir el árbol mínimo: identidad estable, hijos, estado/props y vínculo con bounds de L4; evitar API pública prematura.
2. **Theme:** colores semánticos, tipografía, escala de espaciado, radios, bordes, alturas y densidad; tokens web son referencia, no runtime.
3. Migrar **Text** y **Button** al flujo árbol → comandos/estado → layout → paint.
4. Migrar **TextField** preservando selección, cursor, teclado, IME y portapapeles a través del adaptador de plataforma.
5. Verificar el ciclo integrado con app demo y pruebas antes de ampliar el catálogo.
6. Incorporar luego Select/AutoComplete y Dialog/Alert/Toast, atendiendo accesibilidad, foco, clipping y acciones.
7. Después: Card, Skeleton, LoadingState, Calendar y DataGrid según prioridad de las apps desktop.

## API y composición

- Definir props Rust tipadas y evitar una API que copie atributos HTML.
- Los widgets describen estructura, estado visual y mensajes; no llaman directamente a servicios ni leen/escriben archivos.
- Los componentes compuestos se construyen con primitivas y layout, salvo que un control necesite un tratamiento especializado.
- Mantener tema y estado de interacción fuera de valores de color codificados en cada componente.
- Definir una ruta para iconos/SVG y verificar licencias; no aceptar cualquier SVG sin política de seguridad y límites de complejidad.
- Si se requieren docking, tabs o toolbar overflow, evaluar las piezas de `craft-ui` por separado; consultar [07-integracion-craft-libs.md](07-integracion-craft-libs.md) y no adoptar su renderer egui como árbol visual de Kai.

## Traslado desde Kai Web

Consultar `web/src/theme/tokens.css`, los README de componentes y `web/DESIGN-SYSTEM.md` para recuperar intención, variantes, estados y semántica. Mapear cada token a unidades y capacidades reales del renderer. Marcar explícitamente los tokens sin equivalente o que requieran adaptación.

## Criterios de aceptación

- Cada widget inicial tiene documentación de props, estados, teclado, accesibilidad y ejemplos.
- Variantes visuales derivan del tema; cambios de tema no requieren editar todos los widgets.
- Foco y teclado son utilizables sin mouse.
- Las pruebas cubren transición de estados, mensajes emitidos y geometría interactiva.

## Estado actual de L6

- [x] `theme.rs`: `Theme::kai(Density)` refleja `web/src/theme/tokens.css` con `1rem = 16` puntos lógicos (colores, `--space-*`, tipografía, interlineados, pesos y alturas de control) en densidad `Comfortable` y `Compact`. `--color-hover` y `--color-text-disabled` replican `color-mix` en sRGB. `Theme::color(nombre)` resuelve los nombres de color que usan las clases y `UNMAPPED_TOKENS` lista los tokens sin equivalente, con el motivo.
- [x] `element.rs`: árbol `Element<Msg>` (`div`, `text`, `input`) con clases, rol semántico, `label`, `disabled`, `key` y handlers de mensajes. `FocusOrder::from_tree` deriva el orden de Tab (orden de documento, solo botones e inputs habilitados).
- [x] `style.rs` + `tw.rs`: `Style` tipo CSS y parser de clases tipo Tailwind (abajo).
- [x] `components.rs`: Button (5 variantes y 3 tamaños), TextInput, TextField con etiqueta flotante, Card (header, content, actions), Stat, Heading, Paragraph y Caption, con las clases de los componentes web.
- [x] `main.rs`: demo con contador, formulario controlado, estadísticas y variantes de botón; un test garantiza que solo usa clases soportadas.
- [x] Radios, bordes, sombras, anillos de foco y opacidad, gracias al renderer SDF ([04](04-renderer-y-texto.md)).
- [ ] Select/AutoComplete, Dialog/Alert/Toast, iconos/SVG, transiciones y animaciones.

## Clases tipo Tailwind (`tw.rs`)

`ClassList::parse(clases, &tema)` convierte una cadena de clases en declaraciones por variante; `resolve(State)` aplica, en orden de prioridad CSS, base → `group-hover:` → `group-focus-within:` → `hover:` → `focus-within:` → `focus:` → `active:` → `disabled:`. Con `disabled`, `hover:` y `active:` se ignoran. Las clases desconocidas se reportan (`ClassList::unknown`) en lugar de fallar en silencio.

| Grupo | Clases |
|---|---|
| Display y flex | `block` `flex` `grid` `hidden` `inline-flex` `inline-block` `flex-row` `flex-col` `flex-wrap` `flex-1` `flex-auto` `flex-none` `grow` `shrink-0` `items-*` `self-*` `justify-*` |
| Espaciado | `gap-*` `gap-x-*` `gap-y-*` `p*-*` `m*-*` (negativos y `mx-auto`) `inset-*` `top/right/bottom/left-*` |
| Tamaño | `w-*` `h-*` `size-*` `min-w/h-*` `max-w/h-*` (`max-w-xs`…`7xl`) fracciones `full` `screen` `h-control` `h-control-compact` `min-h-control` `py-control` |
| Posición y overflow | `relative` `absolute` `z-N` `overflow-visible/hidden/clip/auto/scroll` `overflow-y-auto/scroll` `grid-cols-N` |
| Color | `bg-*` `text-*` `border-*` `ring-*` `placeholder-*` con nombre del tema, alfa `/N` o `[#hex]` |
| Borde y radio | `border` `border-N` `border-t/b/l/r/x/y` `rounded-*` (`sm` 2, `md` 6, `lg` 8, `xl` 12, `2xl`, `3xl`, `full`) |
| Efectos | `shadow-sm/md/lg/xl/none` `ring` `ring-N` `ring-offset-N` `outline-none` `opacity-N` |
| Texto | `text-xs`…`3xl` `text-[Npx]` `font-normal/medium/semibold/bold` `leading-*` `whitespace-nowrap` `truncate` |
| Interacción | `cursor-*` `pointer-events-none/auto` `group` |

- El espaciado usa la unidad del tema (`--space-4 / 4`), así que `p-4` es `--space-4` y escala con la densidad; los valores arbitrarios (`w-[180px]`) no escalan.
- `text-lg` y `text-xl` usan los tamaños de título de página y sección del tema; `leading-*` siempre gana al interlineado implícito del tamaño, sin importar el orden.
- Las propiedades de texto se heredan como en CSS (color, tamaño, interlineado, peso y `nowrap`).

## Componentes (`components.rs`)

Los componentes son funciones que devuelven `Element<Msg>`; se componen como JSX y se ajustan encadenando `.on_click`, `.disabled` o más `.class(..)`. El estado vive en el modelo; el único estado propio del framework es el caret y la selección de cada input.

### Button

```rust
button("Guardar", ButtonVariant::Primary).on_click(Msg::Save).disabled(!valid)
button_sized("Más", ButtonVariant::Outlined, ButtonSize::Sm)
```

- Base (`.fs-button`): `inline-flex`, `rounded-md`, borde transparente de 1 pt, `font-medium`, `text-sm`/`leading-5`, `whitespace-nowrap`, `cursor-pointer`. Con foco, anillo `ring-2 ring-primary/40 ring-offset-2`. Deshabilitado, `opacity-50` y `cursor-not-allowed`.
- Variantes: `Primary` (primary → accent en hover → active), `Secondary` (`--color-secondary`), `Outlined` (borde foreground que se rellena con accent en hover), `Text` (texto primary con fondo translúcido en hover) y `Danger` (error). Tamaños: `Sm`, `Md` (`h-control`) y `Lg`.
- Teclado: Tab/Shift+Tab lo enfocan; Enter o Espacio (sin repetición) lo activan. Puntero: activa si se suelta sobre el mismo botón donde se presionó. Un botón deshabilitado sigue capturando el puntero, pero no recibe foco ni activa.
- Accesibilidad: `Role::Button` con la etiqueta igual a su texto; acciones `Click` y `Focus`, o `disabled` sin acciones.

### TextInput y TextField

```rust
text_field("Nombre", text_input(&model.name, "").on_input(Msg::NameChanged).on_submit(Msg::Submit))
```

- `text_input(valor, placeholder)` es un input controlado con las clases de `textfield.css`: `h-control`, `rounded-md`, borde `border-border` (hover `muted-foreground`, foco `primary`), `px-3`, `text-sm`, placeholder `muted-foreground`.
- `text_field(etiqueta, input)` añade la etiqueta flotante de la web: con valor o foco, la etiqueta se coloca sobre el borde superior (`absolute -top-2 z-20 bg-background`); si no, se muestra el placeholder (por defecto, la etiqueta). Funciona con `group` + `group-focus-within:block`, sin estado extra.
- Teclado: flechas, Inicio/Fin, Shift extiende la selección, Backspace/Supr, Cmd+A/C/X/V, Enter emite `on_submit`. IME: preedit subrayado sin modificar el valor; el commit inserta una sola vez. Los errores del portapapeles se muestran como estado.
- Accesibilidad: `Role::TextInput` con `label`, `placeholder` y `value`; acciones `Focus`, `Click` y `ReplaceSelectedText`.

### Card y otros

- `card()` (`.fs-card`): `bg-background`, borde `border/70`, `rounded-lg`, `shadow-sm`, `overflow-hidden`. `card_header(título, subtítulo)`, `card_content()` y `card_actions()` (separador superior, acciones alineadas a la derecha).
- `stat(etiqueta, valor, pista)`: tarjeta compacta de estadística.
- `heading`, `paragraph` y `caption`: texto con rol y estilos de la tipografía del tema.
