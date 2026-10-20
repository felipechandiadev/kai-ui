# kai-ui

Monorepo del design system Kai: librería web, framework desktop nativo y su CLI.

| Carpeta | Descripción |
|---------|-------------|
| [`kai-ui-web/`](kai-ui-web/) | Paquete npm `@felipechandiadev/ui` (React, tokens, componentes). |
| [`kai-ui-desktop/`](kai-ui-desktop/) | Framework Rust/macOS (crate `kai`) con renderer propio: arquitectura Elm, componentes, comandos, MCP, imágenes y mapas. Versión preliminar. |
| [`kai-cli/`](kai-cli/) | Binario `kai` para desarrollar apps Kai. Por ahora solo `kai dev` (recompila al guardar y reinicia conservando la ventana). |

## Desarrollo

- **Web:** ver [`kai-ui-web/README.md`](kai-ui-web/README.md).
- **Desktop:** ver [`kai-ui-desktop/README.md`](kai-ui-desktop/README.md). Estado del camino a plataforma: [hoja de ruta, §1.1](kai-ui-desktop/docs/dev/13-hoja-de-ruta-plataforma.md#11-estado-al-20-de-octubre-de-2026).
- **CLI:** `cargo install --path kai-cli` y luego `kai dev` en la carpeta de la app; detalle en [`kai-ui-desktop/README.md`](kai-ui-desktop/README.md#recompilar-y-reiniciar-con-kai-dev).
- **CI:** `.github/workflows/ci.yml` (web en Ubuntu; desktop y `kai-cli` en macOS).

## Versionado

Fuente de verdad: [`version.json`](version.json). Guía completa en [`docs/versioning.md`](docs/versioning.md).

```bash
node scripts/version/check.mjs
node scripts/version/bump.mjs web patch
node scripts/version/tag.mjs web
```

Tags: `web-vX.Y.Z` (npm) y `desktop-vX.Y.Z` (GitHub Release).

## Repos subtree (publicación por carpeta)

Cada paquete se publica también como repositorio independiente vía `git subtree`. Flujo completo en [`docs/git-subtree.md`](docs/git-subtree.md).

```bash
./scripts/subtree/bootstrap-remotes.sh
./scripts/subtree/push.sh web
./scripts/subtree/push.sh desktop
```
