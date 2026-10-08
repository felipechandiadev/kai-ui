# Kai Design System

Referencia central para construir interfaces con `@kai/ui` en las PWAs del monorepo.

## Dónde vive qué

| Capa | Ubicación | Contenido |
|------|-----------|-----------|
| **Tokens** | `packages/ui/src/theme/tokens.css` + `pwa-*/app/globals.css` | Contrato `--color-*`, tipografía, shell |
| **Primitivos** | `packages/ui/src/components/` | Button, DataGrid, layouts, inputs… |
| **Patrones** | Layouts + docs DataGrid | Composición de pantallas ERP |
| **Showcase vivo** | `kai-admin/app/(app)/design-system/` | Foundations, patrones, galería de componentes |

Abrir en dev: **http://localhost:5031/design-system**

## Estructura del showcase (admin)

```
/design-system
├── /                          Hub
├── /foundations
│   ├── /colors                Tokens y reglas semánticas
│   ├── /typography            Contrato global: roles ERP, POS, eShop, reglas
│   ├── /spacing-and-borders   gap layouts, color-mix líneas
│   └── /interaction           hover, focus, disabled
├── /patterns
│   ├── /page-layouts          Basic, Tab, Collection
│   ├── /data-collection       DataGrid ERP
│   └── /forms-feedback        Inputs + Alert + Dialog
├── /components                Galería interactiva @kai/ui (button, datagrid, …)
└── /governance                Reglas @kai/ui vs dominio
```

Navegación centralizada: `kai-admin/src/navigation/designSystemNav.ts`

## Foundations

- **Colores**: solo `var(--color-*)` y clases Tailwind mapeadas en `@theme inline`. Nuevo token → `globals.css` + `tokens.css` + showcase.
- **Tipografía**: contrato global en `typographyTokens.ts` (roles por contexto: jerarquía, formularios, DataGrid, montos, POS, eShop) y `typographyRules.ts` (grupos, reglas, antipatrones). `layoutPageTitleClassName` es alias de `typographyPageTitleClassName`. Montos negativos en UI: signo explícito (`- $48.200`), no paréntesis contables. Texto secundario: `text-muted-foreground` / `typographyMutedClassName`, no `text-muted`. Enlaces: `typographyLinkClassName` / `text-link`. Showcase: `/design-system/foundations/typography`.
- **Espaciado**: `gap-4` (layout normal), `gap-2` (TabPageLayout compact + DataGrid).
- **Bordes**: `border-border` o `color-mix(border 55%, transparent)` en grids.

## Patrones de página

| Escenario | Layout | Notas |
|-----------|--------|-------|
| Detalle / form | `BasicPageLayout` | title opcional; `headerActions` para CTAs a la derecha |
| Módulo con tabs | `TabPageLayout` | tabs slot; `compact` para grids |
| Catálogo cards + URL search | `CollectionPageLayout` | contentItems, addAction |
| Tabla densa ERP | `TabPageLayout` + `DataGrid` | `dataGridFillViewportTabPageProps` |

Tokens compartidos: `packages/ui/src/components/layouts/layoutPageTokens.ts`

## DataGrid

- Header desktop: grid 3 columnas (add+título | vacío | toolbar+search).
- `headerActions`: máx. 3 por fila.
- Chrome unificado: `dataGridChrome.module.css`.
- Docs: `packages/ui/src/components/DataGrid/README.md`

## Gobernanza

1. **@kai/ui** = primitivos sin dominio.
2. **PWA** = Server Actions, Zod, TopBar, features ERP.
3. **No fetch** en UI/hooks de pantalla.
4. Patrón nuevo repetible → documentar en `/design-system` y/o README del componente.
5. Añadir showcase: `ui-components/<name>/page.tsx` + entrada en `designSystemNav.ts`.

## Scripts útiles

```bash
npm run ui:typecheck
npm run ui:audit
```

## Enlaces

- [ADAPTACION.md](./ADAPTACION.md) — matriz app × componente
- [DataGrid README](./src/components/DataGrid/README.md)
