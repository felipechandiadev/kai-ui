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
- [x] Después de L6 (Paso 14 de [13](13-hoja-de-ruta-plataforma.md)): checkbox, segmented, `Select`, sidebar, toolbar, search field, textarea, `Dialog`, toasts, tabla y lista virtualizadas, `Avatar`, `MediaSlot`, `ColorField` y `Tabs` (equivalente a `NSTabView`). Su detalle está en [12](12-modelo-appkit.md) (Paso 20) y en el Paso 14 de 13, junto con el lenguaje de color de la selección (token `choice`).
- [x] `DotProgress` (puntos en ola) y botón ocupado automático mientras corre la tarea que lanzó (abajo).
- [ ] AutoComplete y formateadores del TextField.
- [ ] Transiciones y animaciones: parcial. Hay animación calculada al pintar (`Motion::Wave`, la usa `DotProgress`); faltan transiciones de propiedades (color, opacidad, tamaño) y keyframes generales.

## Clases tipo Tailwind (`tw.rs`)

`ClassList::parse(clases, &tema)` convierte una cadena de clases en declaraciones por variante; `resolve(State)` aplica, en orden de prioridad CSS, base → `group-hover:` → `group-focus-within:` → `group-busy:` → `hover:` → `focus-within:` → `focus:` → `active:` → `busy:` → `disabled:`. Con `disabled` o `busy`, `hover:` y `active:` se ignoran. `busy:` aplica a un elemento ocupado (`Element::busy`, que la sesión pone mientras corre la tarea que lanzó), y `group-busy:` a los descendientes del `group` ocupado más cercano. Las clases desconocidas se reportan (`ClassList::unknown`) en lugar de fallar en silencio.

| Grupo | Clases |
|---|---|
| Display y flex | `block` `flex` `grid` `hidden` `inline-flex` `inline-block` `flex-row` `flex-col` `flex-wrap` `flex-1` `flex-auto` `flex-none` `grow` `shrink-0` `items-*` `self-*` `justify-*` |
| Espaciado | `gap-*` `gap-x-*` `gap-y-*` `p*-*` `m*-*` (negativos y `mx-auto`) `inset-*` `top/right/bottom/left-*` |
| Tamaño | `w-*` `h-*` `size-*` `min-w/h-*` `max-w/h-*` (`max-w-xs`…`7xl`) fracciones `full` `screen` `h-control` `h-control-compact` `min-h-control` `py-control` |
| Posición y overflow | `relative` `absolute` `sticky` `z-N` `overflow-visible/hidden/clip/auto/scroll` y por eje `overflow-x-*` `overflow-y-*` `grid-cols-N` |
| Color | `bg-*` `text-*` `border-*` `ring-*` `placeholder-*` con nombre del tema (incluye `field-muted`), alfa `/N` o `[#hex]`; `bg-current` (fondo del color del texto, como `currentColor`) |
| Borde y radio | `border` `border-N` `border-t/b/l/r/x/y` `rounded-*` (`sm` 2, `md` 6, `lg` 8, `xl` 12, `2xl`, `3xl`, `full`) |
| Efectos | `shadow-sm/md/lg/xl/none` `ring` `ring-N` `ring-offset-N` `outline-none` `opacity-N` |
| Texto | `text-xs`…`3xl` `text-[Npx]` `font-normal/medium/semibold/bold` `leading-*` `whitespace-nowrap` `truncate` |
| Iconos | `stroke-N` `stroke-[N]` (ancho de trazo heredable) |
| Interacción | `cursor-default/pointer/text/not-allowed/wait` `pointer-events-none/auto` `group` |

