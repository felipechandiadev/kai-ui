# Componentes de Kai UI Desktop

Una página por componente, con su API real (`kai::components`, `kai::charts`,
`kai::maps`, `kai::document`), ejemplos, teclado, accesibilidad, partes para
tests y diferencias entre temas. El test `components_have_docs`
(`src/components.rs`) falla si un componente público no tiene su página.

Antes de leer un componente conviene leer [Elementos y modificadores
comunes](elementos-comunes.md): clases, mensajes, comandos, `test_id`,
partes, foco y menús contextuales valen para todos.

## Acciones

| Componente | Para qué |
|---|---|
| [Button](button.md) | Acción con texto (`button`, `button_sized`, `Button`) |
| [IconButton](icon-button.md) | Acción con solo icono (`icon_button`) |
| [Toolbar](toolbar.md) | Barra superior con `toolbar_item`, título, separadores y búsqueda |
| [CommandPalette](command-palette.md) | Paleta de comandos (⌘⇧P) |

## Entrada de datos

| Componente | Para qué |
|---|---|
| [TextField](text-field.md) | Campo de una línea, con adornos, contraseña y errores |
| [TextArea](text-area.md) | Campo de varias líneas con contador |
| [SearchField](search-field.md) | Búsqueda con lupa y botón de borrar |
| [Autocomplete](autocomplete.md) | Combobox: opciones al escribir, de una lista o una búsqueda asíncrona |
| [ColorField](color-field.md) | Color en hex con muestra y paleta |
| [DateField y TimeField](date-field.md) | Fecha `dd/mm/aaaa` con calendario y hora `HH:MM` |
| [Select](select.md) | Desplegable de una opción (`NSPopUpButton`) |
| [Segmented](segmented.md) | Control segmentado |
| [Switch](switch.md) | Interruptor sí/no |
| [Checkbox](checkbox.md) | Casilla con estado mixto |
| [MediaSlot](media-slot.md) | Imagen de un formulario (foto, portada, logo) |

## Contenido y estado

| Componente | Para qué |
|---|---|
| [Card](card.md) | Tarjeta con encabezado, contenido y acciones |
| [GroupBox](group-box.md) | Caja con título (`NSBox`) |
| [Stat](stat.md) | Cifra con etiqueta y nota |
| [Tipografía](tipografia.md) | `heading`, `paragraph`, `caption` |
| [Badge](badge.md) | Etiqueta de estado en píldora |
| [Alert](alert.md) | Mensaje con icono y color semántico |
| [Avatar](avatar.md) | Foto o inicial de una persona |
| [EmptyState](empty-state.md) | Superficie vacía y `loading_placeholder` |
| [DotProgress](dot-progress.md) | Progreso indeterminado en puntos |

## Superposiciones

| Componente | Para qué |
|---|---|
| [Dialog](dialog.md) | Diálogo modal con botones por defecto y cancelar |
| [Toast](toast.md) | Avisos que se cierran solos |

## Navegación y página

| Componente | Para qué |
|---|---|
| [Sidebar](sidebar.md) | Lista de secciones, con riel plegado |
| [Tabs](tabs.md) | Pestañas sobre un panel |
| [ListToolbar](list-toolbar.md) | Barra de lista: info, acciones y búsqueda expandible |
| [Paneles](panes.md) | Paneles sólidos lado a lado y sidebar flotante de cristal |
| [Page y PageGrid](page.md) | Contenedor de sección y grilla 4/8/12 |
| [Plantillas de página](page-templates.md) | `PageHeader`, lista + detalle, formulario, vista de datos |

## Datos

| Componente | Para qué |
|---|---|
| [Table](table.md) | Tabla con búsqueda, filtros, orden, columnas fijas y exportación |
| [VirtualList](virtual-list.md) | Lista que construye solo las filas visibles |
| [Chart](chart.md) | Barras y sparkline |
| [Map](map.md) | Mapa vectorial con capas y marcadores |
| [Document](document.md) | Editor de documentos tipo Word (feature `document`) |
