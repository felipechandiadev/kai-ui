# Theme tokens (`@kai/ui`)

## Import

```css
@import "@felipechandiadev/ui/theme/tokens.css";
@import "@felipechandiadev/ui/theme/components.css";
```

Las PWAs pueden sobrescribir colores y tipografía en `:root` después del import.

## Densidad

Default del DS: **comfortable** (sin atributo).

Para densificar toda la app:

```html
<html lang="es" data-density="compact">
```

Eso redefine `--font-size-*`, `--space-*`, `--control-height-md`, `--card-padding`, `--page-gap`, `--datagrid-cell-padding-*`, etc. Los componentes que leen esas variables se compactan juntos.

### Props `density` (local)

`TextField` / `Select` / `AutoComplete` aceptan `density="compact"` para un control más bajo **sin** cambiar la densidad de la página. Esos estilos usan `--control-height-compact` / `--control-padding-y-compact` (también reducidos bajo `data-density="compact"`).

## Tokens relevantes

| Token | Comfortable | Compact (aprox.) |
|-------|-------------|------------------|
| `--font-size-body` | `1rem` | `0.875rem` |
| `--control-height-md` | `2.25rem` | `2rem` |
| `--card-padding` | `1rem` | `0.75rem` |
| `--page-gap` | `1rem` | `0.75rem` |
