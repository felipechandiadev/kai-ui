# DataGrid Component

Componente de tabla de datos flexible para aplicaciones Next.js con App Router. Soporta paginación, ordenamiento, búsqueda, filtros y acciones personalizadas con persistencia de estado en URL.

---

## 📁 Estructura de Archivos

Para usar el DataGrid, crea esta estructura en tu página:

```
[tu-pagina]/
├── page.tsx                    # Server Component - fetch de datos
└── ui/
    └── [Modulo]DataGrid.tsx    # Client Component - renderiza DataGrid
```

### Convención de nombres para `[Modulo]DataGrid.tsx`:
- PascalCase con sufijo `DataGrid`
- Ejemplos: `UsersDataGrid.tsx`, `ProductsDataGrid.tsx`, `OrdersDataGrid.tsx`

---

## 🔄 Flujo de Datos

```
┌─────────────────────────────────────────────────────────────────┐
│                         page.tsx                                │
│                    (Server Component)                           │
│                                                                 │
│  1. Lee searchParams de la URL                                  │
│  2. Llama a la función de fetch con los parámetros              │
│  3. Pasa los datos al Client Component                          │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                  [Modulo]DataGrid.tsx                           │
│                    (Client Component)                           │
│                                                                 │
│  1. Recibe data y configuración                                 │
│  2. Define columnas y acciones                                  │
│  3. Renderiza <DataGrid />                                      │
│  4. Actualiza URL en cambios (paginación, filtros, etc.)        │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📋 Implementación

### Paso 1: page.tsx (Server Component)

```tsx
// page.tsx
import { fetchUsers } from './actions'  // Tu función de fetch
import UsersDataGrid from './ui/UsersDataGrid'

interface PageProps {
  searchParams: Promise<{
    page?: string
    limit?: string
    search?: string
    sortField?: string
    sort?: string
  }>
}

export default async function Page({ searchParams }: PageProps) {
  const params = await searchParams
  
  const result = await fetchUsers({
    page: params.page ? parseInt(params.page) : 1,
    limit: params.limit ? parseInt(params.limit) : 10,
    search: params.search || '',
    sortField: params.sortField || 'createdAt',
    sort: (params.sort as 'ASC' | 'DESC') || 'DESC',
  })
  
  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">Usuarios</h1>
      <UsersDataGrid
        data={result.data}
        total={result.total}
        page={result.page}
        limit={result.limit}
      />
    </div>
  )
}
```

### Paso 2: ui/[Modulo]DataGrid.tsx (Client Component)

```tsx
// ui/UsersDataGrid.tsx
'use client'

import DataGrid from '@/shared/components/ui/DataGrid'
import type { DataGridColumn } from '@/shared/components/ui/DataGrid'

interface User {
  id: string
  name: string
  email: string
  role: string
  createdAt: string
}

interface Props {
  data: User[]
  total: number
  page: number
  limit: number
}

