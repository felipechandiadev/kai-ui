# 17 — Ribbon Kai del widget Document

**Estado MVP (2026-10-23):** Home + Insert alimentados por el catálogo
`wordcraft_engine::CommandSpec` / `cmd::registry()`. No se porta
`wordcraft-ui-egui` ni assets ArtCraft.

## Diseño

| Pieza | Rol |
|-------|-----|
| `RibbonMode` | `None` / `HomeOnly` / `HomeInsert` (default) |
| `ribbon_catalog(mode)` | Allowlist de ids → `RibbonTab { groups: RibbonGroup { items } }` |
| `DocumentRibbon` | Builder Kai (`Element`): fila `TabList` + panel `Toolbar` |
| Clic | `on_command(cmd_id)` → app → `DocumentCmd` / `Session::run` vía `pending_cmd` |

Fuente de verdad de **label / location / shortcut**: `CommandSpec` en el
rev fijado en `Cargo.toml` (`wordcraft-engine`). La allowlist MVP acota
tamaño; ampliar tabs = añadir ids, no copiar UI egui.

Flujo:

```text
Ribbon botón → Msg::DocCmd(document_cmd_for(id))
            → document().pending_cmd(...)
            → DocumentEditor::apply_pending_cmd → Session::run
```

Disabled: opcional vía `DocumentRibbon::disabled_ids` (p. ej. desde
`Session::why_disabled`). MVP demo no enruta Session al chrome (el leaf
posee la Session).

## API

```rust
use kai::document::{document_ribbon, RibbonMode, document_cmd_for};

document_ribbon()
    .mode(RibbonMode::HomeInsert)
    .active_tab("home")
    .on_tab(Msg::RibbonTab)
    .on_command(|id| Msg::Cmd(document_cmd_for(&id)))
    .build();
```

Archivo (abrir / guardar / PDF) sigue en fila Kai aparte (`documento.archivo`);
no está en el catálogo ribbon MVP (Backstage = fase posterior).

## Tabs MVP

- **Home:** Clipboard, Font (bold/italic/underline/…), Paragraph, Editing
  (undo/redo + select.all). QAT undo/redo se reubican en Editing.
- **Insert:** Tables (`insert.table`), Illustrations (`insert.picture`),
  Pages (`insert.pageBreak`).

## test_id

- `documento.ribbon`
- `documento.ribbon.tab.{home|insert}`
- `documento.ribbon.panel.{home|insert}`
- `documento.ribbon.cmd.{command.id}` (p. ej. `format.bold`)

## Fuera de MVP (deuda)

- Pestañas Review / References / Mailings / …
- Keytips Alt, Backstage completo, Styles gallery live, rulers
- Atlas glifos WordCraft + clipboard body (stash `wip-doc-debt-atlas-clipboard`)

## Cómo probar

```bash
cargo +1.95 test --features document document::ribbon
cargo +1.95 test --features document --demo document_ribbon_bold
# UI: demo → Datos → Documento; Home Negrita; Insert → Table
```
