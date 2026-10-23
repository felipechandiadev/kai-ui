# kai-cli

Binario `kai` para el ciclo de desarrollo de apps Kai ([docs/dev/13](../kai-ui-desktop/docs/dev/13-hoja-de-ruta-plataforma.md), Pasos 17–19).

```bash
cargo install --path kai-cli
```

## Comandos

| Comando | Qué hace |
|---|---|
| `kai doctor` | Rust ≥ MSRV 1.95, Xcode CLT (macOS), coherencia de `kai.toml` |
| `kai test [--snapshots] [-- …]` | `cargo test` con `[dev].features`; `--snapshots` añade la feature y tests `#[ignore]` |
| `kai new <nombre> [--template basic\|multiwindow]` | Andamiaje con `kai.toml`, `Cargo.toml`, `src/`, `assets/icons/`, `tests/` |
| `kai icons add <nombre> [--from RUTA]` | Copia SVG a `assets/icons/` desde el catálogo Lucide del crate `kai`, o un archivo (`--from` / `archivo.svg --name`) |
| `kai dev [--bin] [--watch]…` | Compila, abre y reinicia conservando ventana/`App::dev_state`; lee `[dev].bin` / features |

## `kai.toml`

Colócalo junto al `Cargo.toml` de la app. Ejemplo en `kai-ui-desktop/kai.toml`.

```toml
[app]
name = "Mi app"
id = "dev.kai.mi-app"

[theme]
kind = "system"          # kai | appkit | system

[mcp]
enabled = false
expose = "none"

[dev]
bin = "mi-app"
# features = []
```

## Fuera de este corte

- `kai commands` / `kai mcp` (necesitan el registro en runtime)
- `kai bundle` / `sign` / `notarize` (Paso 20)
- Recarga de tema/assets sin reiniciar; parcheo en vivo
