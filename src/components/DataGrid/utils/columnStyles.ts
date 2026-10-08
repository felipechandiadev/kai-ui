'use client'
import React from 'react';
import type { DataGridColumn } from '../DataGrid';

export interface ColumnStyle {
  width?: number;
  minWidth?: number;
  maxWidth?: number;
  flex?: string;
  overflow?: string;
}

/** Clases compartidas para filas header/body: mismo ancho intrínseco al hacer scroll horizontal. */
export const DATA_GRID_COLUMN_ROW_CLASS = 'flex w-max min-w-full';

/** Cómo trata el contenido largo la celda respecto al ancho de columna. */
export type DataGridCellOverflow = 'truncate' | 'wrap' | 'clip' | 'visible';

const DEFAULT_CELL_OVERFLOW: DataGridCellOverflow = 'truncate';

/**
 * Clases Tailwind para celda y contenedor interno según `cellOverflow`.
 * La celda incluye `min-w-0` para que flex no la expanda por contenido largo.
 */
export function getCellOverflowClassNames(
  mode: DataGridCellOverflow = DEFAULT_CELL_OVERFLOW,
): { cell: string; content: string } {
  switch (mode) {
    case 'wrap':
      return {
        cell: 'min-w-0 max-w-full overflow-hidden',
        content: 'block min-w-0 w-full break-words [overflow-wrap:anywhere] whitespace-pre-wrap',
      };
    case 'clip':
      return {
        cell: 'min-w-0 overflow-hidden',
        content: 'min-w-0 w-full overflow-hidden',
      };
    case 'visible':
      return {
        cell: 'min-w-0 overflow-visible',
        content: 'min-w-0 w-full',
      };
    case 'truncate':
    default:
      return {
        cell: 'min-w-0 overflow-hidden',
        content: 'min-w-0 w-full truncate',
      };
  }
}

export function resolveColumnCellOverflow(
  column: { cellOverflow?: DataGridCellOverflow },
): DataGridCellOverflow {
  return column.cellOverflow ?? DEFAULT_CELL_OVERFLOW;
}

export type DataGridColumnAlign = 'left' | 'center' | 'right';

/** Padding horizontal compartido entre header y body. */
export const DataGridCellMetrics = {
  paddingX: 'px-3',
} as const;

export function resolveColumnAlign(
  column: { align?: DataGridColumnAlign; headerAlign?: DataGridColumnAlign },
  target: 'header' | 'body',
): DataGridColumnAlign {
  if (target === 'header') {
    return column.headerAlign ?? column.align ?? 'left';
  }
  return column.align ?? 'left';
}

export function getColumnAlignClassNames(
  align: DataGridColumnAlign,
): { cell: string; content: string } {
  switch (align) {
    case 'center':
      return { cell: 'justify-center', content: 'text-center' };
    case 'right':
      return { cell: 'justify-end', content: 'text-right' };
    default:
      return { cell: 'justify-start', content: 'text-left' };
  }
}

/**
 * Hook personalizado para detectar tamaño de pantalla
 */
