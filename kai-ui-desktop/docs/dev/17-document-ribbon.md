# 17 — Ribbon Kai del widget Document

**Estado (2026-10-23):** Home + Insert + **Layout + Review + References + View**
alimentados por el catálogo `wordcraft_engine::CommandSpec` /
`cmd::registry()`. Chrome **AppKit**: iconos + tooltip en español, labels de
grupo discretos y QAT Archivo a la izquierda de las tabs. **Ribbon vivo:**
`pressed` / `disabled` cableados a Session (`format.state`, párrafo/caret,
`view.dark_mode`, `Session::why_disabled`) vía `DocumentChange`. No se porta
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

### Chrome vivo (pressed / disabled)

`ribbon_chrome_for(session, mode)` calcula:

| Estado | Fuente Session (sin inventar) |
|--------|-------------------------------|
| **Pressed** | `format.state` → bold/italic/underline/strike/sub/sup; `para_at(focus)` → align visual + bullets/numbering; `view.dark_mode` → `view.darkMode` |
| **Disabled** | catálogo × `Session::why_disabled` / `cmd_disabled` |

Flujo de sync:

```text
DocumentEditor::change()
  → ribbon_chrome_for(session, Full)
  → DocumentChange { ribbon_pressed, ribbon_disabled, … }
  → Demo Msg::Documento → doc_ribbon_pressed / doc_ribbon_disabled
  → DocumentRibbon::pressed_ids / disabled_ids → Element::pressed / ::disabled
```

Toggles vivos (si están en el catálogo del modo): `format.bold|italic|underline|
strikethrough|subscript|superscript`, `para.alignLeft|Center|Right|justify`,
`para.bullets|numbering`, `view.darkMode`. El resto de cmds solo participa en
**disabled** cuando `why_disabled` lo reporta (p. ej. `edit.cut` sin selección).

## API

```rust
use kai::document::{document_ribbon, RibbonMode, document_cmd_for};
use kai::components::toolbar_item;
use kai::icons::Icon;

document_ribbon()
    .mode(RibbonMode::Full)
    .active_tab("home")
    .pressed_ids(pressed)   // desde DocumentChange.ribbon_pressed
    .disabled_ids(disabled) // desde DocumentChange.ribbon_disabled
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

## Chrome Word-like (slice Paso 4)

- **Format Painter** (`edit.formatPainter`): Inicio › Portapapeles. Pressed mientras `Session::painter` está armado. Al soltar un arrastre de selección no colapsada, Kai llama `edit.pasteFormat`. Escape desarma. Atajos ⌘⇧C / ⌘⇧V (`edit.copyFormat` / `edit.pasteFormat`). Sticky (doble clic) = deuda.
- **Estilos mínimos:** `para.normal` / `para.heading1..3` en Inicio › Estilos; pressed según `format.state.style`.
- **Archivo:** QAT Abrir/Guardar/Exportar + línea `documento.archivo.status` con el nombre del último archivo. Backstage full-screen = deuda.
- **Fuera de este slice:** keytips Alt, reglas de página dibujadas (`view.ruler` solo toggle de flag), pestaña Design.

## Tabs

| Tab (UI ES) | id | Grupos / ids típicos | Notas |
|-------------|----|----------------------|-------|
| **Inicio** | `home` | Portapapeles (+ Format Painter), Fuente, Párrafo, Estilos (Normal/H1–H3), Edición | MVP + Paso 4 slice |
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
| `arrange.position` / align / rotate / … | Requieren params o UI; **sí cableados** Paso 6: `arrange.wrap` / `bringForward` / `sendBackward` (objeto seleccionado) |
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
- `documento.ribbon.tab.{home\|insert\|layout\|review\|references\|view\|table}`
- `documento.ribbon.panel.{…}`
- `documento.ribbon.cmd.{command.id}` (p. ej. `layout.orientation`)

## Fuera de alcance (deuda)

- Keytips Alt, Backstage completo (panel Archivo full-screen), Format Painter sticky (doble clic), rulers dibujados en canvas
- Design tab, Mailings, contextual Picture Format completo
- Table Design avanzado (galería completa de estilos, Draw Table, formulas); Paso 6 cableó `table.style`/`look`/`shading`/`borders` + pestaña Tabla mínima
- Pressed para cmds que Session aún no expone como toggle (p. ej. trackChanges /
  markup / zoom presets) — solo disabled vía `why_disabled` hoy
- Mixed/indeterminate en selección heterogénea (format.state es booleano)

## Cómo probar

```bash
cargo +1.95 test --features document document::ribbon
cargo +1.95 test --features document --lib demo::tests::document_ribbon
# UI: demo → Datos → Documento; tipar → seleccionar todo → Negrita:
#   botón format.bold queda pressed; sin selección edit.cut disabled.
# Tabs Diseño / Revisar / Vista / Referencias.
# Format Painter: negrita en una palabra → Copiar formato → arrastrar otra → suelta.
# Estilos: Título 1 pressed; estado Archivo bajo la QAT.
# Paso 6: Inicio Pegar texto/combinar; Layout Ajustar texto; pestaña Tabla;
#   Insertar Encabezado/Pie/Editar/Cerrar.
```