- El espaciado usa la unidad del tema (`--space-4 / 4`), así que `p-4` es `--space-4` y escala con la densidad; los valores arbitrarios (`w-[180px]`) no escalan.
- Overflow por eje: `overflow-x-auto` desplaza solo a lo ancho, `overflow-y-auto` solo hacia abajo y `overflow-auto` en los dos. Como en CSS, si un eje no es visible el otro deja de serlo. La rueda mueve el contenedor más interno bajo el puntero que todavía puede moverse en ese eje; Mayús+rueda desplaza a lo ancho. La barra horizontal se dibuja abajo.
- `sticky` con `left-*` o `right-*` deja el elemento a esa distancia del borde del contenedor más cercano con scroll a lo ancho (`overflow-x-auto`), sin salir de su padre, y lo pinta y le da los clics por encima de sus hermanos. A diferencia de CSS, un contenedor intermedio que solo desplaza hacia abajo (el cuerpo de una tabla) no lo detiene. Solo el eje horizontal. Al revelar un elemento (foco o clic), sus hermanos `sticky` y los de sus ancestros tapan el borde al que se pegan, como `scroll-padding` en CSS, y un elemento dentro de un `sticky` no desplaza a lo ancho.
- `text-lg` y `text-xl` usan los tamaños de título de página y sección del tema; `leading-*` siempre gana al interlineado implícito del tamaño, sin importar el orden.
- Las propiedades de texto se heredan como en CSS (color, tamaño, interlineado, peso, `nowrap` y el trazo de los iconos).
- Variantes del tema, que se evalúan al parsear: `kai:` `appkit:` `glass:` `mix-a/b/c:` `label-*:` `dark:` `light:` y las preferencias del Mac (`Theme.prefs`, `SystemPrefs`): `motion-reduce:` (reducir movimiento), `contrast-more:` (aumentar contraste) y `transparency-reduce:` (reducir transparencia). El runtime lee esas preferencias de macOS al arrancar y cada vez que la ventana se activa, y las superpone al tema de la app (`Session::set_system_prefs`; en pruebas, `TestApp::set_system_prefs`). Con `contrast-more:` los bordes de campos, botones push, checkbox, segmented y botones de icono con contorno pasan a un borde sólido (`border-foreground` en Kai, `border-secondary-label` en AppKit). `full_keyboard_access` también viene del sistema en AppKit; Kai lo mantiene encendido, como en la web.

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
- **Ocupado.** Mientras corre una tarea que lanzó (`Effects::perform`, ver [03](03-eventos-y-estado.md)), el botón queda `busy`. Conserva su color y su ancho, no toma hover, muestra el cursor de espera (`busy:cursor-wait`) y no recibe clics. El texto queda invisible (`group-busy:opacity-0`) debajo de tres puntos `DotProgress` Sm en el color del texto, que van en una capa `absolute inset-0 hidden group-busy:flex` con `pointer-events-none`.
  - `Button::loading(true)` (o `Element::busy(true)`) lo deja ocupado por una razón que la sesión no ve, como esperar un socket.
  - `Button::busy_feedback(false)` (o `Element::busy_feedback(false)`) lo deja usable mientras su tarea corre, por ejemplo para una sincronización de fondo.
  - AccessKit lo marca `busy` y `disabled`, con la descripción «Cargando» si no tiene otra. El nombre sigue siendo su texto.
  - `icon_button` y `toolbar_item` también quedan bloqueados y con cursor de espera, pero sin puntos.

### DotProgress

```rust
DotProgress::new().size(DotSize::Lg).label("Subiendo").build()
dot_progress()                                   // 3 puntos Md, «Cargando»
```

- Puntos (3 por defecto, `count(n)`) `rounded-full bg-current` que saltan en ola, como `DotProgress` `variant="wave"` de la web: ciclo de 1,05 s, 0,16 s entre puntos, salto del 70 % del diámetro, arriba al 25 % del ciclo y abajo al 55 %. Solo existe la variante `wave`.
- Tamaños: `Sm` (6 pt, gap 3; el del botón ocupado), `Md` (10, gap 5) y `Lg` (16, gap 8, el de la web). La raíz suma `pt` igual al salto, así la ola no se recorta.
- El color es el del texto heredado (`text-primary` en la raíz o en un ancestro).
- La animación es un `Motion` que se calcula al pintar ([04](04-renderer-y-texto.md)): layout y hit-test no cambian. Con `reduce_motion` los puntos quedan quietos y la ventana no pide frames.
- Accesibilidad: `Role::ProgressIndicator` (indeterminado) con nombre «Cargando» o el de `label(..)`. `decorative()` lo deja sin rol ni nombre, como dentro del botón.
- Partes: `<id>.dot.<i>`.

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

