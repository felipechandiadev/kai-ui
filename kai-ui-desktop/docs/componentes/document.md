# Document

Editor de documentos tipo Word / Pages dentro de una app Kai, con motor
WordCraft: páginas, estilos, listas, tablas, imágenes, encabezado y pie,
buscar y reemplazar, comentarios, ortografía, abrir y guardar `.docx` y
exportar PDF. Requiere la feature `document`.

Código: `src/document/` (`mod.rs`, `editor.rs`, `ribbon.rs`). Guías:
`docs/dev/16-widget-document.md` y `docs/dev/17-document-ribbon.md`.

## Uso

```rust
use kai::document::{document, document_ribbon, document_cmd_for, DocumentCmd, RibbonMode};

// La ribbon, fuera del editor.
document_ribbon()
    .mode(RibbonMode::Full)
    .active_tab(self.pestana.clone())
    .pressed_ids(self.pressed.clone())
    .disabled_ids(self.disabled.clone())
    .contexts(self.contextos.clone())
    .on_tab(Msg::Pestana)
    .on_command(|id| Msg::Cmd(document_cmd_for(&id)))
    .build()

// El editor.
let mut editor = document()
    .label("Informe")
    .markdown(&self.texto)
    .on_change(Msg::Cambio)
    .class("flex-1 min-h-0");
if let Some((cmd, serial)) = &self.cmd {
    editor = editor.pending_cmd(cmd.clone(), *serial);
}

// update
Msg::Cmd(cmd) => { self.serial += 1; self.cmd = Some((cmd, self.serial)); }
Msg::Cambio(c) => {
    self.texto = c.markdown;
    self.pressed = c.ribbon_pressed;
    self.disabled = c.ribbon_disabled;
    self.contextos = c.ribbon_contexts;
}
```

## Editor

| Método | Qué hace |
|---|---|
| `document()` | Crea el editor. Dale tamaño y nombre accesible. |
| `markdown(text)` | Contenido que la sesión carga y sincroniza. |
| `placeholder_text(text)` | Texto de reserva. |
| `on_change(f)` | Recibe un `DocumentChange` cuando cambia. |
| `pending_cmd(DocumentCmd, serial)` | Ejecuta un comando una vez por `serial` nuevo. |
| `pending_load(bytes, serial)` | Carga un `.docx` una vez por `serial` nuevo. |

### `DocumentChange`

`plain_text`, `markdown`, `ribbon_pressed`, `ribbon_disabled`,
`ribbon_contexts` (pestañas contextuales visibles), `status` (resultado de
buscar, comentar, revisar), `find_match_count`.

### `DocumentCmd`

`Bold`, `Italic`, `Bullets`, `Undo`, `Redo`, `InsertTable`, `InsertPicture`,
`Find { text }`, `FindNext`, `FindPrevious`, `Replace { text, with }`,
`ReplaceAll { text, with }`, `NewComment { text }`, `InsertText { text }`
(escribe en el caret), `Clear` (borra todo el contenido), `PastePlain`,
`PasteMerge`, `PasteKeepSource`, `Wrap { mode }`, `TableStyle { style }`,
`TakeDocument` (clona el modelo para guardar o exportar fuera del hilo de
UI) y `Raw(id)` para cualquier id de WordCraft (`format.underline`…).

## Ribbon

| Método | Qué hace |
|---|---|
| `document_ribbon()` | Builder. |
| `mode(RibbonMode)` | `None`, `HomeOnly`, `HomeInsert`, `Full` (por defecto). |
| `active_tab(id)` / `on_tab(f)` | Pestaña activa (`home`, `insert`, `layout`, `review`, `references`, `view`, y las contextuales `table` y `picture`). |
| `on_command(f)` | Recibe el id del comando; `document_cmd_for(id)` lo convierte en `DocumentCmd`. `review.newComment` solo emite el id: la app pide el texto. |
| `pressed_ids` / `disabled_ids` / `contexts` | Estado vivo, desde `DocumentChange`. |
| `leading(items)` | Botones a la izquierda de las pestañas (Abrir, Guardar, Exportar PDF). |
| `test_id`, `build()` | — |

Forma Word / Pages: pestañas subrayadas (píldoras en Glass), grupos en una
fila con su nombre abajo, botones grandes para las acciones principales,
galería de estilos, y colapso por ancho de ventana (galería → desplegable
bajo 1280 pt, botones grandes solo icono bajo 1024, sin etiquetas de grupo
bajo 768). En Glass es un panel flotante redondeado.

## Comportamiento

- La hoja se ajusta al ancho disponible sin pasar del 100 %; el alto se
  recorre con scroll.
- El texto se rasteriza a la densidad de la pantalla (2× en Retina).
- Cursor de texto sobre el área del documento.
- IME con preedit visible, portapapeles con formato (HTML, imágenes, tablas).

## Tests (`TestApp`)

`document_selection_collapsed(by)`, y los comandos de la ribbon por
`document.ribbon.cmd.<id>`; pestañas `document.ribbon.tab.<id>`.

## Pendiente

Ver «Deuda post-plan» en `docs/dev/16-widget-document.md`.
