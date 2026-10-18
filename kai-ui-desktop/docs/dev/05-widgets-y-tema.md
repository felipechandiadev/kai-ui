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
- Iconos/SVG: solo el catálogo Lucide generado y versionado en el repositorio (licencia ISC copiada junto a los SVG); no se cargan SVG arbitrarios en tiempo de ejecución.
- Si se requieren docking, tabs o toolbar overflow, evaluar las piezas de `craft-ui` por separado; consultar [07-integracion-craft-libs.md](07-integracion-craft-libs.md) y no adoptar su renderer egui como árbol visual de Kai.

## Traslado desde Kai Web

Consultar `kai-ui-web/src/theme/tokens.css`, los README de componentes y `kai-ui-web/DESIGN-SYSTEM.md` para recuperar intención, variantes, estados y semántica. Mapear cada token a unidades y capacidades reales del renderer. Marcar explícitamente los tokens sin equivalente o que requieran adaptación.

## Criterios de aceptación

- Cada widget inicial tiene documentación de props, estados, teclado, accesibilidad y ejemplos.
- Variantes visuales derivan del tema; cambios de tema no requieren editar todos los widgets.
- Foco y teclado son utilizables sin mouse.
- Las pruebas cubren transición de estados, mensajes emitidos y geometría interactiva.

## Estado actual de L6

- [x] `theme.rs`: `Theme::kai(Density)` refleja `kai-ui-web/src/theme/tokens.css` con `1rem = 16` puntos lógicos (colores, `--space-*`, tipografía, interlineados, pesos y alturas de control) en densidad `Comfortable` y `Compact`. `--color-hover` y `--color-text-disabled` replican `color-mix` en sRGB. `Theme::color(nombre)` resuelve los nombres de color que usan las clases y `UNMAPPED_TOKENS` lista los tokens sin equivalente, con el motivo.
- [x] `element.rs`: árbol `Element<Msg>` (`div`, `text`, `icon`, `input`) con clases, rol semántico, `label`, `disabled`, `required`, `description`, `key` y handlers de mensajes. Los inputs admiten `masked`, `read_only` y `select_on_focus`. `delegates_focus()` hace que un clic en un contenedor enfoque su primer input habilitado (como `<label for>`), y `focus_on_press(false)` activa un control con el puntero sin moverle el foco. `FocusOrder::from_tree` deriva el orden de Tab (orden de documento, solo botones e inputs habilitados).
- [x] `style.rs` + `tw.rs`: `Style` tipo CSS y parser de clases tipo Tailwind (abajo).
- [x] `components.rs`: Button (5 variantes y 3 tamaños), TextField (variantes, densidades, etiqueta flotante, estática o inline, adornos, password, required, helper, read-only y disabled), Card (header, content, actions), Stat, Heading, Paragraph y Caption, con las clases de los componentes web.
- [x] Iconos Lucide como SVG rasterizados en un atlas de máscaras ([04](04-renderer-y-texto.md)).
- [x] `main.rs`: demo con contador, formulario controlado, variantes de TextField, estadísticas y variantes de botón; un test garantiza que solo usa clases soportadas.
- [x] Radios, bordes, sombras, anillos de foco y opacidad, gracias al renderer SDF ([04](04-renderer-y-texto.md)).
- [ ] Select/AutoComplete, Dialog/Alert/Toast, textarea y formateadores del TextField, transiciones y animaciones.

## Clases tipo Tailwind (`tw.rs`)

`ClassList::parse(clases, &tema)` convierte una cadena de clases en declaraciones por variante; `resolve(State)` aplica, en orden de prioridad CSS, base → `group-hover:` → `group-focus-within:` → `hover:` → `focus-within:` → `focus:` → `active:` → `disabled:`. Con `disabled`, `hover:` y `active:` se ignoran. Las clases desconocidas se reportan (`ClassList::unknown`) en lugar de fallar en silencio.

