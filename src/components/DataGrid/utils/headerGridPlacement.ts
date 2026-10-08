export type GridSlot = { row: number; col: number; colSpan?: number };

/** Columnas del header desktop: Add+título | vacío | Toolbar+Search */
export const HEADER_GRID_COLUMNS = 3;

export const HEADER_GRID_TEMPLATE_COLUMNS = "auto minmax(0, 1fr) auto";

/** Primera fila de acciones (fila 1 reservada para Add+título y Toolbar). */
export const HEADER_ACTIONS_FIRST_ROW = 2;

/**
 * Coloca headerActions en un grid de 3 columnas:
 * - Fila 1: col 1 = Add+título, col 2 vacía, col 3 = Toolbar.
 * - Fila 2+: cols 1–3 = acciones (izq → der, 3 por fila).
 * - Ítems con `fullWidthFlags[i]` ocupan la fila completa (`colSpan: 3`).
 */
export function getActionSlotPositions(
  count: number,
  fullWidthFlags?: readonly boolean[],
): GridSlot[] {
  const slots: GridSlot[] = [];
  let row = HEADER_ACTIONS_FIRST_ROW;
  let col = 1;

  for (let i = 0; i < count; i += 1) {
    const full = fullWidthFlags?.[i] === true;
    if (full) {
      if (col !== 1) {
        row += 1;
        col = 1;
      }
      slots.push({ row, col: 1, colSpan: HEADER_GRID_COLUMNS });
      row += 1;
      col = 1;
      continue;
    }
    slots.push({ row, col });
    col += 1;
    if (col > HEADER_GRID_COLUMNS) {
      col = 1;
      row += 1;
    }
  }

  return slots;
}