### Table y virtual_list

```rust
fn columnas() -> Vec<Column<Cliente, Msg>> {
    vec![
        Column::number("N.º", |c: &Cliente| f64::from(c.numero)).fixed(72.0).sortable(),
        Column::text("Nombre", |c: &Cliente| c.nombre.clone())
            .render(|c: &Cliente| avatar_y_nombre(c)) // componente que conserva el valor
            .flex(2.0)
            .min_width(160.0)
            .sortable()
            .filterable(),
        Column::number("Monto", |c: &Cliente| f64::from(c.monto))
            .format(|monto| format!("$ {monto}"))
            .fixed(112.0)
            .sortable()
            .filterable()
            .hideable(),
        Column::actions(|c: &Cliente| ver(c)).fixed(64.0), // fija al final
    ]
}

// update: Msg::Tabla(cambio) => { vista.apply(cambio); vista.refresh(&clientes, &columnas()) }
Table::new(Rc::clone(&clientes), columnas(), &vista)
    .on_view(Msg::Tabla)
    .selected(model.cliente) // índice en `clientes`
    .on_select(Msg::SelectRow)
    .expandable(detalle_cliente, 64.0)
    .label("Clientes")
    .test_id("clientes")
    .build()
    .class("h-[360px]")
```

Tabla virtualizada, equivalente a `NSTableView` ([12](12-modelo-appkit.md), Paso 20). Es un componente especial: con muchos registros o datos de una API, leer [15](15-tablas-con-muchos-registros.md).