export default function UsersDataGrid({ data, total, page, limit }: Props) {
  const columns: DataGridColumn[] = [
    { 
      field: 'name', 
      headerName: 'Nombre',
      sortable: true,
      flex: 1,
    },
    { 
      field: 'email', 
      headerName: 'Email',
      sortable: true,
      flex: 1,
    },
    { 
      field: 'role', 
      headerName: 'Rol',
      width: 120,
      renderCell: ({ value }) => (
        <span className="px-2 py-1 bg-blue-100 text-blue-800 rounded-full text-xs">
          {value}
        </span>
      )
    },
    { 
      field: 'createdAt', 
      headerName: 'Fecha',
      width: 120,
      sortable: true,
      renderCell: ({ value }) => new Date(value).toLocaleDateString()
    },
  ]
  
  return (
    <DataGrid
      columns={columns}
      rows={data}
      totalRows={total}
      title="Usuarios"
    />
  )
}
```

---

## 📊 Props Reference

### DataGridProps

| Prop | Tipo | Default | Descripción |
|------|------|---------|-------------|
| `columns` | `DataGridColumn[]` | **requerido** | Definición de columnas |
| `rows` | `any[]` | `[]` | Datos a mostrar |
| `totalRows` | `number` | **requerido** | Total de registros (para paginación) |
| `title` | `string` | - | Título del grid |
| `height` | `number \| string` | `'70vh'` | Altura del componente (ignorado si `fillViewport`) |
| `fillViewport` | `boolean` | `false` | Pie al borde inferior del viewport; solo el body hace scroll |
| `fillViewportInTabLayout` | `boolean` | `false` | Con `fillViewport`, resta ~52px en el fallback CSS si hay pestañas encima (`dataGridFillViewportTabPageProps`) |
| `viewportBottomInset` | `number` | `24` | Margen inferior en px al calcular `fillViewport` |
| `limit` | `number` | `25` | Filas por página por defecto |
| `showBorder` | `boolean` | `false` | Mostrar borde alrededor |
| `onAddClick` | `() => void` | - | Callback para botón "+" |
| `addButtonVariant` | `'icon' \| 'pillOutlined'` | `'icon'` | Estilo del botón de alta |
| `addButtonLabel` | `string` | - | Con `pillOutlined`: texto del botón (ej. `Recepción`; el + es solo icono) |
| `createForm` | `ReactNode` | - | Formulario de creación (modal interno) |
| `createFormTitle` | `string` | - | Título del modal de creación |
| `onExportExcel` | `() => Promise<void>` | - | Callback para exportar a Excel |
| `expandable` | `boolean` | `false` | Habilitar filas expandibles |
| `expandableRowContent` | `(row) => ReactNode` | - | Contenido del panel expandido |
| `getRowAppearance` | `(ctx) => DataGridRowAppearance \| null` | - | Fondo/clases condicionales por fila. Prioridad de fondo: selección > custom > hover > default |
| `selectedRowId` | `string \| number \| null` | - | Resalta la fila cuyo `row.id` coincide |
| `onRowClick` | `(row) => void` | - | Clic en la fila |
| `headerActions` | `ReactNode` | - | Filtros o acciones de negocio en la cabecera (ver layout abajo) |

### Cabecera (`headerActions`) — layout en grid

La barra superior del DataGrid usa **CSS Grid de 3 columnas** en `md+` y **stack alineado a la izquierda** en `sm` / `xs` (`< md`):

```
| Add + título (col 1)     | vacío (col 2)             | Toolbar + Search (col 3) |  ← fila 1
| header action (col 1)    | header action (col 2)     | header action (col 3)    |  ← fila 2+
```

| Breakpoint | Layout |
|------------|--------|
| `md+` | Grid 3 cols (`auto 1fr auto`); fila 1: Add+título col 1, Toolbar col 3; col 2 fila 1 vacía; acciones desde fila 2 (3 por fila) |
| `sm` / `xs` (`< md`) | Stack vertical, todo alineado a la izquierda: Add+título → Toolbar+Search → acciones (wrap) |

**Recomendación:** pasa cada filtro o control como **hermano directo** (Fragment o array), no envuelto en un `<div className="flex">`. Si usas un wrapper único con varios hijos, el grid los aplana igualmente, pero hermanos directos son más predecibles.

```tsx
<DataGrid
  title="Productos"
  headerActions={
    <>
      <ProductTypeFilter />
      <BrandFilter />
      <StockStatusFilter />
    </>
  }
  /* ... */
