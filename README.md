# kai-ui

Monorepo del design system Kai: librería web y prototipo desktop nativo.

| Carpeta | Descripción |
|---------|-------------|
| [`kai-ui-web/`](kai-ui-web/) | Paquete npm `@felipechandiadev/ui` (React, tokens, componentes). |
| [`kai-ui-desktop/`](kai-ui-desktop/) | Prototipo Rust/macOS con renderer propio. |

## Desarrollo

- **Web:** ver [`kai-ui-web/README.md`](kai-ui-web/README.md).
- **Desktop:** ver [`kai-ui-desktop/README.md`](kai-ui-desktop/README.md).
- **CI:** `.github/workflows/ci.yml` (web en Ubuntu, desktop en macOS).

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
