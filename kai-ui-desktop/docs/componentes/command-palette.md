# CommandPalette

Paleta de comandos: un fondo sobre toda la ventana y un panel con un campo de
búsqueda y la lista de comandos de la app.

Código: `src/components.rs` (`command_palette`, `PaletteItem`).

## Uso

La app normalmente **no** la construye: la sesión la abre con el comando
`kai.paleta` (⌘⇧P) y la llena con el registro de comandos (`App::commands`).
Cada acción registrada con `.expose()` o con menú aparece; las
deshabilitadas salen con su motivo.

`command_palette(items, query, selected)` existe para capturas y para quien
quiera mostrarla por su cuenta; los elementos no envían mensajes: el
comportamiento vive en la sesión.

## `PaletteItem`

| Campo | Qué es |
|---|---|
| `id` | Id del comando. |
| `title` | Texto de la fila. |
| `shortcut` | Atajo a mostrar. |
| `disabled` | Por qué no puede correr ahora (fila deshabilitada). |

## Teclado y accesibilidad

El campo `kai.paleta.buscar` tiene el foco; ↑/↓ mueven la selección, Return
ejecuta y Escape cierra. La fila elegida es el *active descendant* del
campo, así VoiceOver la lee sin mover el foco. Filas: `kai.paleta.<id>`.

## Relacionados

[Elementos comunes: comandos](elementos-comunes.md#mensajes-y-comandos) ·
`docs/dev/14-mcp.md` (los mismos comandos llegan a agentes)