/>
```

Demo interactiva: `/design-system/components/datagrid` (sección *Header actions*).

Utilidades: `utils/flattenHeaderActions.ts`, `utils/headerGridPlacement.ts`. Estilos responsive del toolbar/header: `components/Header.module.css` (evita depender de clases Tailwind `sm:grid` que no siempre se generan en consumidores).

### Alineación column headers ↔ celdas

Los títulos de columna y el contenido de cada fila deben compartir **el mismo ancho de columna y el mismo padding horizontal** en todos los breakpoints (incluido scroll horizontal en viewports estrechos).

#### Reglas de implementación

| Tema | Detalle |
|------|---------|
| Padding horizontal | `DataGridCellMetrics.paddingX` (`px-3`) en `ColHeader` y celdas del `Body` |
| Alineación | `align` en celdas; `headerAlign` (fallback `align`) en headers — ver `resolveColumnAlign` / `getColumnAlignClassNames` en `utils/columnStyles.ts` |
| Ancho de fila | Header y body usan `DATA_GRID_COLUMN_ROW_CLASS` → `flex w-max min-w-full` para que ambas filas tengan el **mismo ancho intrínseco** al hacer scroll horizontal |
| Estilos de columna | `calculateColumnStyles(columns, layoutWidth)` — **una sola vez** en `DataGrid.tsx`; el mismo array se pasa a `ColHeader` y a `Body` vía `computedColumnStyles` |
| `layoutWidth` | Ancho real del área scroll (`scrollAreaRef.clientWidth`), medido con `ResizeObserver` — **no** usar `window.innerWidth` (sidebar, tabs y padding desalineaban columnas en md/sm) |
| Columna expand | Header y body: `w-10 min-w-[40px]` + `px-1` |
| Columna acciones fija | Sin `border-left`; header y body aplican los mismos overrides sticky (`pinActionsColumn`) |
| Borde sutil (header, filas, footer) | `dataGridChrome.module.css` → `1px solid color-mix(border 55%, transparent)` en bottom (headers/filas) y top (footer) |

#### Por qué se desalineaba al reducir el viewport

En `lg+` sobra espacio y el bug pasa desapercibido. Al estrechar la ventana:

1. **`w-full` en la fila de headers** limitaba el header al viewport visible mientras el body necesitaba más ancho → columnas desfasadas al scroll horizontal.
2. **`getSmartMinWidth` con `window.innerWidth`** calculaba `minWidth` distinto al espacio real del grid.

Solución: `w-max min-w-full` + medir `gridWidth` del contenedor scroll + compartir `computedStyles`.

### Bordes sutiles (`dataGridChrome`)

Separadores horizontales del grid (no confundir con `showBorder` del contenedor externo).

| Clase CSS module | Aplicado en |
|------------------|-------------|
| `subtleLineBottom` | `ColHeader`, celdas del `Body`, columna expand, fila/panel expandido |
| `subtleLineTop` | `Footer` (mobile y desktop) |

Definición única en `dataGridChrome.module.css`:

```css
1px solid color-mix(in srgb, var(--color-border) 55%, transparent);
```

**Reglas:**

- No usar `border-border` / `border-b border-border` en filas ni headers; importar `dataGridChrome.module.css`.
- Header: borde **inferior** por celda (cada col header al scroll horizontal).
- Footer: borde **superior** en el contenedor de paginación.
- Filas: borde **inferior** por celda, mismo estilo que el header.
- Alineado con el esquema de colores Kai (`--color-border` + `color-mix`); ver showcase `/design-system/foundations/colors`.

#### Archivos relevantes

```
DataGrid/
├── DataGrid.tsx              # gridWidth, computedStyles, fila header
├── components/
│   ├── ColHeader.tsx         # padding/alineación compartidos
│   ├── Body.tsx              # DATA_GRID_COLUMN_ROW_CLASS + computedColumnStyles
│   ├── Footer.tsx
│   └── ...
├── dataGridChrome.module.css # bordes sutiles compartidos (header / filas / footer)
└── utils/
    └── columnStyles.ts       # calculateColumnStyles, alineación, DATA_GRID_COLUMN_ROW_CLASS
