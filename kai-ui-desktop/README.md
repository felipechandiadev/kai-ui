# Kai UI Desktop

Framework de interfaz de escritorio de Kai en Rust (crate `kai`), con renderizador propio, por ahora solo para macOS. Es una versión preliminar (0.1.x): la API todavía cambia y el paquete no se publica.

El proyecto web está en `../kai-ui-web` y la CLI (`kai dev`) en `../kai-cli`. La primera app real que lo usa es DSP desktop (`dsp-27/desktop`).

Versión del crate: `Cargo.toml` (sincronizado desde `../version.json`). Historial en [`CHANGELOG.md`](CHANGELOG.md). Release con tag `desktop-vX.Y.Z` — ver [`../docs/versioning.md`](../docs/versioning.md).

La [guía de desarrollo](docs/dev/README.md) contiene la visión arquitectónica y el recorrido paso a paso. El estado del camino hacia una plataforma completa (qué está hecho, qué falta y cuánto costaría) está en una tabla de una página: [hoja de ruta, §1.1](docs/dev/13-hoja-de-ruta-plataforma.md#11-estado-al-20-de-octubre-de-2026).

## Estado actual

Kai abre una ventana con `winit`, dibuja sobre `wgpu` en Metal y presenta texto con `glyphon`/`cosmic-text`. Las apps siguen la arquitectura Elm: un modelo, mensajes, `update` y una `view` que devuelve un árbol tipo DOM (`Element<Msg>`), reconstruido en cada frame.

```rust
fn view(&self) -> Element<Msg> {
    card().children([
        card_header("Contador", None),
        card_content().child(text(format!("{} clics", self.clicks)).class("text-2xl font-semibold")),
        card_actions().child(button("Incrementar", ButtonVariant::Primary).on_click(Msg::Increment)),
    ])
}
```

Los estilos son clases tipo Tailwind (`flex`, `p-4`, `rounded-lg`, `shadow-sm`, `hover:bg-accent`, `focus:ring-2`…) resueltas contra los tokens de `kai-ui-web/src/theme/tokens.css`. El elemento raíz ocupa siempre toda la ventana. Taffy calcula flex, grid y block; cada contenedor con `overflow` tiene su propio scroll y recorte, y `z-index` decide el orden de pintura y de clic. El renderer dibuja cajas redondeadas con borde, sombra y anillo de foco mediante una SDF en GPU, intercaladas con el texto en orden de pintor. Los inputs son controlados y admiten teclado, IME, selección y Cmd+A/C/X/V. El `TextField` cubre el alcance base de la web: variantes normal, contrast y autocomplete, densidad compacta con etiqueta estática o inline, adornos e iconos, password con botón para mostrarla, required, helper text, read-only, disabled y selección al enfocar. Los iconos Lucide se rasterizan desde SVG con `resvg` en un atlas de máscaras en GPU (`icon(Icon::Search).class("size-5 text-secondary")`). La UI se expone a VoiceOver mediante AccessKit.

Además del `TextField`, `components` ofrece botones (Kai y bisel AppKit), cards, group boxes, badges, alertas, stats, switch, checkbox con estado mixto, select, control segmentado, search field, textarea con alto automático, toolbar, sidebar plegable a un riel de iconos con tooltips, `Dialog` modal con foco atrapado, avisos (`toast_stack`) con cierre automático, y una tabla y una lista virtualizadas que solo construyen las filas visibles (probadas con 10 000 filas).

Las imágenes (`image(src)` con ruta, URL, bytes o RGBA) se decodifican fuera del hilo de UI en un pool de 4 hilos (`Effects::perform`), se dibujan desde un atlas RGBA en GPU con `object-cover`/`object-contain` y radio propio, y toman su aspecto intrínseco. Sobre ellas están `Avatar` (el ícono `User` de Lucide o la imagen recortada al círculo) y `MediaSlot`, el campo de imagen de un formulario (avatar, banner o logo; la imagen es el botón: un clic abre el panel y, bajo el puntero, toma el vidrio de elección con una pastilla de cámara; error de validación). `Effects::pick_file` abre el `NSOpenPanel` y `on_drop_files` recibe archivos arrastrados desde Finder (variante `drag:`).

El renderer también dibuja geometría: mallas teseladas con `lyon` (rellenos y trazos de ancho constante en pantalla) con una transformación afín por dibujo, y todos los pipelines usan MSAA 4×. Sobre eso está `kai::maps`: el elemento `map` con marcadores, pan con inercia, zoom anclado (rueda, pellizco, doble clic, teclado y botones) y atribución siempre visible. Dibuja un mapa base vectorial leído sin red de un extracto PMTiles de Protomaps (`assets/maps/parral.pmtiles`, Parral a z0–15), con estilo claro y oscuro desde los tokens y nombres de calles y localidades sin choques. Encima van capas de polígonos GeoJSON (`MapLayer::polygon`) con relleno, borde y nombre, que se resaltan bajo el puntero y entregan su id al hacer clic, y `fit_bounds` encuadra una caja. `polygon_editor` edita un polígono existente. La app guarda el borrador (`PolygonDraft`) en su modelo, y el editor permite arrastrar vértices, agregar uno con el punto medio de un borde y borrarlo con Supr. Las flechas mueven el vértice enfocado, Cmd+Z deshace, los bordes que se cruzan se marcan en rojo y el área se calcula en hectáreas.

La demo (`cargo run`) muestra todos los componentes: una toolbar en lugar del header y una sidebar con las secciones Inicio, Campos, Botones, Selección, Contenedores, Badges y alertas, Multimedia, Tabla (con un avatar junto a cada nombre), Diálogos y avisos, y Mapa (Parral con los 5 sectores de partida de DSP, una lista que los selecciona y encuadra, «Editar sector» con Terminar y Cancelar, y un campo de color, `ColorField`, que cambia el del sector elegido con la paleta de DSP o un hex). Un test de cobertura falla si un constructor público de `components` o de `maps` no aparece en ella. La demo también expone los comandos `ui.ir_a`, `ui.alternar_sidebar` (Ctrl+Cmd+S), `avisos.mostrar`, `multimedia.cargar_url`, `mapa.set_view`, `mapa.get_view`, `mapa.fit_bounds`, `areas.seleccionar` y `areas.editar_geometria` a scripts y agentes (`mcp-schema.json`). El último rechaza como error una geometría inválida: menos de 3 vértices, coordenadas fuera de rango o bordes que se cruzan. Hay 500 tests de la biblioteca (504 con `--features mcp`) y 4 doctests, y la baseline de capturas (`scripts/snapshot-baseline.sh`) tiene 22 imágenes: una por sección más la sidebar plegada, el diálogo, los avisos, Multimedia con un archivo arrastrado y con el puntero sobre una imagen, el mapa con sectores, con uno seleccionado y con uno en edición, los botones ocupados y las primitivas de imagen y de geometría.

Un botón que lanza una tarea con `Effects::perform` queda ocupado hasta que la tarea entrega su `Msg`, y también las que esa respuesta encadene. Conserva su color y su ancho, muestra tres puntos en ola (`DotProgress`) sobre el texto y no recibe clics. Si lo lanzó un comando, el comando queda «En curso» en el menú, la paleta y para agentes. La sección Botones de la demo lo muestra en «Acciones con espera», con el comando `botones.simular_espera`.

Todo lo que es una elección activa (fila de la sidebar, opción del select, segmento, ítem de toolbar presionado, fila de tabla, muestra de color, imagen bajo el puntero) usa el `secondary` de Kai, cyan, como vidrio teñido: el token `choice` y las clases `components::CHOICE_ACTIVE` y `CHOICE_HOVER`, que las apps también usan en sus listas. El azul queda para acción, encendido y foco.

Escritura normal, acentos, foco con Tab, portapapeles, IME japonés y cambio de pantalla se validaron manualmente en un Apple M4 con escala `2.00x` antes de la migración a la arquitectura Elm. Desde entonces, las capturas offscreen y los tests automáticos cubren el aspecto visual y el comportamiento.

## Requisitos y ejecución

- Rust estable con Cargo.
- macOS y Xcode Command Line Tools.

Desde la raíz del repositorio:

```sh
cargo run --manifest-path kai-ui-desktop/Cargo.toml
```

### Recompilar y reiniciar con `kai dev`

`kai dev` recompila la app al guardar un archivo y la reinicia con su estado: la ventana queda en el mismo lugar y del mismo tamaño, y la app conserva lo que devuelva `App::dev_state` (en la demo, la sección, la barra lateral y el sector elegido). Si la compilación falla, los errores salen en la terminal y la versión anterior sigue abierta. Instalación, desde la carpeta que contiene `kai-ui`:

```sh
cargo install --path kai-ui/kai-cli
```

Uso, en la carpeta de la app (aquí, `kai-ui-desktop`):

```sh
kai dev                      # la app del paquete actual
kai dev --bin demo           # si el paquete tiene varios binarios
kai dev --watch ../temas     # una carpeta más que observar
kai dev -- --argumento       # argumentos para la app
```

Observa `src/`, `assets/`, `build.rs` y `Cargo.toml` del paquete y de sus dependencias por ruta, Kai incluido. Ctrl+C termina todo. Solo funciona con compilaciones debug. Ver [13](docs/dev/13-hoja-de-ruta-plataforma.md), Paso 18.

Validaciones:

```sh
cargo fmt --manifest-path kai-ui-desktop/Cargo.toml --check
cargo check --manifest-path kai-ui-desktop/Cargo.toml
cargo test --manifest-path kai-ui-desktop/Cargo.toml
cargo clippy --manifest-path kai-ui-desktop/Cargo.toml --all-targets -- -D warnings
```

Benchmark de vista + layout + paint (sin caché) y capturas offscreen de la demo (necesita GPU; escribe BMP en `kai-ui-desktop/target/snapshots/`):

```sh
cargo test --manifest-path kai-ui-desktop/Cargo.toml --release -- --ignored --nocapture bench_layout_and_paint
cargo test --manifest-path kai-ui-desktop/Cargo.toml --release -- --ignored --nocapture bench_map_pan
cargo test --manifest-path kai-ui-desktop/Cargo.toml -- --ignored snapshot_demo
```

Si `CARGO_TARGET_DIR` está definido, el binario actualizado está allí y no en `kai-ui-desktop/target/debug/`; usar `cargo run`.

Comparar las capturas con la baseline, y regenerarla cuando un cambio visual es intencional (la baseline vive en `snapshots/baseline/`, fuera de git, porque depende de la GPU de cada Mac):

```sh
cd kai-ui-desktop
cargo test --lib -- --ignored snapshot_demo && scripts/snapshot-baseline.sh --check
scripts/snapshot-baseline.sh   # regenera la baseline
```

El CI (`.github/workflows/ci.yml`) ejecuta fmt, clippy y tests en macOS para la librería y para `kai-cli`, además de un `cargo check` con el MSRV 1.89.

Los iconos se generan desde `lucide-react` con `node kai-ui-desktop/scripts/gen-icons.mjs` (requiere `npm ci` en `kai-ui-web`); los SVG resultantes y su licencia ISC quedan versionados en `assets/icons/`, y el catálogo generado en `src/icons/`, así que compilar no necesita `node_modules`.

## Limitaciones conocidas

- **Panel de archivos:** `Effects::pick_file` abre el `NSOpenPanel` como modal dentro del manejador de eventos. Si la app se termina con el panel abierto (Cmd+Q, o `kai dev` que la reinicia), el proceso aborta. El arreglo propuesto (hoja no bloqueante) está en el Paso 15 de la [hoja de ruta](docs/dev/13-hoja-de-ruta-plataforma.md).
- **Faltan** persistencia (`Store`), `kai.toml`, el resto de la CLI (`kai new`, `kai test`…), bundle y firma, router, varias ventanas y Windows/Linux. El orden y el costo están en la [hoja de ruta, §1.2](docs/dev/13-hoja-de-ruta-plataforma.md#12-lo-que-falta-para-cada-hito-con-costo-estimado).