export function useScreenSize() {
  // Provide a SSR-safe initial state - use 1024 to match server
  const getInitial = () => ({
    width: 1024,
    height: 768,
    isMobile: false,
    isTablet: false,
    isDesktop: true,
  });

  const [screenSize, setScreenSize] = React.useState(getInitial);

  React.useEffect(() => {
    if (typeof window === 'undefined') return;
    const handleResize = () => {
      setScreenSize({
        width: window.innerWidth,
        height: window.innerHeight,
        isMobile: window.innerWidth < 640,
        isTablet: window.innerWidth >= 640 && window.innerWidth < 1024,
        isDesktop: window.innerWidth >= 1024,
      });
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  return screenSize;
}

/**
 * Calcula el ancho mínimo inteligente basado en el tamaño de pantalla y el texto del header
 */
function getSmartMinWidth(
  baseMinWidth: number,
  layoutWidth: number,
  totalColumns: number,
  headerText: string = ''
): number {
  // Calcular ancho mínimo basado en el texto del header
  // Estimación más precisa: caracteres anchos (W, M, etc.) ~10px, caracteres normales ~7px
  const wideChars = (headerText.match(/[WMwm]/g) || []).length;
  const normalChars = headerText.length - wideChars;
  const headerTextWidth = (wideChars * 10) + (normalChars * 7) + 32; // 32px para padding interno

  // Ancho mínimo nunca debe ser menor que el necesario para el header
  const headerBasedMinWidth = Math.max(60, headerTextWidth);

  // En móviles (< 640px):
  // - Calcula espacio disponible después de padding
  // - Distribuye equitativamente entre columnas
  // - Mínimo garantizado: 35px para legibilidad
  if (layoutWidth < 640) { // Mobile
    const availableWidth = layoutWidth - 32;
    const calculatedMinWidth = Math.max(35, Math.floor(availableWidth / totalColumns));

    return Math.max(headerBasedMinWidth, calculatedMinWidth);
  }

  if (layoutWidth < 1024) { // Tablet
    return Math.max(headerBasedMinWidth, Math.max(40, baseMinWidth * 0.8));
  }

  return Math.max(headerBasedMinWidth, baseMinWidth);
}

/**
 * Utilidad centralizada para calcular estilos de columna de DataGrid.
 * `layoutWidth` = ancho real del área scroll del grid (no window.innerWidth).
 */
export function calculateColumnStyles(columns: DataGridColumn[], layoutWidth: number = 1024): ColumnStyle[] {
  const visibleColumns = columns.filter((c) => !c.hide);
  const hasFlex = visibleColumns.some((c) => typeof c.flex === 'number');

  return visibleColumns.map((col, idx) => {
    const style: ColumnStyle = {};

    // Lógica de dimensionamiento
    if (typeof col.width === 'number' && typeof col.flex === 'number') {
      style.flex = `${col.flex} 1 ${col.width}px`;
      style.minWidth =
        typeof col.minWidth === 'number'
          ? col.minWidth
          : col.width;
    } else if (typeof col.width === 'number') {
      style.width = col.width;
      style.flex = `0 0 ${col.width}px`;
    } else if (typeof col.flex === 'number') {
      style.flex = `${col.flex} 1 0`;
    } else {
      // Sistema de distribución automática
      if (hasFlex) {
        style.flex = '1 1 0';
      } else {
        // Última columna se expande, las demás tienen tamaño automático
        if (idx === visibleColumns.length - 1) {
          style.flex = '1 1 0';
        } else {
          style.flex = '0 0 auto';
        }
      }
    }

    // Ancho mínimo inteligente basado en pantalla y texto del header
    const baseMinWidth = typeof col.minWidth === 'number' ? col.minWidth : 50;
    style.minWidth = getSmartMinWidth(baseMinWidth, layoutWidth, visibleColumns.length, col.headerName);

    // Ancho máximo si está definido
    if (typeof col.maxWidth === 'number') {
      style.maxWidth = col.maxWidth;
    }

    return applyOverflowConstraints(style, col);
  });
}

/** Columna flex con contenido acotado: evita que min-content del texto ensanche la fila. */
function isFlexGrowColumn(style: ColumnStyle, col: DataGridColumn): boolean {
  if (typeof col.width === 'number' && typeof col.flex !== 'number') {
    return false;
  }
  if (typeof col.flex === 'number' && col.flex > 0 && typeof col.width !== 'number') {
    return true;
  }
  if (typeof style.flex !== 'string') return false;
  const grow = Number.parseFloat(style.flex.trim().split(/\s+/)[0] ?? '0');
  return grow > 0;
}

function applyOverflowConstraints(style: ColumnStyle, col: DataGridColumn): ColumnStyle {
  const mode = resolveColumnCellOverflow(col);
  if (mode === 'visible') return style;

  if (!isFlexGrowColumn(style, col)) {
    return { ...style, overflow: 'hidden' };
  }

  return {
    ...style,
    width: 0,
    minWidth: style.minWidth ?? 0,
    overflow: 'hidden',
  };
}

/**
 * Capas sticky del DataGrid (scroll vertical u horizontal).
 * El header y la esquina de acciones deben quedar por encima de las celdas de acciones del body.
 */
export const DataGridZIndex = {
  bodyPinnedCell: 1,
  headerRow: 20,
  headerPinnedCell: 21,
  expandedStickyRow: 15,
  // Contenido expandido: por debajo de la fila que lo despliega.
  expandedContent: 14,
} as const;

/**
 * Constantes para estilos consistentes del DataGrid
 */
export const DataGridStyles = {
  // Contenedor principal (fondo transparente: deja ver el fondo ambiental)
  container: 'rounded-md flex flex-col',

  // Contenedor scrollable (min-h-0 permite que flex-1 encoja y haga scroll el body)
  scrollContainer: 'min-h-0 flex-1 overflow-auto',

  // Fila de encabezados de columna (mismo ancho intrínseco que las filas del body).
  headerRow: DATA_GRID_COLUMN_ROW_CLASS,
  headerCell: 'px-3 py-2 text-sm font-medium text-foreground border-r border-border last:border-r-0',

  // Celdas del body (desde Cell.tsx)
  bodyCell: 'px-3 py-1 text-sm text-foreground border-b border-border border-r border-border whitespace-pre-line break-words min-h-[22px] flex-auto last:border-r-0',

  // Responsive breakpoints
  responsive: {
    minWidth: 'min-w-[280px] sm:min-w-[400px] md:min-w-[600px]',
    mobileScroll: 'sm:overflow-x-visible overflow-x-auto', // Scroll horizontal forzado en móviles
  }
} as const;