- **Virtualización.** Solo se construyen las filas visibles, más 6 por lado (`virtual_rows.rs`); el resto son espaciadores con su altura. 10 000 filas cuestan lo que una pantalla. La fila seleccionada y la que recibe el foco se construyen siempre.
- **Datos y columnas.** La tabla recibe los registros (`Rc<[T]>`) y columnas con un valor tipado por registro: `Column::text`, `number`, `date` y `boolean` (`Value::{Text, Number, Date, Bool}`). `format` decide cómo se lee el valor y `render` lo muestra con un componente (un avatar, un badge), construido solo para las filas visibles; la celda conserva el valor como nombre accesible y para buscar, filtrar, ordenar y exportar. `Column::actions` no tiene valor: nunca se ordena, filtra, busca, oculta ni exporta, y su encabezado se lee «Acciones». Ancho: `fixed(pt)` o `flex(parte)` con `min_width(pt)` (80 por defecto).
- **`TableView`** (pura, sin `Msg`): guarda la búsqueda global (`query`), los filtros por columna, el modo filtro, el orden, las columnas ocultas, las filas expandidas y el resultado (`rows()`, índices de los registros en orden de pantalla). `apply(ViewChange)` aplica un cambio y `refresh(&datos, &columnas)` recalcula: búsqueda en las columnas `searchable` (por defecto las de texto, también las ocultas) y cada filtro, luego orden estable. El texto se compara sin mayúsculas ni acentos; un número acepta `=`, `>`, `<` y `a..b`; una fecha, un prefijo como `2026-10`; un booleano, «sí» o «no».
- **Cambios** (`on_view`, `ViewChange`): `Sort` (clic en un encabezado `sortable`; invierte si ya era la ordenada y muestra una flecha), `Filtering` y `Filter` (con el modo filtro, cada columna `filterable` tiene un campo «Filtrar <título>» bajo el encabezado, un `search_field` `Inline` y `Small` de 28 pt en Kai, su propia parada de Tab, y Esc lo borra; apagar el modo borra los filtros), `Hide` y `Expand`. La app decide cuándo refrescar: la demo espera una pausa al tipear un filtro (`Effects::after(..).keyed("filtro")`).
- **Ocultar columnas.** Un clic derecho (o Ctrl+clic, o la acción `ShowContextMenu` de AccessKit) en el encabezado abre un menú con un check por columna `hideable`, que queda abierto al elegir. Nunca se ocultan todas. `column_menu(botón, &columnas, &vista, Msg::Tabla)` da el mismo menú a un botón «Columnas».
- **Exportar.** `vista.export(&datos, &columnas)` devuelve una `Sheet` con lo que se ve: títulos y valores tipados de las columnas visibles (sin acciones ni ocultas), en el orden del resultado de búsqueda, filtros y orden. `xlsx::write(&sheet, hoja, ruta)` la guarda como Excel (`rust_xlsxwriter`) con la fila de títulos fija, autofiltro, y números y fechas con su tipo. La ruta la pide `Effects::save_file` ([03](03-eventos-y-estado.md)).
- **Filas expandibles.** `expandable(|registro| detalle, alto)` suma una columna de chevrons al inicio: el chevron, → y ← sobre la fila abren y cierran un panel de ese alto debajo de ella. Varias a la vez. El panel es una `Region` con el nombre de la fila y sus controles son paradas de Tab propias. La fila publica `expanded`. No se exporta.
- **Scroll horizontal.** Si las columnas no caben con su ancho mínimo, encabezado, filtros y filas se desplazan juntos a lo ancho (trackpad, Mayús+rueda, o el foco que llega a una celda fuera de la vista).
- **Columnas fijas.** `pin(Pin::Start | Pin::End)` deja una columna al inicio o al final mientras el resto se desplaza (`sticky`); una columna flex fija mide su `min_width`. `Column::actions` queda fija al final salvo `unpin()`; los chevrons, al inicio. Encabezado, filtros y filas usan las mismas: las celdas fijas son opacas con el tinte de su fila, y la fija más interna de cada lado dibuja el borde. El foco y los clics revelan una celda sin dejarla bajo una columna fija.
- **Selección.** Con `on_select` las filas son una sola parada de Tab; un clic, ↑/↓, Inicio/Fin y RePág/AvPág eligen una. Una sola fila a la vez. `selected` y `on_select` usan el índice del registro, así la selección sobrevive al ordenar y filtrar.
- **Aspecto.** Filas de 32 pt por defecto (`row_height`; 24 es la de AppKit); en AppKit alternan el fondo. La fila elegida lleva el tinte de elección (`bg-choice/20`).
- **Accesibilidad.** `Role::Table` con `row_count` (el total), encabezados `ColumnHeader` con su orden, y filas `Row` de `Cell` con su `row_index` real; el encabezado publica `ShowContextMenu`, el menú de columnas es de `MenuItemCheckBox` y los paneles son `Region`.
- **Partes:** `<id>.scroller`, `<id>.header`, `<id>.column.<i>`, `<id>.filters`, `<id>.filter.<i>`, `<id>.body`, `<id>.row.<r>`, `<id>.row.<r>.toggle` y `<id>.detail.<r>` (`i` es el índice de la columna y `r` el de pantalla).
- **Límites:** sin selección múltiple, sin edición en la celda y sin aviso a la app de qué filas se ven (`on_rows_needed`, necesario para cargar páginas de una API; ver [15](15-tablas-con-muchos-registros.md)). La barra vertical del cuerpo queda en el borde derecho de la grilla, así que se desplaza con el scroll horizontal. Una grilla editable (DataGrid) es un componente aparte, por planificar.
- **`virtual_list(count, row_height, selected, render)`**: la misma virtualización para listas sin columnas; las filas sin rol son `ListItem` dentro de una `List`. Con `Element::selecting_focus_group` es una sola parada de Tab cuyas flechas eligen.

### Botones con comando y paleta

- **`.on_command(Accion)`** conecta un control a una acción registrada (Paso 8 de [13](13-hoja-de-ruta-plataforma.md)). Pone su `Msg` como `on_click` y, en cada `view`, aplica la disponibilidad: si la acción está `Disabled` o `Hidden` el control se deshabilita y el motivo es su descripción accesible. Su `test_id` por defecto es el id del comando y su nombre, el título, si no tiene texto propio.

