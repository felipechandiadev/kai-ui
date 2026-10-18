# 06 — Calidad y distribución para macOS

## Objetivo

Asegurar que el framework pueda usarse en aplicaciones desktop reales, no solo en un ejemplo local.

## Calidad técnica

Ejecutar los gates en cada fase L0–L6; ningún resultado manual pendiente se marca como aprobado por inferencia.

1. Baseline por incremento: `cargo fmt --manifest-path desktop/Cargo.toml --check`, `cargo check --manifest-path desktop/Cargo.toml`, `cargo test --manifest-path desktop/Cargo.toml` y `cargo clippy --manifest-path desktop/Cargo.toml --all-targets -- -D warnings`.
2. **L0:** tests puros de geometría/unidades/conversiones; comprobar hit-test y dibujo con los mismos bounds.
3. **L1–L3:** tests de transiciones, invalidación, traducción de eventos y despacho de comandos sin ventana.
4. **L4:** resultados de layout deterministas y equivalencia de caché contra cálculo completo; benchmark antes de alegar mejoras.
5. **L5–L6:** tests de paint/tree y pruebas integradas; añadir capturas visuales cuando la ruta sea estable.
6. Validar manualmente HiDPI, resize, foco, suspensión/reanudación, cierre, IME, Unicode y portapapeles en macOS; registrar específicamente qué se probó y el resultado.
7. Medir inicio, frame time, CPU/GPU, fuentes y memoria antes de optimizar; documentar hardware, macOS y método.
8. Auditar VoiceOver/roles/etiquetas/foco/acciones y revisar dependencias, licencias y comportamiento sin GPU antes del hito de distribución.

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