```

### DataGridColumn

| Prop | Tipo | Default | Descripción |
|------|------|---------|-------------|
| `field` | `string` | **requerido** | Nombre del campo en el objeto de datos |
| `headerName` | `string` | **requerido** | Texto del encabezado |
| `width` | `number` | - | Ancho fijo en píxeles |
| `minWidth` | `number` | - | Ancho mínimo |
| `maxWidth` | `number` | - | Ancho máximo |
| `flex` | `number` | - | Factor de crecimiento flexible |
| `cellOverflow` | `'truncate' \| 'wrap' \| 'clip' \| 'visible'` | `'truncate'` | Cómo recorta o envuelve contenido largo sin desplazar columnas vecinas |
| `sortable` | `boolean` | `false` | Si la columna es ordenable |
| `filterable` | `boolean` | `true` | Si la columna es filtrable |
| `hide` | `boolean` | `false` | Ocultar la columna |
| `align` | `'left' \| 'center' \| 'right'` | `'left'` | Alineación del contenido |
| `renderCell` | `(params) => ReactNode` | - | Renderizado personalizado |
| `renderType` | `'currency' \| 'badge' \| 'dateString'` | - | Renderizado predefinido |
| `actionComponent` | `ComponentType<{row}>` | - | Componente de acciones |

---

## 🎯 Ejemplos

### Columna con texto largo (`cellOverflow`)

```tsx
{ field: 'name', headerName: 'Nombre', flex: 1, minWidth: 160, cellOverflow: 'truncate' },
{ field: 'notes', headerName: 'Notas', flex: 1, minWidth: 200, maxWidth: 320, cellOverflow: 'wrap' },
```

### Columna con renderizado personalizado

```tsx
{
  field: 'status',
  headerName: 'Estado',
  width: 100,
  cellOverflow: 'truncate',
  renderCell: ({ value }) => (
    <span className={`px-2 py-1 rounded-full text-xs ${
      value === 'active' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
    }`}>
      {value === 'active' ? 'Activo' : 'Inactivo'}
    </span>
  )
}
```

### Columna con imagen

```tsx
{
  field: 'avatar',
  headerName: 'Foto',
  width: 60,
  renderCell: ({ value }) => (
    <img 
      src={value || '/placeholder.png'} 
      alt="Avatar"
      className="w-10 h-10 rounded-full object-cover"
    />
  )
}
```

### Columna de acciones

```tsx
// Componente de acciones
function UserActions({ row }: { row: User }) {
  const router = useRouter()
  
  return (
    <div className="flex gap-1">
      <IconButton icon="edit" size="xs" onClick={() => router.push(`/users/${row.id}/edit`)} />
      <IconButton icon="delete" size="xs" className="text-red-500" onClick={() => handleDelete(row.id)} />
    </div>
  )
}

// En las columnas
{
  field: 'actions',
  headerName: '',
  width: 100,
  align: 'center',
  sortable: false,
  filterable: false,
  actionComponent: UserActions,
}
```

### Con formulario de creación (modal interno)

```tsx
<DataGrid
  columns={columns}
  rows={data}
  totalRows={total}
  title="Usuarios"
  createForm={<CreateUserForm />}
  createFormTitle="Crear Usuario"
/>
```

### Con callback de creación externo

```tsx
<DataGrid
  columns={columns}
  rows={data}
  totalRows={total}
  title="Usuarios"
  onAddClick={() => router.push('/users/new')}
/>
```

### Filas expandibles

```tsx
<DataGrid
  columns={columns}
  rows={data}
  totalRows={total}
  expandable={true}
  expandableRowContent={(row) => (
    <div className="p-4 bg-gray-50">
      <p>Descripción: {row.description}</p>
      <p>Notas: {row.notes}</p>
    </div>
  )}
/>
```

### Fondo condicional por fila

```tsx
<DataGrid
  columns={columns}
  rows={data}
  totalRows={total}
  getRowAppearance={({ row }) => {
    if (!row.isUrgent) return null;
    return {
      backgroundColor:
        'color-mix(in srgb, var(--color-destructive) 8%, var(--color-background))',
      variant: 'urgent', // data-row-appearance="urgent"
    };
  }}
/>
```

---

## 🔗 Contrato de Datos

### Query Params esperados

El DataGrid actualiza la URL con estos parámetros:

| Param | Tipo | Descripción |
|-------|------|-------------|
| `page` | `number` | Página (1-indexed) |
| `limit` | `number` | Registros por página |
| `search` | `string` | Término de búsqueda |
| `sortField` | `string` | Campo para ordenar |
| `sort` | `'asc' \| 'desc'` | Dirección |
| `filters` | `string` | Filtros por columna |

### Respuesta esperada del fetch

```typescript
{
  data: T[],        // Array de registros
  total: number,    // Total sin paginar
  page: number,     // Página actual
  limit: number     // Límite usado
}
```

---

## ✅ Checklist

- [ ] Crear `page.tsx` que lea `searchParams`
- [ ] Crear función de fetch que construya query params
- [ ] Crear `ui/[Modulo]DataGrid.tsx`
- [ ] Definir columnas con `field`, `headerName`
- [ ] Agregar `renderCell` para formatos personalizados
- [ ] Agregar `actionComponent` si necesitas acciones por fila
- [ ] Configurar `onAddClick` o `createForm` si necesitas crear registros
- [ ] Probar paginación, ordenamiento y búsqueda
