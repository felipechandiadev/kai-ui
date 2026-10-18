# Versionado (web y desktop)

La fuente de verdad es [`version.json`](../version.json) en la raíz del monorepo. Los manifiestos se sincronizan desde ahí.

| Paquete | Manifiesto | Tag de release | Publicación |
|---------|------------|----------------|-------------|
| Web (`@felipechandiadev/ui`) | `kai-ui-web/package.json` | `web-vX.Y.Z` | GitHub Packages (npm) |
| Desktop (`kai-ui-desktop-prototype`) | `kai-ui-desktop/Cargo.toml` | `desktop-vX.Y.Z` | GitHub Release (binario opcional en CI) |

Web y desktop **no comparten número de versión** (web en 1.x, desktop en 0.x prototipo).

## Comandos

```bash
# Alinear package.json / Cargo.toml con version.json
node scripts/version/sync.mjs

# Comprobar coherencia (también corre en CI)
node scripts/version/check.mjs

# Subir versión
node scripts/version/bump.mjs web patch
node scripts/version/bump.mjs desktop minor
node scripts/version/bump.mjs web set 1.5.0

# Tras commitear el bump, crear el tag anotado
node scripts/version/tag.mjs web
git push origin main web-v1.4.4   # ejemplo
```

## Flujo de release web

1. `node scripts/version/bump.mjs web patch`
2. Actualizar `kai-ui-web/CHANGELOG.md`
3. Commit, `node scripts/version/tag.mjs web`, push rama y tag `web-v*`
4. El workflow `publish-web.yml` publica en GitHub Packages
5. `./scripts/subtree/push.sh web`

## Flujo de release desktop

1. `node scripts/version/bump.mjs desktop patch`
2. Actualizar `kai-ui-desktop/CHANGELOG.md`
3. Commit, `node scripts/version/tag.mjs desktop`, push tag `desktop-v*`
4. El workflow `release-desktop.yml` crea el GitHub Release
5. `./scripts/subtree/push.sh desktop`

## Historial

Cada paquete lleva su propio `CHANGELOG.md` (Keep a Changelog). El monorepo no usa un único tag `v*` para ambos: los tags antiguos `v1.3.0` etc. siguen siendo válidos para releases web históricos; los nuevos releases web usan el prefijo `web-v`.
