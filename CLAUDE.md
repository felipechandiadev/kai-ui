# kai-ui — notas para Claude

Monorepo: `kai-ui-web` (npm), `kai-ui-desktop` (crate `kai`, Rust/macOS),
`kai-build` (genera la raíz de una app desde carpetas), `kai-cli` (`kai`).
Guías de diseño en `kai-ui-desktop/docs/dev/`; componentes en
`kai-ui-desktop/docs/componentes/`.

## Normas

- **Código en inglés**: tipos, funciones, variables, módulos, carpetas,
  variantes, ids de comando y `test_id`. Solo el texto que ve la persona
  usuaria (etiquetas, mensajes, tooltips) va en español. Respuestas y
  documentación en español.
- Cada componente público tiene su página en `docs/componentes/` (el test
  `components_have_docs` lo exige).
- No hacer commit ni push sin que lo pidan.

## Compilar y verificar (kai-ui-desktop)

- Usar **`cargo +1.95`**: el Rust estable de esta Mac (1.92) no compila
  el proyecto (MSRV 1.95).
- El disco se llena: el perfil `dev` ya va sin incremental y con poca info
  de depuración; si `target/` pasa de ~15 GB, `cargo clean`.
- Antes de dar algo por hecho: **`scripts/check.sh`** (~1 min). Corre fmt,
  clippy `-D warnings` y tests en los tres modos (sin features,
  `document`, `mcp`), `cargo doc`, los dos fixtures, `kai-build` y
  `kai-cli`; muestra una línea por paso y solo el detalle de lo que falla.
  Mientras se itera: `scripts/check.sh quick` (solo fmt, clippy y tests con
  `document`).
- Capturas de la demo: `cargo +1.95 test --manifest-path demo/Cargo.toml
  --target-dir target --test snapshots -- --ignored snapshot_demo` (tema
  Kai) o `snapshots_appkit` (incluye Glass 27); salen en
  `target/snapshots/`. Línea base: `scripts/snapshot-baseline.sh`
  (`--check` para comparar).

## Estado del router (docs/dev/26-router-y-kai-build.md)

R0 nombres en inglés ✅ · R1 `Element::map`/`Effects::map` ✅ ·
R2 `kai::router` (Route, Router, comandos ⌘[ ⌘] ⌘↑, scroll y foco por
ruta) ✅ · R3 `Page`/`Context`/`SectionMeta` ✅ · R4 `kai-build` +
`kai::include_app!()` + `kai new section` ✅ · R5 ✅ la demo es el crate
`demo/` (una carpeta por sección en `demo/src/sections/`, raíz generada
por `kai-build`; `cargo run` o `kai dev` desde `demo/`); `src/demo.rs` ya no
existe (docs/dev/27). El cambio de vista se anuncia con `kai::router::announcer`
(región viva que `kai-build` añade a la raíz); falta oírlo con VoiceOver.

Convenciones de `kai-build`: cada `page.rs` define `pub struct Screen`
(implementa `kai::router::Page`); cada `section.rs`,
`pub const SECTION: SectionMeta`; un parámetro por ruta (`[id]`).
