# Guía: `@felipechandiadev/ui`

La librería canónica vive en este repo. kai y realEstate consumen la versión publicada en GitHub Packages.

## Decisiones

| Tema | Valor |
|---|---|
| Gestor | npm |
| Registry | GitHub Packages (`https://npm.pkg.github.com`) |
| Nombre | `@felipechandiadev/ui` |
| Repo | `github.com/felipechandiadev/kai-ui` |
| Build | `tsc` espejando `src/` + copia de CSS a `dist/` |
| Playground | `playground/` (Next). Fuera del publish |
| Primera versión | `1.3.0` |

El scope es el usuario de GitHub. `@kai/ui` no se publica en GitHub Packages con la cuenta `felipechandiadev`.

## Qué entra

Base: `kai/packages/ui`.

Desde realEstate, solo lo genérico que las apps ya usan: `Toast`, `CollectionGrid`, `DotProgress` (variante `wave`), `LocationPreview`, `theme/appearance-theme.ts` y `theme/components.css`.

Queda afuera la UI de dominio (PropertyCard, TopBar, filtros, formularios de contratos). `Calendar` y `navigation/` se conservan de kai.

## Build

Los imports profundos (`@felipechandiadev/ui/components/...`, `theme/...`, `hooks/...`) apuntan a `dist/` con la misma forma de carpetas. No es un bundle único.

```bash
npm run typecheck
npm test
npm run build
npm pack
```

`files` publica `dist` y `README.md`. `playground/` no entra.

## Playground

Next 16, alias a `src/` para desarrollar. La ruta `/` monta `DataGridTable` (SSR, query real vía `NextNavProvider search`) y `DataGrid` (wrapper `ssr: false`).

```bash
cd playground && npm install && npm run dev
```

Para probar el artefacto compilado: `KAI_UI_DIST=1 npm run build` dentro de `playground/`.

## Publicar

CI en pull request: typecheck, test, build. El tag `v*` dispara `npm publish` con `NODE_AUTH_TOKEN`.

En cada consumidor:

```ini
@felipechandiadev:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${NODE_AUTH_TOKEN}
```

```bash
npm i @felipechandiadev/ui@^1.3.0
```

Next: `transpilePackages: ["@felipechandiadev/ui"]`. Tailwind: `@source` a `node_modules/@felipechandiadev/ui/dist`.

## Consumidores

Estrategia en kai: dual temporal. `packages/ui` local sigue hasta que cada app use el paquete publicado; después se archiva.

realEstate reemplaza `@realestate/ui`. `legacy/` no se borra.

Flujo permanente: cambio en este repo, tag, publish, bump en cada producto.
