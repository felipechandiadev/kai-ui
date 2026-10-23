# 17 — Ribbon Kai del widget Document

**Estado (2026-10-23):** Home + Insert + **Layout + Review + References + View**
alimentados por el catálogo `wordcraft_engine::CommandSpec` /
`cmd::registry()`. No se porta `wordcraft-ui-egui` ni assets ArtCraft.

## Diseño

| Pieza | Rol |
|-------|-----|
| `RibbonMode` | `None` / `HomeOnly` / `HomeInsert` / **`Full`** (default) |
| `ribbon_catalog(mode)` | Allowlist de ids → `RibbonTab { groups: RibbonGroup { items } }` |
| `default_run_params(id)` | JSON mínimo para cmds que no toleran `{}` (márgenes, columnas…) |
| `DocumentRibbon` | Builder Kai (`Element`): fila `TabList` + panel `Toolbar` |
| Clic | `on_command(cmd_id)` → app → `DocumentCmd` / `Session::run` vía `pending_cmd` |

Fuente de verdad de **label / location / shortcut**: `CommandSpec` en el
rev fijado en `Cargo.toml` (`wordcraft-engine`). La allowlist acota tamaño y
excluye cmds que abren dialogs WordCraft-UI / egui / rfd o exigen params
interactivos (texto libre, archivos, fuentes).

Flujo:

```text
Ribbon botón → Msg::DocCmd(document_cmd_for(id))
            → document().pending_cmd(...)
            → DocumentEditor::run(id, default_run_params(id))
            → Session::run
```

Disabled: opcional vía `DocumentRibbon::disabled_ids` (p. ej. desde
`Session::why_disabled`). MVP demo no enruta Session al chrome (el leaf
posee la Session).

## API

```rust
use kai::document::{document_ribbon, RibbonMode, document_cmd_for};

document_ribbon()
    .mode(RibbonMode::Full)
    .active_tab("home")
    .on_tab(Msg::RibbonTab)
    .on_command(|id| Msg::Cmd(document_cmd_for(&id)))
    .build();
```

Archivo (abrir / guardar / PDF) sigue en fila Kai aparte (`documento.archivo`);
Backstage = fase posterior.

## Tabs

| Tab | Grupos / ids típicos | Notas |
|-----|----------------------|-------|
| **Home** | Clipboard, Font, Paragraph, Editing (+ select.all; QAT undo/redo → Editing) | MVP original |
| **Insert** | Tables, Illustrations, Pages | table / picture / pageBreak |
| **Layout** | Page Setup, Breaks | orientation (toggle), margins→`preset:normal`, columns→`count:2`, size→Letter, hyphenation, lineNumbers, textDirection, sectionRtl, columnBreak, break→`kind:page` |
| **Review** | Proofing, Tracking, Changes, Comments (nav) | spelling, wordCount, trackChanges, markup, accept/reject (±All), next/prev change & comment |
| **References** | TOC, Footnotes | toc / updateToc / removeToc, footnote / endnote / nextFootnote |
| **View** | Views, Show, Zoom, Dark Mode | print/web/draft, ruler/gridlines/nav/hideWhiteSpace, zoom±/100/onePage/pageWidth/multiplePages, darkMode |

### Excluidos (y por qué)

| Cmd / área | Motivo |
|------------|--------|
| `layout.pageSetup` | Empuja `ui_requests` (dialog WC-UI) |
| `layout.manualHyphenation` | Flujo interactivo de puntos de guion |
| `arrange.*` | Requiere selección de objeto flotante |
| `review.newComment` / reply / edit | Exigen `text` / `id` |
| `review.compare` / `combine` | Params de archivo |
| `review.restrict`, speech / readAloud UI | Protect / host audio |
| Citations / sources / researcher / index mark | Params de fuentes o UI de investigación |
| `view.newWindow` / `split` / `fullScreen*` / macros / organizer | Ventana / egui-only / plantillas |
| `view.interfaceTheme` | Exige `value` |
| **Design** (themes/watermark/…) | Casi todos exigen nombre/color/dialog; aplazado |

## test_id

- `documento.ribbon`
- `documento.ribbon.tab.{home\|insert\|layout\|review\|references\|view}`
- `documento.ribbon.panel.{…}`
- `documento.ribbon.cmd.{command.id}` (p. ej. `layout.orientation`)

## Fuera de alcance (deuda)

- Keytips Alt, Backstage completo, Styles gallery live, rulers dibujados
- Design tab, Mailings, contextual Table/Picture Format
- Atlas glifos WordCraft + clipboard body (stash `wip-doc-debt-atlas-clipboard`)

## Cómo probar

```bash
cargo +1.95 test --features document document::ribbon
cargo +1.95 test --features document --lib demo::tests::document_ribbon
# UI: demo → Datos → Documento; tabs Layout / Review / View / References
```
