# Theme tokens (`@felipechandiadev/ui`)

## Import

```css
@import "@felipechandiadev/ui/theme/tokens.css";
@import "@felipechandiadev/ui/theme/components.css";
```

Las PWAs pueden sobrescribir colores y tipografía en `:root` después del import.

## Breakpoints

Alineados a Tailwind:

| Nombre | Min width |
|--------|-----------|
| `xs` | 0 |
| `sm` | 640px |
| `md` | 768px |
| `lg` | 1024px |
| `xl` | 1280px |
| `2xl` | 1536px |

### CSS (recomendado para fuente / gap / padding)

`tokens.css` densifica automáticamente en `xs` y `sm` (sin JS).

Opt-out / override:

```html
<html data-density="comfortable">  <!-- nunca densificar por viewport -->
<html data-density="compact">      <!-- siempre compact -->
```

### JS (layout estructural)

```tsx
import { KaiResponsiveRoot, useBreakpoint } from "@felipechandiadev/ui";

// En Providers:
<KaiResponsiveRoot>{children}</KaiResponsiveRoot>
// → document.documentElement.dataset.kaiBp = "xs" | "sm" | …

const { breakpoint, isMobile, atLeast } = useBreakpoint();
```

## Densidad manual

Default: **comfortable** (sin atributo; en desktop md+).

```html
<html lang="es" data-density="compact">
```

Redefine `--font-size-*`, `--space-*`, `--control-height-md`, `--card-padding`, `--page-gap`, `--datagrid-cell-padding-*`, etc.

### Props `density` (local)

`TextField` / `Select` / `AutoComplete` aceptan `density="compact"` para un control más bajo **sin** cambiar la densidad de la página.

## Tokens relevantes

| Token | Comfortable | Compact / xs (aprox.) |
|-------|-------------|------------------------|
| `--font-size-body` | `1rem` | `0.875rem` |
| `--font-size-page-title` | `1.125rem` | `1rem` |
| `--control-height-md` | `2.25rem` | `2rem` |
| `--card-padding` | `1rem` | `0.75rem` |
| `--page-gap` | `1rem` | `0.75rem` |