| Grupo | Clases |
|---|---|
| Display y flex | `block` `flex` `grid` `hidden` `inline-flex` `inline-block` `flex-row` `flex-col` `flex-wrap` `flex-1` `flex-auto` `flex-none` `grow` `shrink-0` `items-*` `self-*` `justify-*` |
| Espaciado | `gap-*` `gap-x-*` `gap-y-*` `p*-*` `m*-*` (negativos y `mx-auto`) `inset-*` `top/right/bottom/left-*` |
| Tamaño | `w-*` `h-*` `size-*` `min-w/h-*` `max-w/h-*` (`max-w-xs`…`7xl`) fracciones `full` `screen` `h-control` `h-control-compact` `min-h-control` `py-control` |
| Posición y overflow | `relative` `absolute` `z-N` `overflow-visible/hidden/clip/auto/scroll` `overflow-y-auto/scroll` `grid-cols-N` |
| Color | `bg-*` `text-*` `border-*` `ring-*` `placeholder-*` con nombre del tema (incluye `field-muted`), alfa `/N` o `[#hex]` |
| Borde y radio | `border` `border-N` `border-t/b/l/r/x/y` `rounded-*` (`sm` 2, `md` 6, `lg` 8, `xl` 12, `2xl`, `3xl`, `full`) |
| Efectos | `shadow-sm/md/lg/xl/none` `ring` `ring-N` `ring-offset-N` `outline-none` `opacity-N` |
| Texto | `text-xs`…`3xl` `text-[Npx]` `font-normal/medium/semibold/bold` `leading-*` `whitespace-nowrap` `truncate` |
| Iconos | `stroke-N` `stroke-[N]` (ancho de trazo heredable) |
| Interacción | `cursor-*` `pointer-events-none/auto` `group` |

- El espaciado usa la unidad del tema (`--space-4 / 4`), así que `p-4` es `--space-4` y escala con la densidad; los valores arbitrarios (`w-[180px]`) no escalan.
- `text-lg` y `text-xl` usan los tamaños de título de página y sección del tema; `leading-*` siempre gana al interlineado implícito del tamaño, sin importar el orden.
- Las propiedades de texto se heredan como en CSS (color, tamaño, interlineado, peso, `nowrap` y el trazo de los iconos).

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

### TextField

```rust
TextField::new("Contraseña", &model.password)
    .password(model.show_password, Msg::TogglePassword)
    .start_icon(Icon::User)
    .required(true)
    .helper_text("Mínimo 8 caracteres")
    .on_input(Msg::PasswordChanged)
    .on_submit(Msg::Submit)
    .build()
```

Builder tipado con el alcance base de `kai-ui-web/src/components/TextField`. `build()` devuelve el `Element<Msg>`.

- **Estructura.** La raíz (`group`, `delegates_focus`) contiene el contorno y, si corresponde, el helper text. El contorno es una fila flex `[adorno inicial][input flex-1][adorno final][botón de ojo]` con el borde, el radio y `focus-within:border-primary`. Los adornos ocupan su propio espacio, en lugar de superponerse con `absolute` y medirse como en la web. Un clic en cualquier parte del campo (etiqueta, iconos, padding) enfoca el input.
- **Etiqueta.** En densidad `Default` flota sobre el borde superior (`absolute left-3 -top-2 z-20`) con foco o valor, usando como máscara la superficie del campo. Sin foco ni valor se muestra el placeholder, que por defecto es la etiqueta.
- **Props:**

