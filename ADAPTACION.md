# Adaptación — `@kai/ui`

Librería de **primitivos UI genéricos** compartidos entre las PWAs Next.js del ecosistema Kai.

## Matriz app × componente

| Componente | admin | pos | eshop | stock | Notas |
|------------|:-----:|:---:|:-----:|:-----:|-------|
| Alert, Badge, Button, IconButton, Switch, DotProgress | ✓ | ✓ | ✓ | ✓ | Fase 1 |
| TextField, Select, AutoComplete, DropdownList, NumberStepper | ✓ | ✓ | ✓ | ✓ | stock sin AutoComplete |
| Dialog, DeleteDialog | ✓ | ✓ | ✓ | ✓ | |
| DataGrid | ✓ | ✓ | — | — | ERP/backoffice |
| layouts (BasicPageLayout, …) | ✓ | ✓ | — | — | |
| Tabs, Card, StatisticsCard | ✓ | ✓ | ✓ | ✓ | Fase 2 |
| Stepper, RangeSlider | ✓ | ✓ | — | — | Fase 2 |
| LoadingState, Skeleton | ✓ | ✓ | ✓ | ✓ | Fase 2 |
| PrintDialog, DialogToPrint | ✓ | ✓ | — | — | Fase 2 |
| TopBar, SideBar, PosTopBar | — | — | — | — | Por app |
| PurchaseDocumentBuilder, PrintDocuments, Multimedia | — | — | — | — | Dominio ERP |
| EShop* (storefront) | — | — | ✓ | — | Solo eShop |

## Incluido en el paquete

| Carpeta | Estado |
|---------|--------|
| Alert, Badge, Button, IconButton, Switch, DotProgress | Listo |
| TextField (unificado admin + touch POS) | Listo |
| Select, AutoComplete, DropdownList, NumberStepper | Listo |
| Dialog (+ DeleteDialog, DialogToPrint, PrintDialog) | Listo |
| DataGrid | Listo |
| layouts (BasicPageLayout, CollectionPageLayout, TabPageLayout) | Listo |
| Tabs, Cards, Stepper, RangeSlider | Listo |
| LoadingState, Skeleton | Listo |

## Permanece en cada PWA

| Carpeta | Motivo |
|---------|--------|
| TopBar/, SideBar/ | Navegación y features admin |
| PurchaseDocumentBuilder/, PlannedPaymentLines/, PrintDocuments/ | Dominio ERP/compras |
| Multimedia/ | Acoplado a features multimedia |
| PosTopBar/ | Específico POS |
| BaseForm/ | Generadores de formularios por entidad |
| Dialog/ChangePasswordDialog | Usa next-auth — vive en cada PWA |
| EShop* (kai-eshop) | Storefront |
| kai-printers-desktop | Pospuesto — stack Tauri/Vite |

## Consumo en apps

**Código actual:** importar primitivos directamente desde `@kai/ui`:

```ts
import { Button, Dialog, TextField } from "@kai/ui";
```

`src/shared/components/` en cada PWA contiene solo **componentes de dominio** (TopBar, Multimedia, EShop*, etc.).

## Limpieza de stubs (post-migración)

Scripts de inventario y migración (raíz del monorepo):

```bash
npm run ui:audit          # conteo legacy vs @kai/ui; stubs eliminables
npm run ui:migrate        # reescribe imports legacy → @kai/ui (por app o all)
npm run ui:remove-stubs   # borra carpetas stub sin referencias (gate con rg)
```

**Gates antes de borrar una carpeta stub:**

1. `rg --glob '*.{ts,tsx}' '@/shared/components/Button' kai-admin` → vacío
2. `npm run ui:typecheck`
3. `npm run build --prefix <pwa-afectada>`

CSS de primitivos (p. ej. `tabs.css`, `dialog.css`): importar desde `@kai/ui/components/...` solo cuando hace falta el stylesheet sin montar el componente.

**No ejecutar** `npm run ui:stubs` tras la limpieza: recrearía stubs eliminados. Reservado para reintroducir un primitivo nuevo antes de migrar imports.

## Tailwind

- **kai-admin**, **kai-pos**, **kai-eshop**, **kai-stock**: Tailwind v4 con `@source "../../../packages/ui/src"` en `globals.css`.
- Cada app define tokens `--color-*` (y opcionalmente `--fs-*`) en `globals.css`.

## Scripts (raíz del monorepo)

```bash
npm run ui:typecheck
npm run ui:test
npm run ui:audit
```

## Contribución

1. Solo primitivos **sin dominio** ni acoplamiento a NextAuth/rutas de negocio.
2. CSS colocalizado + variables `var(--color-*)`.
3. Exportar desde `src/index.ts`; evitar deep imports salvo transición.
4. `next` es peer dependency **opcional** (Tabs usa `next/link`).
