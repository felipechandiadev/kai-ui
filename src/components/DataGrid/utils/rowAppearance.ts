import type { DataGridRowAppearance } from '../DataGrid';

/**
 * Prioridad: selección > apariencia custom > hover > default.
 */
export function resolveRowCellBackgroundColor(params: {
  isSelected: boolean;
  isHovered: boolean;
  isExpanded?: boolean;
  appearance?: DataGridRowAppearance | null;
}): string {
  // Una fila expandida mantiene fondo blanco (base) por encima de selección/hover.
  if (params.isExpanded) {
    return 'var(--color-background)';
  }
  if (params.isSelected) {
    return 'color-mix(in srgb, var(--color-primary) 16%, var(--color-background))';
  }
  if (params.appearance?.backgroundColor) {
    return params.appearance.backgroundColor;
  }
  if (params.isHovered) {
    return 'var(--color-hover)';
  }
  return 'var(--color-background)';
}
