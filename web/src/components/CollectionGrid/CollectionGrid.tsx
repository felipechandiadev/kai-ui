"use client";

import React, { useEffect, useMemo, useRef } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import Footer from "../DataGrid/components/Footer";
import { DataGridStyles } from "../DataGrid/utils/columnStyles";
import {
  DATA_GRID_TAB_LAYOUT_FALLBACK_EXTRA_PX,
  dataGridFillViewportFallbackHeight,
  useDataGridFillViewportHeight,
} from "../DataGrid/utils/useDataGridFillViewportHeight";
import type { DataGridPaginationChange } from "../DataGrid/components/Pagination";
import { CollectionGridHeader } from "./CollectionGridHeader";
import {
  buildContentGridClassNames,
  renderCollectionGridContent,
  type CollectionGridColumnConfig,
} from "./collectionGridUtils";

export type { CollectionGridColumnConfig };

export type CollectionGridProps = {
  title?: string;
  subtitle?: string;
  addAction?: React.ReactNode;
  onAddClick?: () => void;
  addButtonAriaLabel?: string;
  showSearch?: boolean;
  searchParamName?: string;
  searchLabel?: string;
  searchPlaceholder?: string;
  onSearchChange?: (value: string) => void;
  children?: React.ReactNode;
  contentItems?: React.ReactNode[];
  contentEmptyMessage?: string;
  contentGridColumns?: number | CollectionGridColumnConfig;
  contentGridGapClassName?: string;
  contentGridClassName?: string;
  contentGridItemsAlign?: "start" | "stretch";
  loading?: boolean;
  totalRows: number;
  totalGeneral?: number;
  limit?: number;
  showFooter?: boolean;
  paginationMode?: "url" | "controlled";
  page?: number;
  onPaginationChange?: (next: DataGridPaginationChange) => void;
  height?: number | string;
  fillViewport?: boolean;
  fillViewportInTabLayout?: boolean;
  viewportBottomInset?: number;
  showBorder?: boolean;
  className?: string;
  "data-test-id"?: string;
};

function CollectionGridView({
  title,
  subtitle,
  addAction,
  onAddClick,
  addButtonAriaLabel,
  showSearch = true,
  searchParamName = "search",
  searchLabel = "Buscar",
  searchPlaceholder = "Buscar...",
  onSearchChange,
  children,
  contentItems,
  contentEmptyMessage,
  contentGridColumns,
  contentGridGapClassName,
  contentGridClassName,
  contentGridItemsAlign,
  loading = false,
  totalRows,
  totalGeneral,
  limit = 25,
  showFooter = true,
  paginationMode = "url",
  page: controlledPage,
  onPaginationChange,
  height = "70vh",
  fillViewport = false,
  fillViewportInTabLayout = false,
  viewportBottomInset = 24,
  showBorder = false,
  className = "",
  "data-test-id": dataTestId,
}: CollectionGridProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const searchParams = useSearchParams();
  const router = useRouter();

  const fillViewportHeightPx = useDataGridFillViewportHeight(
    fillViewport,
    containerRef,
    viewportBottomInset,
  );

  useEffect(() => {
    if (paginationMode !== "url") return;
    const currentLimit = searchParams.get("limit");
    if (!currentLimit) {
      const params = new URLSearchParams(searchParams.toString());
      params.set("limit", limit.toString());
      router.replace(`?${params.toString()}`, { scroll: false });
    }
  }, [searchParams, limit, router, paginationMode]);

  const containerClasses = [
    DataGridStyles.container,
    fillViewport ? "min-h-0" : "",
    showBorder ? "border border-border" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  const tabLayoutFallbackExtra =
    fillViewport && fillViewportInTabLayout ? DATA_GRID_TAB_LAYOUT_FALLBACK_EXTRA_PX : 0;

  const containerStyle = useMemo((): React.CSSProperties => {
    if (fillViewport) {
      if (fillViewportHeightPx != null) {
        return { height: fillViewportHeightPx };
      }
      return {
        height: dataGridFillViewportFallbackHeight(
          viewportBottomInset,
          tabLayoutFallbackExtra,
        ),
      };
    }
    return {
      height: typeof height === "number" ? `${height}px` : height,
    };
  }, [
    fillViewport,
    fillViewportHeightPx,
    height,
    viewportBottomInset,
    tabLayoutFallbackExtra,
  ]);

  const bodyContent = loading ? (
    <div className="flex min-h-[12rem] w-full flex-1 items-center justify-center py-10">
      <p className="text-sm text-muted-foreground">Cargando…</p>
    </div>
  ) : (
    renderCollectionGridContent({
      contentItems,
      contentGridColumns,
      contentGridGapClassName,
      contentGridClassName,
      contentEmptyMessage,
      contentGridItemsAlign,
      children,
    })
  );

  return (
    <div
      ref={containerRef}
      className={containerClasses}
      style={containerStyle}
      data-test-id={dataTestId ?? "collection-grid-root"}
    >
      <div className="shrink-0">
        <CollectionGridHeader
          title={title}
          subtitle={subtitle}
          addAction={addAction}
          onAddClick={onAddClick}
          addButtonAriaLabel={addButtonAriaLabel}
          showSearch={showSearch}
          searchParamName={searchParamName}
          searchLabel={searchLabel}
          searchPlaceholder={searchPlaceholder}
          onSearchChange={onSearchChange}
        />
      </div>

      <div className={`${DataGridStyles.scrollContainer} p-1`}>{bodyContent}</div>

      {showFooter ? (
        <div className="shrink-0">
          <Footer
            total={totalRows}
            totalGeneral={totalGeneral}
            paginationMode={paginationMode}
            page={controlledPage}
            limit={limit}
            onPaginationChange={onPaginationChange}
          />
        </div>
      ) : null}
    </div>
  );
}

export default CollectionGridView;

export { buildContentGridClassNames };
