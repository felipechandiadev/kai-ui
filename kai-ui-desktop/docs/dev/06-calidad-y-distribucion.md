# 06 — Calidad y distribución para macOS

## Objetivo

Asegurar que el framework pueda usarse en aplicaciones desktop reales, no solo en un ejemplo local.

## Calidad técnica

Ejecutar los gates en cada fase L0–L6; ningún resultado manual pendiente se marca como aprobado por inferencia.

1. Baseline por incremento: `cargo fmt --manifest-path kai-ui-desktop/Cargo.toml --check`, `cargo check --manifest-path kai-ui-desktop/Cargo.toml`, `cargo test --manifest-path kai-ui-desktop/Cargo.toml` y `cargo clippy --manifest-path kai-ui-desktop/Cargo.toml --all-targets -- -D warnings`.
2. **L0:** tests puros de geometría/unidades/conversiones; comprobar hit-test y dibujo con los mismos bounds.
3. **L1–L3:** tests de transiciones, invalidación, traducción de eventos y despacho de comandos sin ventana.
4. **L4:** resultados de layout deterministas y equivalencia de caché contra cálculo completo; benchmark antes de alegar mejoras.
5. **L5–L6:** tests de paint/tree y pruebas integradas; añadir capturas visuales cuando la ruta sea estable.
6. Validar manualmente HiDPI, resize, foco, suspensión/reanudación, cierre, IME, Unicode y portapapeles en macOS; registrar específicamente qué se probó y el resultado.
7. Medir inicio, frame time, CPU/GPU, fuentes y memoria antes de optimizar; documentar hardware, macOS y método.
8. Auditar VoiceOver/roles/etiquetas/foco/acciones y revisar dependencias, licencias y comportamiento sin GPU antes del hito de distribución.

## Estado actual

- [x] CI: el job `desktop` de `.github/workflows/ci.yml` corre en `macos-latest` con Rust estable: `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` y `cargo test --locked`. Los tests son puros (sin ventana, superficie GPU ni portapapeles del sistema), así que funcionan en el runner.
- [x] CI: el job `desktop-msrv` comprueba `cargo check --locked --all-targets` con Rust 1.89, el MSRV declarado (impuesto por `cosmic-text` 0.19).
- [x] CI: el job `kai-cli` corre fmt, clippy y tests del crate de la CLI.
- [x] 500 tests normales (504 con `--features mcp`) cubren las capas base (geometría, señales, dispatch, layout, paint, backend, texto, AccessKit), los componentes, comandos, MCP, efectos, imágenes, mapas y la demo, casi todos con `TestApp` y sin ventana ni GPU. Más 4 doctests y 8 tests de `kai-cli`.
- [x] 14 tests `#[ignore]` que se ejecutan a mano porque necesitan la GPU local o son lentos: benchmarks (layout y paint, abajo; scroll de la tabla; pan del mapa), capturas (`snapshot_demo`, galería, AppKit, tabla, sidebar, toasts…) y pruebas de dibujo (mezcla en gamma, imágenes, geometría). Las capturas quedan como BMP en `kai-ui-desktop/target/snapshots/` ([04](04-renderer-y-texto.md#capturas)).
- [x] Comparación de capturas contra la línea base: `scripts/snapshot-baseline.sh --check` compara con `snapshots/baseline/SHA256SUMS`; `snapshot::diff` con tolerancia para revisar diferencias.
- [ ] Comparación automática de capturas en CI: requiere un runner con GPU.
- [ ] Auditoría VoiceOver completa, bundle `.app`, firma y notarización.

### Benchmark de vista + layout + paint

```sh
cargo test --manifest-path kai-ui-desktop/Cargo.toml --release -- --ignored --nocapture bench_layout_and_paint
```

Ejecuta 400 iteraciones de `view` + `Layout::compute` + `paint` sobre la demo, con medida real de cosmic-text y alternando anchos (1024, 800, 600 y 420 pt) para forzar el re-envolvido del texto. Referencia (Apple M4, macOS 15, release, 18-10-2026): media ≈ 196 µs, p50 ≈ 114 µs, p95 ≈ 132 µs, 142 operaciones de dibujo por frame; el máximo (≈ 32 ms) es la primera iteración, en frío. Con este árbol un caché no se justifica; repetir la medición cuando el árbol crezca y antes de proponer invalidación incremental.

### Nota sobre el binario

Si `CARGO_TARGET_DIR` está definido, el binario no está en `kai-ui-desktop/target/debug/`: ejecutar con `cargo run` o desde `"$CARGO_TARGET_DIR/debug/kai-ui-desktop-prototype"`. Un binario antiguo en `kai-ui-desktop/target/debug/` no refleja los cambios.

## Integración de capacidades de aplicación

Impresión, PDF, preferencias persistentes, escritura atómica, CMS y funciones de IA se modelan como crates/adaptadores opcionales consumidos por las apps. No deben ser dependencia obligatoria de `ui-render` ni del núcleo de widgets salvo que un requisito del framework lo exija.

Para capacidades disponibles en `craft-libs`, revisar [07-integracion-craft-libs.md](07-integracion-craft-libs.md), fijar una revisión reproducible y validar MSRV, features, patches, licencias y soporte macOS antes de adoptar.

## Empaquetado y distribución

1. Definir mínimo de macOS y arquitectura objetivo (Apple Silicon; decidir si Intel es requisito).
2. Crear una app de ejemplo consumidora del framework para validar experiencia de integración.
3. Generar bundle `.app`, nombre/identificador, icono, permisos y metadatos.
4. Documentar firma, notarización y distribución directa o por canal elegido; gestionar certificados fuera del repositorio.
5. Automatizar CI macOS para formato, lint, tests y build. Mantener pruebas gráficas interactivas separadas si el runner no ofrece GPU/entorno apropiados.
6. Publicar ejemplos, guía de instalación y política de versiones/API.

## Definition of Done del primer hito

- Una aplicación de ejemplo se ejecuta como app de escritorio en macOS.
- El botón y campo editable de Kai funcionan con mouse y teclado, incluido foco y entrada de texto validada.
- La interfaz es dibujada por el renderer de Kai; dependencias de bajo nivel no sustituyen los widgets.
- Se conocen y documentan límites de IME, accesibilidad, GPU y distribución; no hay bloqueantes ocultos.
- Tests y comandos para reproducir la verificación están documentados.
