# Kai UI Desktop

Prototipo inicial del framework de interfaz de escritorio de Kai en Rust.

El proyecto web existente está en `../web`. Esta carpeta se reserva para la implementación desktop con renderizador propio, comenzando por macOS.

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

Los estilos son clases tipo Tailwind (`flex`, `p-4`, `rounded-lg`, `shadow-sm`, `hover:bg-accent`, `focus:ring-2`…) resueltas contra los tokens de `web/src/theme/tokens.css`. El elemento raíz ocupa siempre toda la ventana. Taffy calcula flex, grid y block; cada contenedor con `overflow` tiene su propio scroll y recorte, y `z-index` decide el orden de pintura y de clic. El renderer dibuja cajas redondeadas con borde, sombra y anillo de foco mediante una SDF en GPU, intercaladas con el texto en orden de pintor. Los inputs son controlados y admiten teclado, IME, selección y Cmd+A/C/X/V. La UI se expone a VoiceOver mediante AccessKit.

Escritura normal, acentos, foco con Tab, portapapeles, IME japonés y cambio de pantalla se validaron manualmente en un Apple M4 con escala `2.00x` antes de la migración a la arquitectura Elm. Desde entonces, las capturas offscreen y los tests automáticos cubren el aspecto visual y el comportamiento.

## Requisitos y ejecución

- Rust estable con Cargo.
- macOS y Xcode Command Line Tools.

Desde la raíz del repositorio:

```sh
cargo run --manifest-path desktop/Cargo.toml
```

Validaciones:

```sh
cargo fmt --manifest-path desktop/Cargo.toml --check
cargo check --manifest-path desktop/Cargo.toml
cargo test --manifest-path desktop/Cargo.toml
cargo clippy --manifest-path desktop/Cargo.toml --all-targets -- -D warnings
```

Benchmark de vista + layout + paint (sin caché) y capturas offscreen de la demo (necesita GPU; escribe BMP en `desktop/target/snapshots/`):

```sh
cargo test --manifest-path desktop/Cargo.toml --release -- --ignored --nocapture bench_layout_and_paint
cargo test --manifest-path desktop/Cargo.toml -- --ignored snapshot_demo
```

Si `CARGO_TARGET_DIR` está definido, el binario actualizado está allí y no en `desktop/target/debug/`; usar `cargo run`.

El CI (`.github/workflows/ci.yml`) ejecuta fmt, clippy y tests en macOS, además de un `cargo check` con el MSRV 1.89.

Siguiente: más componentes del catálogo (Select, Dialog, Toast), iconos, transiciones y comparación automática de capturas, según la [guía de desarrollo](docs/dev/README.md).
