# 31 · Ajustes globales de la demo

Tarea #2: tema, densidad, tamaño de control y fondo elegidos una vez, en la
barra superior, para todas las secciones, y guardados entre sesiones.

## 1. Qué ve el usuario

El botón **Ajustes** (icono de engranaje, `bar.settings`) de la barra
superior abre una franja bajo la barra con cuatro controles segmentados:

| Control | Opciones | Por defecto |
|---|---|---|
| Tema (`settings.theme`) | Inicial, Glass, Glass Dark, AppKit, AppKit Dark, Kai | Inicial |
| Densidad (`settings.density`) | Del tema, Comfortable, Compact | Del tema |
| Tamaño (`settings.size`) | Del tema, Mini, Small, Regular, Large, XL | Del tema |
| Fondo (`settings.background`) | Ninguno, Mesh, Aurora, Glow | Ninguno |

«Inicial» es el tema con que arrancó la demo (el que `main` pasa a
`kai::runtime::run`: Glass, o el de `KAI_THEME`). Con todo en su valor por
defecto la ventana queda exactamente como antes de este cambio.

Los presets conservan los experimentos del tema inicial (`KAI_HOVER`,
`KAI_MIX`, `KAI_LABEL`). Kai arma sus tablas de espaciado según la densidad
elegida (Comfortable si es «Del tema»).

## 2. Cómo lo provee la raíz: hooks del `Layout`

La raíz que genera `kai-build` solo le preguntaba el tema y el fondo a la
página en pantalla. Ahora el trait `kai::router::Layout` tiene dos métodos
opcionales, con implementación por defecto que no opina:

```rust
fn theme(&self, state: &Self::State) -> Option<Theme> { None }
fn background(&self, state: &Self::State) -> Option<BackgroundEffect> { None }
```

Es compatible hacia atrás: los layouts existentes no cambian, y una app sin
layouts genera exactamente el mismo código que antes. Se eligió el layout y
no el `State` porque el `State` es un tipo libre de cada app (exigirle un
trait rompería todas) y porque el layout ya es lo que envuelve a todas las
páginas, tiene `update` con acceso mutable al estado compartido y ahí vive
la barra.

En la demo, `src/layout.rs` lee los ajustes de `State` (`state.settings`) y
los cambia con sus propios mensajes; `src/settings.rs` traduce los ajustes a
`Theme` y `BackgroundEffect`.

## 3. Precedencia

**Tema** (el primero que responda):

1. `Page::theme` de la página en pantalla. El catálogo compara presets, así
   que mientras está abierto manda su propio tema completo (incluidos
   densidad y tamaño); al salir vuelve el de los ajustes.
2. `Layout::theme`, del layout más interno al más externo (solo los que
   enmarcan la ruta actual).
3. El tema que la app pasó a `run` (en la demo, «Inicial»).

**Fondo**:

1. `Page::background` si es visible (`BackgroundEffect::is_visible`). Como
   ese método devuelve `NONE` por defecto, «sin fondo propio» y «Ninguno»
   elegido en la página son lo mismo: en ambos casos se ve el de los
   ajustes. Inicio y el catálogo tienen su propio selector y ganan en
   cuanto eligen un efecto.
2. `Layout::background`, del más interno al más externo.
3. `BackgroundEffect::NONE`.

## 4. `KAI_THEME` y los ajustes guardados

`KAI_THEME` (y los demás `KAI_*`) siguen armando el tema inicial. Si
`KAI_THEME` viene definido y no vacío, **gana sobre el tema guardado**
durante esa ejecución: la franja muestra «Inicial» y la elección guardada
en disco no se toca (`State::env_theme`). Al elegir un tema en la barra se
acaba la anulación y la nueva elección se guarda. Densidad, tamaño y fondo
guardados sí se aplican sobre el tema de `KAI_THEME`.

## 5. Persistencia

`Settings` es un `kai::store::Stored` (`demo.settings.json`, versión 1) en
la carpeta de datos de la app, la misma de la última sección. Se lee al
arrancar (`State::with_store`) y se guarda tras cada cambio con
`Store::save_effect`, fuera del hilo de UI. Un archivo ilegible avisa por
stderr y arranca con los valores de fábrica.

`Store::for_app` ya no exige `HOME`:

| Sistema | Carpeta |
|---|---|
| macOS | `~/Library/Application Support/<id>` |
| Windows | `%APPDATA%\<id>` (o `%LOCALAPPDATA%\<id>`) |
| Otros | `$XDG_DATA_HOME/<id>` o `~/.local/share/<id>` |
| Sin esas variables | `<temp>/<id>` |

Sigue devolviendo `Result` por compatibilidad, pero hoy nunca falla.
`StoreError::NoDataDir` queda para las apps que lo comparan. La lógica es la
misma idea que `kai::crash::dir` (doc 29) pero para datos y no para logs;
se podría unificar más adelante en un helper común de carpetas.

## 6. Pruebas

- `kai-build`: `layouts_back_the_page_theme_and_background_innermost_first`
  y `without_layouts_the_page_alone_picks_theme_and_background`.
- `kai::store`: carpeta de datos por plataforma y sin `HOME`.
- Demo: `src/settings.rs` (unitarias) y `tests/settings.rs` (franja,
  tema en todas las secciones, el catálogo gana, fondo, persistencia entre
  dos arranques con un `Store` temporal, `KAI_THEME`). Todas sin ventana.
- `fixtures/sections-app`: el layout de `customers` (archivo que no estaba
  en edición) responde `theme` y `background`, y `tests/layout_theme.rs`
  comprueba que solo aplica en sus rutas (también bajo el layout con clave
  de `[id]`, que no opina). El tema que pide es el Kai de la app marcado
  con `MixStyle::A`, que Kai solo registra: nada se mueve en pantalla y las
  pruebas existentes no cambian. El layout raíz y el de `[id]` usan los
  métodos por defecto.
