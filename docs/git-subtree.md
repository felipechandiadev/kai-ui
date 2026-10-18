# Git subtree: `kai-ui-web` y `kai-ui-desktop`

El monorepo [kai-ui](https://github.com/felipechandiadev/kai-ui) es la fuente de verdad. Los repos publicados solo contienen el prefijo correspondiente:

| Paquete | Prefijo en monorepo | Repo subtree |
|--------|---------------------|--------------|
| Web (`@felipechandiadev/ui`) | `kai-ui-web/` | [kai-ui-web](https://github.com/felipechandiadev/kai-ui-web) |
| Desktop (Rust) | `kai-ui-desktop/` | [kai-ui-desktop](https://github.com/felipechandiadev/kai-ui-desktop) |

Remotes locales recomendados: `subtree-web`, `subtree-desktop`.

## Una vez por máquina

```bash
./scripts/subtree/bootstrap-remotes.sh
```

Variables opcionales:

- `KAI_UI_SUBTREE_WEB_REMOTE` / `KAI_UI_SUBTREE_DESKTOP_REMOTE` — URL del remote (HTTPS o SSH).
- `KAI_UI_SUBTREE_BRANCH` — rama publicada (por defecto `main`).

## Publicar cambios del monorepo al repo standalone

Después de commitear en `main`:

```bash
./scripts/subtree/push.sh web
./scripts/subtree/push.sh desktop
```

Equivale a `git subtree push --prefix=kai-ui-web subtree-web main` (y el análogo para desktop).

## Traer cambios del repo standalone al monorepo

Si alguien editó solo el repo subtree:

```bash
./scripts/subtree/pull.sh web
# resolver conflictos si los hay, commitear
```

Por defecto el pull usa `--squash`. Para merge con historial completo: `KAI_UI_SUBTREE_PULL_SQUASH=0 ./scripts/subtree/pull.sh web`.

## Integrar un clone vacío con subtree (caso raro)

Si partes de un monorepo sin la carpeta y el repo subtree ya existe:

```bash
./scripts/subtree/add.sh web
```

## Inspeccionar el historial filtrado

```bash
./scripts/subtree/split.sh web
git log --oneline subtree/web -5
```

## Notas

- El historial del subtree solo incluye commits que tocaron el prefijo actual (`kai-ui-web/` o `kai-ui-desktop/`). Commits antiguos bajo `web/` o `desktop/` no se reescriben automáticamente en el repo publicado.
- CI del monorepo sigue en `.github/workflows/`; los repos subtree pueden tener su propio CI mínimo si hace falta.
- El push inicial a repos nuevos se hace con el mismo `push.sh` una vez creados en GitHub.