```rust
button("Reiniciar", ButtonVariant::Outlined).on_command(Reiniciar)
// test_id "contador.reiniciar"; deshabilitado con 0 clics, con el motivo
```

- **Partes**: un elemento que es parte de un componente (`.part(..)`) conserva su sub-id; `on_command` solo le aplica el estado. Por eso **`TextField::password_command(revelada, Accion)`**, la variante de `password` con comando, deja el ojo como `x.toggle` y lo deshabilita junto con la acción.

```rust
TextField::new("Contraseña", &model.password)
    .test_id("form.password")
    .password_command(model.show_password, AlternarContrasena)
    .build()
// el ojo sigue siendo "form.password.toggle"
```

- **`command_palette(items, query, selected)`** dibuja la paleta: fondo `absolute inset-0 z-50`, panel `Role::Dialog` «Paleta de comandos», búsqueda `kai.paleta.buscar` y una lista (`Role::List` «Comandos») con una fila `Role::Row` por `PaletteItem` (`kai.paleta.<id>`): título, motivo si está deshabilitada y atajo (`KeyEquivalent::label`); la elegida está en `On`. El foco se queda en la búsqueda y la sesión publica la fila elegida como su `active_descendant` (`A11ySnapshot::active_descendant`), así VoiceOver lee la fila que mueven las flechas. La muestra la sesión con ⇧⌘P; los elementos no llevan `Msg`. Comparte el layout entre temas y elige colores y tipografía con `themed`.

### Ids de prueba y partes

Un componente con `.test_id("x")` nombra solo sus partes internas, así un test las encuentra sin conocer su estructura (convención de [13 §6.4](13-hoja-de-ruta-plataforma.md#64-convención-de-sub-ids-en-componentes)):

| Componente | Partes |
|---|---|
| `TextField` | `x` es la raíz (delega el foco); `x.input`, `x.label` (etiqueta estática), `x.floating` (etiqueta sobre el borde), `x.shell` (caja con borde), `x.placeholder` (placeholder de dos colores de un campo requerido vacío), `x.helper`, `x.start`, `x.end` y `x.toggle` (ojo de contraseña). Una parte que el campo no dibuja no existe. |
| `card()` | `x.header`, `x.title`, `x.subtitle`, `x.content` y `x.actions`, de `card_header`, `card_content` y `card_actions`. |
| `group_box` | `x.label` (título AppKit, fuera de la caja), `x.box`, y `x.header`, `x.title` y `x.content` de la card interior. |
| `stat` | `x.label`, `x.value` y `x.hint`. |
| `alert` | `x.icon` y `x.message`. |
| `switch_labeled` | `x.track` (la pista, `Role::Switch`) y `x.label`. Un `switch(..)` sin label es la pista. |
| `Tabs` | `x.tab.<i>` (cada pestaña, `Role::Tab`) y `x.panel` (el panel de la elegida, `Role::TabPanel`). |
| `button`, `icon_button`, `badge` | Solo `x`. |

```rust
card().test_id("form").children([card_header("Formulario", None), card_content()]);
// form.header, form.title, form.content

let app = TestApp::new(Demo::default());
app.screen().get_by_test_id("form.password.toggle");
```

- **Componentes propios.** `.part("nombre")` marca una parte y `.parts_root()` marca la raíz del componente. `assign_ids` le da a cada parte `<id del ancestro más cercano con test_id propio>.<nombre>`: las partes anidadas no se encadenan (`x.title`, no `x.header.title`). Si la raíz no tiene id, sus partes no toman el de un ancestro; así dos campos sin id dentro de `card().test_id("form")` no chocan. Un `test_id` explícito siempre gana sobre la parte.
- **`TestApp`.** `value`, `is_focused` e `is_enabled` sobre la raíz de un `TextField` actúan sobre su input, así que `By::test_id("form.nombre")` sirve igual que `By::test_id("form.nombre.input")`.
- **Sin layout.** `Element::find_test_id("x.title")` busca en el árbol después de `assign_ids`, para tests de componentes que no necesitan `Screen`.
