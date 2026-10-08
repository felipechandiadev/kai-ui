# `@felipechandiadev/ui`

Design system de kai y realEstate. Se publica en GitHub Packages.

## Uso

```ts
import { Button, Dialog, DataGrid, DataGridTable, Toast, CollectionGrid } from "@felipechandiadev/ui";
```

Tokens y CSS colocalizado:

```css
@import "@felipechandiadev/ui/theme/tokens.css";
@import "@felipechandiadev/ui/theme/components.css";
```

En Tailwind v4, escanear las clases del paquete:

```css
@source "../node_modules/@felipechandiadev/ui/dist";
```

Las apps Next necesitan `transpilePackages: ["@felipechandiadev/ui"]` para compilar los CSS Modules.

`DataGrid` es el wrapper con `ssr: false`. `DataGridTable` es la grilla que sí renderiza en el servidor. Para que la paginación por URL sobreviva ese render, pasar la query al provider:

```tsx
<NextNavProvider search={querySinSigno}>
  <DataGridTable columns={columns} rows={rows} totalRows={total} />
</NextNavProvider>
```

## Registry

En el consumidor, con un token `read:packages`:

```ini
@felipechandiadev:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${NODE_AUTH_TOKEN}
```

```bash
npm i @felipechandiadev/ui@^1.3.0
```

## Scripts

- `npm run typecheck`
- `npm run test`
- `npm run build` — `tsc` a `dist/` y copia de CSS al lado de cada componente

## Playground

`playground/` es una app Next aparte. No entra en el paquete publicado.

```bash
cd playground && npm install && npm run dev
```

`http://localhost:3099/?page=2&limit=25&filtration=true` muestra el DataGrid directo (HTML de servidor) y el wrapper (hueco con `DotProgress` hasta hidratar).

## Release

Un cambio de UI se versiona aquí, se etiqueta `vX.Y.Z` y el workflow `publish` lo sube a GitHub Packages. Cada producto actualiza el rango con un PR propio.

El repo remoto es https://github.com/felipechandiadev/kai-ui. El tag local `v1.3.0` ya está creado. Para subirlo, el token de `gh` tiene que incluir `workflow` y `write:packages`:

```bash
gh auth refresh -s workflow,write:packages,repo
git push -u origin main
git push origin v1.3.0
```
