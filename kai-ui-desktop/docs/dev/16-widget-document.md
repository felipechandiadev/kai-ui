# 16 — Widget `document`: mini Word dentro de Kai

**Fase:** fases 0–2 hechas (MSRV 1.95; tipar / páginas / tabla 2×2 + imagen). Depende del árbol y la `Session` ([03](03-eventos-y-estado.md)), del renderer ([04](04-renderer-y-texto.md)), de los widgets y el tema ([05](05-widgets-y-tema.md)), de la frontera con craft ([07](07-integracion-craft-libs.md)) y del patrón del mapa y el chart ([10](10-mapas.md), [05](05-widgets-y-tema.md#chart)). La app guía sigue siendo DSP desktop ([13](13-hoja-de-ruta-plataforma.md)).

La base de dominio es [`storytold/wordcraft`](https://github.com/storytold/wordcraft) (WordCraft, ArtCraft; MIT OR Apache-2.0; clean-room). La información de capas y crates se tomó del README y de [`docs/architecture.md`](https://github.com/storytold/wordcraft/blob/master/docs/architecture.md) el **21-oct-2026**. Fijar un `rev` de Git y volver a leer manifests antes de depender.

## 1. Recomendación

- **Un componente Kai, no una app Word embebida.** `document()` / `rich_text()` vive en el árbol como `map` y `chart`: spec en el nodo, paint con `DrawOp`, clics y teclas hacia un `Session` de documento.
- **Motor de documento independiente.** Cosmic-text + glyphon se quedan en la UI (`TextField`, `TextArea`, labels). El cuerpo del documento no comparte `TextStore` ni shaping con un campo.
- **WordCraft como motor, no como UI.** Se reutilizan las capas que no dependen de egui (`doc`, `fonts`, `layout`, `engine`, más adelante `docx` / `formats` / `proof`). No se adopta `wordcraft-ui-egui` ni se pinta el frame de UI con `wordcraft-render` (vello_cpu).
- **Dentro de ese widget, WordCraft de punta a punta.** No híbrido: ni runs de WordCraft medidos con cosmic-text, ni páginas de Kai shapeadas con HarfRust. El híbrido duplica font DB, caret e IME.
- **Progresivo y opcional.** Feature `document`. DSP no necesita ribbon, mail merge, track changes ni ecuaciones en el primer corte.
- **MSRV alineado a WordCraft.** El piso es Rust **1.95** ([06](06-calidad-y-distribucion.md)), el `rust-version` declarado de WordCraft / craft-pdf. Subidas futuras van en el mismo cambio que el `rev` que las exija.

```mermaid
flowchart LR
  view["document builder"] --> kind["Kind::Document"]
  kind --> engine["wordcraft-engine Session"]
  engine --> doc["wordcraft-doc"]
  engine --> layout["wordcraft-layout display list"]
  layout --> paint["Kai DrawList"]
  toolbar["toolbar Kai"] --> cmds["Session::run id"]
  cmds --> engine
```

## 2. Qué es WordCraft y qué no se clona

WordCraft es un procesador de texto en Rust (~85k líneas, 15 crates) con paridad Word como objetivo. Está partido por capas; **nada por debajo de L6 depende de egui, eframe, winit o rfd**.

| Capa | Crates | En Kai |
|---|---|---|
| L0 | `wordcraft-geom` | Unidades del documento (twips, EMU). No sustituye a `geometry.rs`. |
| L1 | `doc`, `fonts`, `proof` | **Sí** el modelo y el shaping del documento. `proof` más tarde. |
| L2 | `layout`, `docx`, `formats`, `docbin` | **Sí** `layout` (display list + hit-test). I/O de archivo desde la fase de persistencia. |
| L3 | `render`, `pdf` | `render` (vello_cpu) **no** en el frame de UI. `pdf` como `Effects` al exportar. |
| L4 | `engine` | **Sí:** `Session`, undo del documento, registro de comandos (`format.bold`, …). |
| L5 | `mcp` | No hace falta: Kai ya tiene MCP sobre su propio registro ([09](09-framework-y-plataforma.md), [14](14-mcp.md)). Se reexportan ids de documento como comandos Kai. |
| L6 | `ui-egui` | **No.** Ribbon, keytips y canvas egui son su front. |
| apps | `wordcraft`, `cli`, `web` | **No** se incrustan. |

La misma frontera que [07](07-integracion-craft-libs.md) marca para `craft-ui`: modelo y lógica, no un segundo renderer.

## 3. Dos motores de texto

| Superficie | Motor | Ejemplos |
|---|---|---|
| Controles Kai | cosmic-text 0.19 + glyphon, `TextStore` | `TextField`, `TextArea`, títulos, toolbar del editor |
| Cuerpo del documento | WordCraft (`fonts` / HarfRust + `layout`) | Párrafos, runs, listas, tablas, páginas |

Un `TextArea` es un buffer plano (un estilo, wrap por palabras, caret de campo). Un documento son runs, estilos, paginación y hit-test por glifo. Compartir `TextStore` mezcla contratos e IME.

El widget posee **una** `wordcraft_engine::Session` por editor, como el mapa posee su cámara. `update` de la app Kai manda ids de comando (`"format.bold"`) o texto; no muta el árbol a mano. El undo del documento es el de WordCraft, no el ⌘Z del `TextField` ([03](03-eventos-y-estado.md)).

## 4. MSRV y supply chain

**MSRV** (*Minimum Supported Rust Version*) es la Rust más vieja con la que Kai promete compilar. Hoy es **1.95** (`rust-version` en `kai-ui-desktop/Cargo.toml` y `kai-cli`; job `desktop-msrv` con `dtolnay/rust-toolchain@1.95`), alineado a WordCraft / craft-pdf (spike §10). Antes era 1.89 por cosmic-text 0.19. El CI diario usa Rust *stable*; el MSRV es el piso de quien clona el repo.

Al cambiar el `rev` de WordCraft:

1. Fijar `rev` (nunca `branch = "main"` en builds reproducibles).
2. Leer `rust-version` y `Cargo.toml` de `doc`, `fonts`, `layout` y `engine`.
3. `cargo tree` y un crate de prueba que solo importe esos cuatro, **sin** `ui-egui`.
4. Si el nuevo `rev` exige más que 1.95, subir el MSRV de Kai (`rust-version`, CI, `kai-cli`, fixture, docs) **en el mismo cambio**.
5. Licencia MIT OR Apache-2.0; NOTICE y `ATTRIBUTION.md` de fuentes e iconos embebidos. La marca ArtCraft en `docs/brand/` **no** se copia.

`muda` se mantiene en **0.19** (comparte objc2 0.6 / objc2-app-kit 0.3 con winit). 0.21 compilaría en 1.95; no se sube salvo que los menús lo pidan.

## 5. Cableado en Kai (el mismo que mapa y chart)

API pública (esqueleto; sin tipar aún):

```rust
document(&self.doc)
    .on_change(Msg::Doc)
    .class("flex-1 min-h-0")
    .label("Informe");
```

- `Kind::Document` + `Content::Document` (caja de clases, como `Map` / `Chart`).
- El builder no recibe el `Document` por valor cada frame si es grande: la `Session` de Kai guarda el `wordcraft_engine::Session` keyed por `WidgetId`, o la app lo tiene en el modelo y el spec lleva un handle. Elegir en el spike; no clonar el doc en `view` si el layout de WordCraft ya cachea.
- `paint`: recorre la display list del layout de WordCraft y emite `DrawOp::{Text, Quad, Image, Geometry}` en la `padding_box`. Sin vello_cpu en este camino.
- Puntero y teclado: hit-test de WordCraft (`layout`) → selección / caret; IME de plataforma entra como `Session::run` de inserción, no como `text_field.rs`.
- Toolbar, menú y paleta: comandos Kai cuyos `into_msg` llaman al id de WordCraft (`format.bold`, `edit.undo`, …). Un subconjunto expuesto a MCP.
- Accesibilidad: `Role::Document` o `Role::TextArea` según el corte; nombre = `label`; más adelante árbol de párrafos. VoiceOver no lee un bitmap.

Tamaño solo por clases (`flex-1`, `min-h-0`, `h-96`). El documento hace scroll en su caja (`overflow-auto`) o pagina y el viewport desplaza páginas: decidirlo en la fase 1.

## 6. Fases

Cada fase es un commit Kai (o un PR). No mezclar dsp-27 ni `kai-cli/src/dev.rs`. `cargo test` / `clippy -D warnings` con permisos de máquina. Feature `document` desde el primer código que importe WordCraft.

### Fase 0 — nota rica (semanas)

Un cuerpo, párrafos y runs: negrita, cursiva, tamaño, color. Toolbar Kai con 6–8 acciones (`format.bold`, `format.italic`, lista, undo). Persistencia: Markdown o JSON propio. Sin paginación ni .docx.

**Gate:** tipear, IME en macOS, seleccionar, ⌘B, TestApp clic/teclas, captura de la demo. Si el spike de `wordcraft-doc` + MSRV tarda, un modelo mínimo de runs *solo* para esta fase está permitido, con la frontera “se sustituye por WordCraft antes de páginas o .docx”.

### Fase 1 — página

Viewport, márgenes, hit-test, selección arrastrando, scroll. Aquí `wordcraft-layout` deja de ser opcional.

**Modelo de scroll (decisión):** viewport fijo del widget (`padding_box` / clases `h-96`…) + stack vertical de páginas Letter en un canvas WordCraft + `scroll_y` interno (rueda / `TestApp::scroll_wheel`). **No** se crece el leaf con overflow-auto del árbol: el documento se comporta como el mapa (contenido propio + pan/scroll interno). Escala al ancho útil (`PAGE_INSET`); gap `PAGE_GAP_PT` entre hojas. Hit-test: coords locales del viewport → índice de página + punto en página → `DocLayout::hit(page, x, y)`.

**Gate:** un documento de varias páginas se desplaza; el caret cae en el run correcto vía hit-test WordCraft; selección arrastrando (multi-página); no se usa cosmic-text para *medir* el cuerpo (el bridge `TextStore`→`DrawOp::Text` en paint es deuda de atlas, no shaping del layout).

### Fase 2 — tablas e imágenes

Insertar tabla 2×2, imagen inline o wrap simple. Reutilizar `ImageSource` / atlas de Kai para bits; WordCraft decide la caja.

**Hecho (23-oct-2026):** comandos `insert.table` (2×2) e `insert.picture` (PNG demo en base64). Display list: `Draw::Fill`/`Line` → quads; `Draw::Image` → `ImageDraw` + `Bitmap` (decode Kai, atlas). Toolbar / Actions / MCP ids públicos. Wrap flotante avanzado = deuda.

### Fase 3 — archivo

Abrir/guardar .docx (`wordcraft-docx`) y exportar PDF (`wordcraft-pdf` / `craft-pdf`) con `Effects::pick_file` / `save_file`. Fuera del hilo de paint.

**Hecho (23-oct-2026):** `pending_load` + `DocumentCmd::TakeDocument` → clone del modelo → `Effects::perform` encode/write. Demo Datos→Documento fila Archivo (Abrir / Guardar / Exportar PDF); Actions + MCP `documento.abrir` / `documento.guardar` / `documento.exportar_pdf`. TestApp `on_file_pick` / `on_file_save`.

### Fuera de este widget (hasta que DSP lo pida)

Mail merge, track changes, comentarios, ecuaciones, Zotero, web/WASM de WordCraft, segundo event loop egui. Ribbon Kai MVP (Home+Insert desde catálogo): ver [`17-document-ribbon.md`](17-document-ribbon.md).

## 7. Pruebas y demo

- Tests del mapeo display list → `DrawOp` y del hit-test (índice de run / caret), como `charts::tests`.
- `TestApp`: tipear, ⌘B, undo; scroll multi-página + tipar tras scroll; query a11y por `label`.
- Sección **Documento** en Datos (junto a Tabla, Mapa, Gráficos) con markdown seed multi-página y fila Archivo. Cobertura en `the_demo_shows_every_component` y `ui.ir_a` (`documento`). Comandos `format.*` / `edit.*` / `insert.*` / `documento.abrir|guardar|exportar_pdf` / schema MCP.
- Captura `demo-documento` (`#[ignore]`, GPU) cuando el patrón de capturas lo permita.

## 8. Criterios de aceptación

- Un solo renderer de UI: Kai. Cero egui en el binario de la demo por esta feature.
- El layout/hit del cuerpo no usa cosmic-text (solo WordCraft). El paint puede puente `TextStore` hasta atlas propio.
- Dependencia WordCraft fijada por `rev`, feature `document`, NOTICE al día.
- MSRV documentado: **1.95** (alineado al `rev` de WordCraft).
- Comandos del editor reutilizan `kai::commands`; MCP no abre un segundo servidor WordCraft.
- Cada fase tiene tests de geometría o de `TestApp` y una entrada en [CHANGELOG](../../CHANGELOG.md).

## 9. Estado

| Pieza | Estado |
|---|---|
| Guía (este documento) | Hecha (21-oct-2026) |
| Spike §4 / §0 (crate aislado + display list) | **Hecho (23-oct-2026)** — ver §10 |
| Feature `document` y crates WordCraft en Kai | **Hecho** (opt-in) |
| `Kind::Document`, paint, hit-test | **Hecho** — Session WordCraft por `WidgetId`; tipar / IME / selección / ⌘B / undo; hit multi-página |
| Demo sección Documento | **Hecho** (Datos → Documento; Ribbon Home+Insert; Archivo; markdown multi-página + scroll) |
| Subida de MSRV | **Hecho** — 1.95 |
| Fase 0 — nota rica | **Hecho (23-oct-2026)** — gate tipar+⌘B+undo+TestApp |
| Fase 1 — páginas | **Hecho (23-oct-2026)** — stack Letter + `scroll_y` viewport; hit-test `layout.hit`; selección arrastre; TestApp scroll+caret |
| Fase 2 — tablas e imágenes | **Hecho (23-oct-2026)** — `insert.table` 2×2 + `insert.picture`; paint fills/lines/ImageDraw (atlas Kai); bridge TextStore glifos = deuda atlas WordCraft |
| Ribbon Kai (catálogo WC) | **MVP (23-oct-2026)** — Home+Insert; ver guía [17](17-document-ribbon.md); keytips/Backstage = deuda |
| Fase 3 — archivo | **Hecho (23-oct-2026)** — abrir/guardar `.docx` + export PDF vía Effects; plan Document **0–3 cerrado** a nivel widget demo |
| Deuda post-plan | Atlas glifos WordCraft (bridge TextStore), clipboard rich, markdown rich de ida y vuelta |

## 10. Spike (23-oct-2026) — go / no-go

Fixture: [`fixtures/wordcraft-spike`](../../fixtures/wordcraft-spike/).

| Pregunta | Resultado |
|---|---|
| ¿`rev` fijado (no `main`)? | Sí: `3a4770ef8ce037d2650c93e2d5764536549357c7` (2026-10-22, tip de `origin/main` al spike) |
| ¿Licencia? | MIT OR Apache-2.0; sin marca ArtCraft |
| ¿Compila `doc` + `fonts` + `layout` + `engine` sin `ui-egui`? | Sí. Cero `egui` / `eframe` / `winit` / `rfd` en `cargo tree` |
| ¿Sin vello en el camino de paint? | El spike pinta vía `layout::display::page_display` → `Draw`. **`engine` arrastra `wordcraft-render` → `vello_cpu` de forma transitiva** (reexport + comandos de imagen/PDF); no se llama desde el spike |
| ¿Display list mínima (un párrafo)? | Sí. 1 página Letter (612×792 pt), 1 `Draw::Glyphs` (~52 glifos, size 12, color negro) |
| ¿MSRV declarado WordCraft? | **1.95** (`workspace.package.rust-version`; `craft-pdf` también 1.95; fork `krilla` declara 1.92) |
| ¿MSRV medido (compila+corre)? | Con `--ignore-rust-version`: **1.89** y **1.92** OK. Sin el flag, cargo rechaza en 1.89 (`craft-pdf` exige 1.95) |
| ¿Kai MSRV (al spike)? | **1.89**; **tras este corte: 1.95** |

### Muestra de display list → paint ops (sin implementar aún)

Salida del spike (`cargo +1.95 run --release`):

```text
spike-ok
pages=1
page_size_pt=612.0x792.0
draw_items=1
draw_summary=glyphs=1 fills=0 lines=0 other=0
sample=Glyphs text="Hola Kai — un párrafo mínimo para el spike Docum" size=12 n_glyphs=52 color=(0,0,0)
```

Mapeo previsto (fase posterior, no en este corte):

| `wordcraft_layout::display::Draw` | Op Kai probable |
|---|---|
| `Glyphs { face, size, glyphs, color, … }` | `DrawOp::Text` / run de glifos (atlas propio del documento, no `TextStore`) |
| `Fill { rect, color, alpha }` | `DrawOp::Quad` |
| `Line { … }` | `DrawOp::Quad` fino o geometría |
| `Image { … }` | `DrawOp::Image` |
| `Path` / `Shape` / `Ink` | `DrawOp::Geometry` |

### Supply chain del spike

- Dependencias git: `storytold/wordcraft` @ rev anterior.
- `[patch.crates-io] krilla` → mismo `craft-libs` rev que WordCraft (`c3487320…`). Obligatorio fuera del lock de WordCraft.
- `engine` tira además: `docx`, `docbin`, `formats`, `pdf`/`craft-pdf`, `render`/`vello_cpu`, `proof`, `metafile`. Para el widget Kai el paint sigue siendo display list → `DrawOp`; vello no entra al frame de UI.

### Veredicto

**Go ejecutado (esqueleto + MSRV 1.95).**

- El motor de documento (layout + display list + `Session`) es usable sin egui y produce ops mapeables.
- Kai alinea MSRV a **1.95** (`rust-version` desktop + `kai-cli` + fixture `external-app`, job `desktop-msrv` `@1.95`). **`muda` sigue en 0.19** (objc2 alineado con winit); no hace falta 0.21 para compilar.
- Corte hecho: `feature = "document"`, `Kind::Document` + paint de hoja, sección **Documento** en la demo Kai (Datos). Fuera de alcance: tipar / IME / ⌘B, páginas, docx, PDF, DSP.

### Cómo reproducir

```bash
cd kai-ui-desktop/fixtures/wordcraft-spike
cargo +1.95 run --release
cargo +1.95 test --release
# Medición 1.89 (solo investigación; no es el contrato de Kai):
cargo +1.89 build --release --ignore-rust-version
```
