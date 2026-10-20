# Kai UI Desktop

Prototipo inicial del framework de interfaz de escritorio de Kai en Rust.

El proyecto web existente está en `../kai-ui-web`. Esta carpeta se reserva para la implementación desktop con renderizador propio, comenzando por macOS.

Versión del crate: `Cargo.toml` (sincronizado desde `../version.json`). Historial en [`CHANGELOG.md`](CHANGELOG.md). Release con tag `desktop-vX.Y.Z` — ver [`../docs/versioning.md`](../docs/versioning.md).

La [guía de desarrollo](docs/dev/README.md) contiene la visión arquitectónica y el recorrido paso a paso.

## Estado actual

El prototipo abre una ventana con `winit`, dibuja sobre `wgpu` en Metal y presenta texto con `glyphon`/`cosmic-text`. Las apps siguen la arquitectura Elm: un modelo, mensajes, `update` y una `view` que devuelve un árbol tipo DOM (`Element<Msg>`), reconstruido en cada frame.

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

Las imágenes (`image(src)` con ruta, URL, bytes o RGBA) se decodifican fuera del hilo de UI en un pool de 4 hilos (`Effects::perform`), se dibujan desde un atlas RGBA en GPU con `object-cover`/`object-contain` y radio propio, y toman su aspecto intrínseco. Sobre ellas están `Avatar` (iniciales, icono `User` o imagen recortada al círculo) y `MediaSlot`, el campo de imagen de un formulario (avatar, banner o logo; botón Plus/Camera debajo o en el borde; error de validación). `Effects::pick_file` abre el `NSOpenPanel` y `on_drop_files` recibe archivos arrastrados desde Finder (variante `drag:`).

La demo (`cargo run`) muestra todos los componentes: una toolbar en lugar del header y una sidebar con las secciones Inicio, Campos, Botones, Selección, Contenedores, Badges y alertas, Multimedia, Tabla (con un avatar junto a cada nombre), y Diálogos y avisos. Un test de cobertura falla si un constructor público de `components` no aparece en ella. La demo también expone los comandos `ui.ir_a`, `ui.alternar_sidebar` (Ctrl+Cmd+S), `avisos.mostrar` y `multimedia.cargar_url` a scripts y agentes (`mcp-schema.json`). Hay 394 tests de la biblioteca (398 con `--features mcp`) y 4 doctests, y la baseline de capturas (`scripts/snapshot-baseline.sh`) tiene 15 imágenes: una por sección más la sidebar plegada, el diálogo, los avisos, Multimedia con un archivo arrastrado y la primitiva de imagen.

Escritura normal, acentos, foco con Tab, portapapeles, IME japonés y cambio de pantalla se validaron manualmente en un Apple M4 con escala `2.00x` antes de la migración a la arquitectura Elm. Desde entonces, las capturas offscreen y los tests automáticos cubren el aspecto visual y el comportamiento.

## Requisitos y ejecución

- Rust estable con Cargo.
- macOS y Xcode Command Line Tools.

Desde la raíz del repositorio:

```sh
cargo run --manifest-path kai-ui-desktop/Cargo.toml
```

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
cargo test --manifest-path kai-ui-desktop/Cargo.toml -- --ignored snapshot_demo
```

Si `CARGO_TARGET_DIR` está definido, el binario actualizado está allí y no en `kai-ui-desktop/target/debug/`; usar `cargo run`.

El CI (`.github/workflows/ci.yml`) ejecuta fmt, clippy y tests en macOS, además de un `cargo check` con el MSRV 1.89.

Los iconos se generan desde `lucide-react` con `node kai-ui-desktop/scripts/gen-icons.mjs` (requiere `npm ci` en `kai-ui-web`); los SVG resultantes y su licencia ISC quedan versionados en `assets/icons/`.

Siguiente: más componentes del catálogo (Select, Dialog, Toast), textarea y formateadores del TextField, transiciones y comparación automática de capturas, según la [guía de desarrollo](docs/dev/README.md).
