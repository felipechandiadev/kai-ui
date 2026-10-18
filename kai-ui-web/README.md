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

Densidad (default comfortable). Compacto en toda la PWA:

```html
<html data-density="compact">
```

Detalle: [`src/theme/README.md`](./src/theme/README.md).

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

1. En la raíz del monorepo: `node scripts/version/bump.mjs web patch` (o `minor` / `major`).
2. Actualizar `CHANGELOG.md` en esta carpeta.
3. Commit, tag: `node scripts/version/tag.mjs web` → crea `web-vX.Y.Z`.
4. Push: `git push origin main web-vX.Y.Z` — el workflow `publish-web.yml` publica en GitHub Packages.
5. `./scripts/subtree/push.sh web` para el repo standalone.

Los tags legados `vX.Y.Z` siguen disparando `publish.yml`. Detalle: [`../docs/versioning.md`](../docs/versioning.md).

```bash
gh auth refresh -s workflow,write:packages,repo
```