| Prop | Efecto |
|---|---|
| `placeholder(s)` | Texto con el campo vacío y sin foco. |
| `variant(Normal \| Contrast \| Autocomplete)` | `Contrast`: borde y texto `background`, etiqueta sobre `bg-primary`, para barras primary. `Autocomplete`: sin borde y transparente. |
| `density(Default \| Compact)` | `Compact`: `h-control-compact`, etiqueta estática arriba (`text-[11px] font-medium leading-tight`), símbolos de 15 px e iconos `size-4`. |
| `label_layout(Stack \| Inline)` | Solo en compact. `Inline` pone la etiqueta dentro del borde, con `border-r` y `text-muted-foreground font-semibold`. `inline_leading(el)` agrega un elemento a su izquierda. |
| `start_symbol` / `start_icon` / `start_adornment` y sus `end_*` | Símbolo de texto (20 px), icono Lucide (`size-5`) o elemento libre, en `text-secondary`. |
| `password(revealed, on_toggle)` | Input enmascarado salvo con `revealed`, y botón redondo `size-8 rounded-full hover:bg-primary/10` con `Eye`/`EyeOff`. El botón tiene `focus_on_press(false)`: alterna sin quitar el foco al input y sigue siendo tabulable. El estado vive en el modelo. |
| `required(b)` | Asterisco `text-error` en la etiqueta y en un placeholder propio de dos colores (fila de textos superpuesta), porque el placeholder nativo tiene un solo color. |
| `helper_text(s)` | `mt-1 text-xs text-muted-foreground` debajo del campo; también es la descripción accesible. |
| `read_only(b)` / `disabled(b)` | Superficie apagada `bg-field-muted` (`color-mix(muted 50%, background)`), también tras la etiqueta, con contenido al 50 %. Disabled agrega `cursor-not-allowed`, saca el input del orden de foco y deshabilita el botón de ojo. |
| `select_on_focus(b)` | Al ganar foco (Tab o clic) selecciona todo el valor; ese primer clic no reubica el caret. |
| `placeholder_color(token)`, `on_input(f)`, `on_submit(msg)` | Color del placeholder como token del tema, mensaje por cada edición y mensaje al pulsar Enter. |

- **Teclado.** Flechas, Inicio/Fin, Shift extiende la selección, Backspace/Supr, Cmd+A/C/X/V y Enter emite `on_submit`. IME: el preedit se subraya sin modificar el valor y el commit inserta una sola vez. Los errores del portapapeles se muestran como estado. En password no se puede copiar ni cortar; en read-only no se edita.
- **Accesibilidad.** Ver [08](08-accesibilidad.md): `TextInput` o `PasswordInput` (sin valor), con `label`, `placeholder`, `required`, `read_only` y la descripción del helper. El botón de ojo se publica como "Mostrar contraseña" / "Ocultar contraseña".
- **Fuera de alcance** por ahora: textarea, formateadores chilenos (moneda, RUT, teléfono), tipos numéricos, `title`/tooltips y la transición animada de la etiqueta.

### Iconos

```rust
icon(Icon::Search).class("size-5 text-secondary stroke-[1.5]")
```

- `icon(Icon)` es una hoja del árbol. Mide 1em del tamaño de fuente heredado salvo que se le dé `size-*`, y se pinta con el color de texto heredado (`currentColor`) y el trazo heredado (`stroke-1`, `stroke-2` por defecto como Lucide, `stroke-[2.25]`).
- El catálogo es un enum generado (`src/icons/catalog.rs`) con solo los iconos que usa Kai web: `CircleAlert`, `TriangleAlert`, `CircleCheckBig`, `ChevronDown`, `Eye`, `EyeOff`, `Image`, `Info`, `Minus`, `Plus`, `Search`, `Trash2`, `User`, `X`, `CircleX` y `LoaderCircle`. `Icon::from_name("eye-off")` traduce nombres de Lucide.
- Para agregar un icono: sumar su nombre a la lista al inicio de `scripts/gen-icons.mjs` y ejecutar `node kai-ui-desktop/scripts/gen-icons.mjs` desde la raíz (necesita `npm ci` en `kai-ui-web`). El script lee `lucide-react` de `kai-ui-web/node_modules`, sigue los alias, escribe `assets/icons/<nombre>.svg` (solo los hijos del `<svg>`), regenera el catálogo y copia la licencia ISC de Lucide en `assets/icons/LICENSE`.
- Sin `label`, el icono es decorativo y no aparece en el árbol de accesibilidad. Con `.label(..)` se publica como `Image`.

### Card y otros

- `card()` (`.fs-card`): `bg-background`, borde `border/70`, `rounded-lg`, `shadow-sm`, `overflow-hidden`. `card_header(título, subtítulo)`, `card_content()` y `card_actions()` (separador superior, acciones alineadas a la derecha).
- `stat(etiqueta, valor, pista)`: tarjeta compacta de estadística.
- `heading`, `paragraph` y `caption`: texto con rol y estilos de la tipografía del tema.
