# Kai UI Desktop

Prototipo inicial del framework de interfaz de escritorio de Kai en Rust.

El proyecto web existente está en `../web`. Esta carpeta se reserva para la implementación desktop con renderizador propio, comenzando por macOS.

La [guía de desarrollo](docs/dev/README.md) contiene la visión arquitectónica y el recorrido paso a paso.

## Estado actual

El prototipo abre una ventana con `winit`, crea una superficie `wgpu` sobre Metal, dibuja geometría 2D en unidades lógicas, presenta texto con `glyphon` y ofrece un botón interactivo. El campo editable recibe texto de teclado e IME, y tiene cursor, selección, navegación, borrado y Cmd+A/C/X/V mediante el portapapeles. El usuario confirmó que la escritura normal y el recorrido de foco con Tab funcionan. En este equipo se verificó el adaptador Apple M4 y escala `2.00x`; composición IME, portapapeles y detalles visuales Retina siguen requiriendo prueba manual.

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

Siguiente: validar IME/dead keys, operaciones de portapapeles y alineación/legibilidad en Retina; luego cerrar la revisión del prototipo y empezar a separar layout, eventos y renderer en módulos estables.
