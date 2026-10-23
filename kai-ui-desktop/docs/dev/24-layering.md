# 24 — Layering AppKit (superficies)

Mapa corto de **capas** de color semántico para ventana / panel / control. No reinventan el chrome (Sidebar + Toolbar); orientan fondos de contenido.

## Superficies

| Clase | Token semántico | Uso típico |
|---|---|---|
| `bg-window` | `windowBackgroundColor` | Fondo de la ventana / shell |
| `bg-under-page` | `underPageBackgroundColor` | Lienzo detrás de “páginas” (Document) |
| `bg-control` | `controlBackgroundColor` | Campos, filas, paneles de contenido |
| `bg-fill-2`…`bg-fill-5` | `systemFill` levels | Hover / agrupaciones / chips |
| `bg-selected-content` | `selectedContentBackgroundColor` | Selección (lista master/detail) |

Bordes: `border-separator` (AppKit) / `border-border` (Kai). Texto: `text-label`, `text-secondary-label`, `text-tertiary-label`.

## Popover / overlay

Diálogos y toasts viven como capas encima del shell (`Role::Dialog`, toast stack). No hay todavía un material “popover glass” tipado aparte: usar `bg-control` + `border-separator` + sombra del tema si hace falta. Liquid Glass real (desenfoque) queda fuera hasta que el renderer lo soporte.

## Demo

**Catálogo** → card «Capas AppKit» (`catalogo.capas`): swatches window / under-page / control / fill-3.

## Qué no es

No es un design-token set paralelo al de [11](11-tema-appkit.md); es una guía de uso de los `SemanticColors` ya existentes.
