import React from "react";

/**
 * Columnas de la grilla (mobile-first). Valores 1–12. Omite claves no definidas.
 */
export type CollectionGridColumnConfig = {
  default?: number;
  sm?: number;
  md?: number;
  lg?: number;
  xl?: number;
  "2xl"?: number;
};

/** Mapeo explícito para el scanner de Tailwind (evitar clases dinámicas). */
const GRID_COLS: Record<number, string> = {
  1: "grid-cols-1",
  2: "grid-cols-2",
  3: "grid-cols-3",
  4: "grid-cols-4",
  5: "grid-cols-5",
  6: "grid-cols-6",
  7: "grid-cols-7",
  8: "grid-cols-8",
  9: "grid-cols-9",
  10: "grid-cols-10",
  11: "grid-cols-11",
  12: "grid-cols-12",
};

function clampGridCols(n: number): number {
  if (Number.isNaN(n)) return 1;
  return Math.min(12, Math.max(1, Math.round(n)));
}

function gridColToken(n: number): string {
  return GRID_COLS[clampGridCols(n)] ?? "grid-cols-1";
}

export function buildContentGridClassNames(
  cols: number | CollectionGridColumnConfig | undefined,
): string {
  if (cols == null) {
    return "grid-cols-1";
  }
  if (typeof cols === "number") {
    return gridColToken(cols);
  }
  const parts: string[] = [];
  parts.push(cols.default != null ? gridColToken(cols.default) : "grid-cols-1");
  if (cols.sm != null) parts.push(`sm:${gridColToken(cols.sm)}`);
  if (cols.md != null) parts.push(`md:${gridColToken(cols.md)}`);
  if (cols.lg != null) parts.push(`lg:${gridColToken(cols.lg)}`);
  if (cols.xl != null) parts.push(`xl:${gridColToken(cols.xl)}`);
  if (cols["2xl"] != null) parts.push(`2xl:${gridColToken(cols["2xl"])}`);
  return parts.join(" ");
}

export const DEFAULT_COLLECTION_EMPTY_MESSAGE = "No hay nada que mostrar";

export type RenderCollectionGridContentOptions = {
  contentItems?: React.ReactNode[];
  contentGridColumns?: number | CollectionGridColumnConfig;
  contentGridGapClassName?: string;
  contentGridClassName?: string;
  contentEmptyMessage?: string;
  contentGridItemsAlign?: "start" | "stretch";
  children?: React.ReactNode;
  emptyTestId?: string;
  gridTestId?: string;
  cellTestIdPrefix?: string;
};

export function renderCollectionGridContent({
  contentItems,
  contentGridColumns,
  contentGridGapClassName,
  contentGridClassName,
  contentEmptyMessage,
  contentGridItemsAlign,
  children,
  emptyTestId = "collection-grid-empty",
  gridTestId = "collection-grid-body",
  cellTestIdPrefix = "collection-grid-cell",
}: RenderCollectionGridContentOptions): React.ReactNode {
  if (contentItems !== undefined) {
    if (contentItems.length === 0) {
      const text =
        (contentEmptyMessage ?? DEFAULT_COLLECTION_EMPTY_MESSAGE).trim() ||
        DEFAULT_COLLECTION_EMPTY_MESSAGE;
      return (
        <div
          className="flex min-h-[12rem] w-full min-w-0 flex-1 flex-col items-center justify-center py-10"
          data-test-id={emptyTestId}
        >
          <p className="text-center text-sm text-muted-foreground">{text}</p>
        </div>
      );
    }
    const colClass = buildContentGridClassNames(contentGridColumns ?? 1);
    const gap = contentGridGapClassName?.trim() || "gap-4";
    const alignClass = contentGridItemsAlign === "stretch" ? "items-stretch" : "items-start";
    const cellClass =
      contentGridItemsAlign === "stretch"
        ? "flex h-full min-h-0 min-w-0 flex-col"
        : "min-w-0";
    return (
      <div
        className={`grid w-full min-w-0 ${alignClass} ${colClass} ${gap} ${contentGridClassName ?? ""}`.trim()}
        data-test-id={gridTestId}
      >
        {contentItems.map((item, i) => (
          <div key={i} className={cellClass} data-test-id={`${cellTestIdPrefix}-${i}`}>
            {item}
          </div>
        ))}
      </div>
    );
  }
  return children;
}
