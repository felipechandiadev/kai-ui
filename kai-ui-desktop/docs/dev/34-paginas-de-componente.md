# Páginas de componente: `/components` con controles en vivo

Estado: **implementado** (parche 09). Código: `demo/src/showcase.rs`
(registro y tipos), `demo/src/showcase/*.rs` (un descriptor por
componente, por grupo), `demo/src/sections/components/` (índice y ficha) y
`Effects::copy_text` en `src/runtime.rs` + `src/session.rs`. Pruebas:
`demo/tests/components.rs`, `showcase::tests` y `tests/copy_text.rs`.

## 1. Para qué

`docs/componentes/` describe cada componente con texto y ejemplos fijos;
el [catálogo vivo](25-catalogo-vivo.md) muestra unos pocos controles bajo
tema × densidad × tamaño. Faltaba una página **por componente** donde
cambiar sus props y ver al mismo tiempo la instancia, sus variantes y el
código que la escribe, listo para copiar. Eso son las fichas de
componente.

## 2. Rutas

| Carpeta | Ruta | Qué muestra |
|---|---|---|
| `components/section.rs` | — | Sección «Fichas de componentes», grupo «Catálogo», ícono `Library` |
| `components/page.rs` | `/components` | Índice por grupo (orden de `docs/componentes/README.md`) con filtro |
| `components/[id]/page.rs` | `/components/:id` | La ficha del componente `id` |

El `id` es el nombre del archivo en `docs/componentes/` sin `.md`
(`/components/button`, `/components/date-field`, `/components/tipografia`).
Un `id` que no está en el registro muestra un `EmptyState`
(`components.missing`) con «Ver todos los componentes».

## 3. La ficha

De arriba abajo (cada parte tiene `test_id`):

| Parte | `test_id` | Contenido |
|---|---|---|
| Encabezado | `components.header` | Nombre, resumen, migas y «Todos los componentes» (`components.index`) |
| Vista previa | `components.preview` | Instancia viva (`components.live`) + último evento (`components.event`, «on_click ×2») |
| Código | `components.snippet` | Bloque `components.code` (una línea de texto por línea de código), «Copiar» (`components.copy`) y aviso «Copiado» (`components.copied`, 2 s) |
| Matriz | `components.matrix` | Una fila por eje (Variantes, Tamaños, Estados…); celda `components.matrix.<eje>.<variante>` |
| Props | `components.props` | Un control por prop (`components.prop.<key>`) y «Restablecer» (`components.reset`) |
| Notas | `components.notes` | Teclado (`components.keyboard`), accesibilidad (`components.a11y`), archivo de docs (`components.doc`) y «Ver en la demo: …» (`components.demo`) |

Controles del panel según `Control`:

- `Toggle(bool)` → `switch_labeled` compacto.
- `Choice(opciones, inicial)` → `segmented` si son hasta cuatro opciones de
  hasta nueve caracteres; si no, `select`.
- `Text(inicial)` → `TextField` compacto.

La vista previa también escribe en las props: lo que se tipea en el campo
vivo, la casilla que se marca o el interruptor que se mueve llegan como
`PreviewMsg::Set` y el panel y el código lo reflejan. Los eventos sin
estado (`on_click`, `on_pick`…) llegan como `PreviewMsg::Event` y se anotan
en `components.event`. Las celdas de la matriz son instancias reales pero
inertes: sus mensajes no cambian nada.

`Dialog` se abre desde la vista previa y se dibuja sobre toda la ventana
(`Page::overlay`); `Toast` usa `Context::toast` como cualquier página.

### Estado por ficha

kai-build reinicia una subpágina cada vez que se abre, así que las props
cambiadas se guardan en el estado global, `State::components`
(`showcase::Memory`, un mapa `id → Props`). Volver a una ficha la muestra
como se dejó; «Restablecer» vuelve a los valores iniciales y borra la
entrada. El estado vivo que no es prop (búsqueda del autocompletar, mes del
calendario, vista de la tabla, último evento) es de la página y se pierde
al salir.

## 4. El registro

`showcase::COMPONENTS` tiene **una entrada por componente documentado**
(35, una por archivo de `docs/componentes/` salvo `README.md` y
`elementos-comunes.md`); una prueba lo verifica. Cada entrada es un
`Component`:

