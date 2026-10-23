# 17 — Ribbon Kai del widget Document

**Estado (2026-10-23):** Home + Insert + **Layout + Review + References + View**
alimentados por el catálogo `wordcraft_engine::CommandSpec` /
`cmd::registry()`. Chrome **AppKit**: iconos + tooltip en español, labels de
grupo discretos y QAT Archivo a la izquierda de las tabs. No se porta
`wordcraft-ui-egui` ni assets ArtCraft.

## Diseño

| Pieza | Rol |
|-------|-----|
| `RibbonMode` | `None` / `HomeOnly` / `HomeInsert` / **`Full`** (default) |
| `ribbon_catalog(mode)` | Allowlist de ids → `RibbonTab { groups: RibbonGroup { items } }` |
| `icon_for_cmd` / `localize_cmd` | Icono Lucide Kai + etiqueta ES (tooltip / a11y); ids de comando siguen en EN |
| `default_run_params(id)` | JSON mínimo para cmds que no toleran `{}` (márgenes, columnas…) |
| `DocumentRibbon` | Builder Kai (`Element`): QAT opcional + fila `TabList` + panel `Toolbar` |
| Clic | `on_command(cmd_id)` → app → `DocumentCmd` / `Session::run` vía `pending_cmd` |

### Modelo visual (AppKit / Word ligero)

1. **Iconos + tooltip** (`toolbar_item`): el label largo del catálogo WordCraft
   no se pinta; el tooltip / nombre accesible es la etiqueta ES (+ atajo si
   existe). Si no hay icono razonable → botón de texto corto ES.
2. **Grupos**: label 11px muted (`Portapapeles`, `Fuente`, `Párrafo`, …) +
   **separador vertical** entre grupos.
3. **Densidad**: `IconButton`/`toolbar_item` small / AppKit control; gaps
   `gap-0.5`–`gap-1.5` sin inflar altura.
4. **Idioma UI en español**; ids internos (`format.bold`, …) en inglés.
5. **Archivo (Abrir / Guardar / Exportar PDF)**: QAT borderless a la izquierda
   de las tabs (`DocumentRibbon::leading`), no una fila Outlined suelta.
6. **Estados**: hover / pressed / `selected` (toggle) / disabled ~40 % —
   reutiliza `toolbar_item` + variantes Button existentes.

Fuente de verdad de **shortcut / location**: `CommandSpec` en el rev fijado
en `Cargo.toml` (`wordcraft-engine`). Labels visibles los aporta Kai
(`localize_cmd`). La allowlist acota tamaño y excluye cmds que abren dialogs
WordCraft-UI / egui / rfd o exigen params interactivos.

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
use kai::components::toolbar_item;
use kai::icons::Icon;

document_ribbon()
    .mode(RibbonMode::Full)
    .active_tab("home")
    .on_tab(Msg::RibbonTab)
    .on_command(|id| Msg::Cmd(document_cmd_for(&id)))
    .leading([
        toolbar_item(Icon::FolderOpen, "Abrir…").on_command(DocAbrir),
        toolbar_item(Icon::Save, "Guardar…").on_command(DocGuardar),
        toolbar_item(Icon::FileDown, "Exportar PDF…").on_command(DocExportarPdf),
    ])
    .build();
```

Backstage = fase posterior.

## Tabs

| Tab (UI ES) | id | Grupos / ids típicos | Notas |
|-------------|----|----------------------|-------|
| **Inicio** | `home` | Portapapeles, Fuente, Párrafo, Edición (+ select.all; QAT undo/redo → Edición) | MVP original |
| **Insertar** | `insert` | Tablas, Ilustraciones, Páginas | table / picture / pageBreak |
| **Diseño** | `layout` | Configurar página, Saltos | orientation (toggle), margins→`preset:normal`, columns→`count:2`, size→Letter, hyphenation, lineNumbers, textDirection, sectionRtl, columnBreak, break→`kind:page` |
| **Revisar** | `review` | Revisión, Seguimiento, Cambios, Comentarios (nav) | spelling, wordCount, trackChanges, markup, accept/reject (±All), next/prev change & comment |
| **Referencias** | `references` | Tabla de contenido, Notas al pie | toc / updateToc / removeToc, footnote / endnote / nextFootnote |
| **Vista** | `view` | Vistas, Mostrar, Zoom, Modo oscuro | print/web/draft, ruler/gridlines/nav/hideWhiteSpace, zoom±/100/onePage/pageWidth/multiplePages, darkMode |

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
- `documento.archivo` (QAT Abrir / Guardar / Exportar)
- `documento.ribbon.tab.{home\|insert\|layout\|review\|references\|view}`
- `documento.ribbon.panel.{…}`
- `documento.ribbon.cmd.{command.id}` (p. ej. `layout.orientation`)

## Fuera de alcance (deuda)

- Keytips Alt, Backstage completo, Styles gallery live, rulers dibujados
- Design tab, Mailings, contextual Table/Picture Format
- Atlas glifos WordCraft + clipboard body (stash `wip-atlas-clipboard-debt`)
- Toggle-on cableado a `Session` (bold/italic pressed state)

## Cómo probar

```bash
cargo +1.95 test --features document document::ribbon
cargo +1.95 test --features document --lib demo::tests::document_ribbon
# UI: demo → Datos → Documento; tabs Diseño / Revisar / Vista / Referencias
```
