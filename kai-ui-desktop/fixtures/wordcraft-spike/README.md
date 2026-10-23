# Spike WordCraft (§0 Document)

Crate aislado del plan [`docs/dev/16-widget-document.md`](../../docs/dev/16-widget-document.md):
importa solo `wordcraft-doc`, `wordcraft-fonts`, `wordcraft-layout` y `wordcraft-engine`.
No depende de `wordcraft-ui-egui`. El paint del spike usa `layout::display::page_display`
(`Draw`), no `wordcraft-render` / vello (aunque `engine` arrastra `render` de forma transitiva).

## Rev fijado

Ver `Cargo.toml` (`rev` de `storytold/wordcraft`). Nunca `branch = "main"`.

Hace falta `[patch.crates-io] krilla = { git = "…/craft-libs", rev = "…" }` (el mismo que
WordCraft): sin el `Cargo.lock` de WordCraft, cargo toma krilla de crates.io y `craft-pdf`
no compila (`CRAFT_UNWIND_POISONING`).

## Reproducir

```bash
cd kai-ui-desktop/fixtures/wordcraft-spike
cargo +1.95 run --release
cargo +1.95 test --release
```

MSRV declarado de WordCraft / craft-pdf: **1.95**. Compila y corre en **1.89** solo con
`--ignore-rust-version` (el gate de cargo sigue en 1.95). Kai no sube MSRV en este corte.

## Licencia

WordCraft: MIT OR Apache-2.0. No se copia marca ArtCraft.