```rust
pub const BADGE: Component = Component {
    id: "badge",                 // ruta y archivo de docs
    name: "Badge",
    group: Group::Content,       // grupo del índice
    summary: "…",
    doc: "badge.md",
    props: &[
        text("label", "Texto", "Activo"),
        choice("color", "Color", BADGES, 2), // opciones e índice inicial
        toggle("outlined", "Contorno", false),
    ],
    matrix: &[/* Axis { title, variants: &[Variant { label, set }] } */],
    preview: |props, cx| { /* Element<PreviewMsg> con test_id cx.id */ },
    code: |props| { /* String: el código de esas props */ },
    overlay: None,
    keyboard: &["…"],
    accessibility: &["…"],
    demo: Some(("Badges y alertas", "/badges")),
};
```

- `preview` recibe las props actuales (o las de una celda: las actuales
  con el `set` de la variante encima) y un `Cx` con el estado vivo, el
  `test_id` de la instancia (`cx.part("track")` para sus partes) y si es la
  instancia principal (solo ella abre paneles flotantes).
- `code` arma el código con `showcase::code` (`snippet`, `chain`, `lit`,
  `when`, `call`, `option`); los literales salen escapados como Rust.
- Las notas de teclado y accesibilidad se escribieron revisando el código
  de cada componente (rol, nombre accesible, teclas); si un componente
  cambia, hay que actualizarlas.
- `..REFERENCE` llena lo que falta. Sin props, la ficha es **de
  referencia**: muestra el uso, las notas y el enlace a su sección, sin
  panel ni matriz. Hoy lo son `command-palette`, `map` y `document`.

### Agregar un componente

1. Escribir su página en `docs/componentes/<id>.md` y sumarla al README.
2. Agregar `pub const <NOMBRE>: Component` en el archivo de su grupo en
   `demo/src/showcase/`.
3. Sumarlo a `COMPONENTS` en el orden del README.
4. `cargo test -p kai-demo --test components`: las pruebas recorren el
   registro, así que la ficha nueva ya queda probada (navegación, partes,
   copiar).

## 5. `Effects::copy_text`

API nueva de la biblioteca para «Copiar»:

```rust
Msg::Copiar => Effects::copy_text(self.codigo.clone()),
```

- Pone el texto en el portapapeles del sistema como texto plano (el mismo
  `ClipboardPort` que usa ⌘C en los campos). No devuelve mensaje: la app
  muestra su propia confirmación (la ficha usa `Effects::after` con clave
  `components.copied` para volver de «Copiado» a «Copiar»).
- Si el portapapeles falla, la sesión escribe el error en stderr.
- Se puede `map`/`keyed` como cualquier efecto; `Debug` lo muestra como
  `copy_text("…")`.
- En pruebas: `TestApp::builder().clipboard(MemoryClipboard::default())` y
  `app.clipboard().text()`.

## 6. Pruebas

`demo/tests/components.rs` (15 pruebas, sin ventana):

- Hay una ficha por cada `.md` de `docs/componentes/`, el índice las lista
  todas y cada fila navega a `/components/<id>`; el filtro acota la lista y
  avisa cuando no queda nada.
- Cada ficha dibuja encabezado, vista previa, código, notas y, si es viva,
  props y matriz.
- Cambiar un interruptor, un segmentado, un `select` o un texto cambia la
  instancia y la línea de código correspondiente; la vista previa escribe
  de vuelta en el panel.
- «Copiar» deja en el portapapeles exactamente las líneas del bloque de
  código, el botón dice «Copiado» y vuelve a «Copiar» después de
  `COPIED_FOR`.
- Las props se conservan al volver a la ficha y «Restablecer» las borra.
- El diálogo se abre sobre la ventana con sus props y se cierra.
- Un `id` desconocido muestra `components.missing`.

`tests/copy_text.rs` cubre el efecto en la biblioteca (portapapeles,
`map`, `Debug`).

## 7. Límites

- `date-field` usa la API de `date_field` tal como está; si cambia, hay
  que ajustar su descriptor (`showcase/inputs.rs`).
- `Map`, `Document` y `CommandPalette` son de referencia: son grandes o
  dependen de la sesión; se ven en `/map`, `/document` y con ⌘⇧P.
- La matriz no muestra combinaciones (tema × tamaño): eso sigue en el
  [catálogo vivo](25-catalogo-vivo.md).
- El código es un fragmento para pegar dentro de `view`/`update`: usa
  nombres de ejemplo para el estado y los mensajes de la app
  (`self.avisos`, `Msg::Guardar`), como los ejemplos de
  `docs/componentes/`, y no compila solo.
- No hay snapshots visuales de las fichas todavía.
