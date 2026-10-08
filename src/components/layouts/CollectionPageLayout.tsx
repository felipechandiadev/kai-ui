"use client";

import React, { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "../../navigation";
import { Search } from "lucide-react";
import TextField from "../TextField";
import IconButton from "../IconButton";
import {
  layoutPageContentClassName,
  layoutPageHeaderClassName,
  layoutPageRootClassName,
  layoutPageSubtitleClassName,
  layoutPageTitleClassName,
} from "./layoutPageTokens";

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

export type CollectionPageLayoutProps = {
  /** Título principal (arriba a la izquierda). Si está vacío y no hay subtítulo, no se reserva espacio. */
  title?: string;
  /** Subtítulo bajo el título. Si está vacío, no se muestra. */
  subtitle?: string;
  /**
   * Slot de la acción “añadir” (izquierda). Pasa un componente cliente (p. ej. botón + Dialog)
   * creado en la feature. Si está definido, sustituye al `IconButton` por defecto y se ignora
   * `onAddClick` / `addButtonAriaLabel` salvo que no quieras pasar `addAction` y uses el botón genérico.
   */
  addAction?: React.ReactNode;
  /** Acción del botón + integrado (solo si no usas `addAction`). */
  onAddClick?: () => void;
  /** Etiqueta accesible del botón + por defecto (solo sin `addAction`). */
  addButtonAriaLabel?: string;
  /** Mostrar barra de búsqueda sincronizada con la URL (query). */
  showSearch?: boolean;
  /** Nombre del parámetro en la query (p. ej. `search` → `?search=...`). */
  searchParamName?: string;
  /** Etiqueta flotante del TextField de búsqueda */
  searchLabel?: string;
  searchPlaceholder?: string;
  /**
   * Contenido libre. Si `contentItems` está definido (incl. `[]`), la sección principal no usa
   * `children`: con ítems muestra grilla; con array vacío muestra vacío centrado.
   */
  children?: React.ReactNode;
  /**
   * Celdas en grilla. Si se pasa un array **vacío** (sin datos, o filtro sin coincidencias),
   * se muestra un mensaje centrado (ver `contentEmptyMessage`), no `children`.
   * Si el array tiene elementos, se renderizan en grilla; `children` se ignora.
   */
  contentItems?: React.ReactNode[];
  /**
   * Texto mostrado centrado cuando `contentItems` es `[]` (sin datos, filtro, etc.; genérico, no solo búsqueda).
   * @default "No hay nada que mostrar"
   */
  contentEmptyMessage?: string;
  /**
   * Número de columnas fijo, o desglose por breakpoint. Por defecto 1.
   * Solo aplica con `contentItems` no vacío.
   */
  contentGridColumns?: number | CollectionGridColumnConfig;
  /**
   * Clase de separación entre celdas (p. ej. `gap-4`, `gap-3 md:gap-6`). Por defecto `gap-4`.
   */
  contentGridGapClassName?: string;
  /**
   * Clases extra en el contenedor de la grilla (p. ej. alineación o `self-stretch`).
   */
  contentGridClassName?: string;
  /**
   * Alineación vertical de las celdas en la grilla. `stretch` iguala la altura de la fila
   * a la card más alta (útil con cards `fillHeight`).
   * @default 'start'
   */
  contentGridItemsAlign?: "start" | "stretch";
  className?: string;
  "data-test-id"?: string;
};

const SEARCH_DEBOUNCE_MS = 300;

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
  cols: number | CollectionGridColumnConfig | undefined
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

const DEFAULT_CONTENT_EMPTY_MESSAGE = "No hay nada que mostrar";

function renderContentSection(
  contentItems: React.ReactNode[] | undefined,
  contentGridColumns: number | CollectionGridColumnConfig | undefined,
  contentGridGapClassName: string | undefined,
  contentGridClassName: string | undefined,
  contentEmptyMessage: string | undefined,
  contentGridItemsAlign: "start" | "stretch" | undefined,
  children: React.ReactNode
) {
  if (contentItems !== undefined) {
    if (contentItems.length === 0) {
      const text = (contentEmptyMessage ?? DEFAULT_CONTENT_EMPTY_MESSAGE).trim() || DEFAULT_CONTENT_EMPTY_MESSAGE;
      return (
        <div
          className="flex min-h-[12rem] w-full min-w-0 flex-1 flex-col items-center justify-center py-10"
          data-test-id="collection-page-layout-empty"
        >
          <p className="text-center text-sm text-muted-foreground">{text}</p>
        </div>
      );
    }
    const colClass = buildContentGridClassNames(
      contentGridColumns ?? 1
    );
    const gap = contentGridGapClassName?.trim() || "gap-4";
    const alignClass = contentGridItemsAlign === "stretch" ? "items-stretch" : "items-start";
    const cellClass =
      contentGridItemsAlign === "stretch"
        ? "flex h-full min-h-0 min-w-0 flex-col"
        : "min-w-0";
    return (
      <div
        className={`grid w-full min-w-0 ${alignClass} ${colClass} ${gap} ${contentGridClassName ?? ""}`.trim()}
        data-test-id="collection-page-layout-grid"
      >
        {contentItems.map((item, i) => (
          <div key={i} className={cellClass} data-test-id={`collection-page-layout-cell-${i}`}>
            {item}
          </div>
        ))}
      </div>
    );
  }
  return children;
}

function CollectionPageLayoutView({
  title,
  subtitle,
  addAction,
  onAddClick,
  addButtonAriaLabel = "Añadir",
  showSearch = true,
  searchParamName = "search",
  searchLabel = "Buscar",
  searchPlaceholder = "",
  children,
  contentItems,
  contentGridColumns,
  contentGridGapClassName,
  contentGridClassName,
  contentEmptyMessage,
  contentGridItemsAlign,
  className = "",
  "data-test-id": dataTestId,
}: CollectionPageLayoutProps) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const paramValue = searchParams.get(searchParamName) ?? "";
  const [searchInput, setSearchInput] = useState(paramValue);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setSearchInput(paramValue);
  }, [paramValue]);

  const pushQuery = useCallback(
    (value: string) => {
      const next = new URLSearchParams(searchParams.toString());
      const trimmed = value.trim();
      if (trimmed) {
        next.set(searchParamName, trimmed);
      } else {
        next.delete(searchParamName);
      }
      const qs = next.toString();
      router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
    },
    [pathname, router, searchParamName, searchParams]
  );

  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const v = e.target.value;
    setSearchInput(v);
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      pushQuery(v);
      debounceRef.current = null;
    }, SEARCH_DEBOUNCE_MS);
  };

  const titleTrim = title?.trim() ?? "";
  const subtitleTrim = subtitle?.trim() ?? "";
  const hasCustomAdd = addAction !== undefined;
  const hasDefaultAdd = !hasCustomAdd && onAddClick != null;
  const hasAddSlot = hasCustomAdd || hasDefaultAdd;
  /** Misma idea que el header del DataGrid: fila con +, título y búsqueda (desktop). */
  const showTitleRow = hasAddSlot || Boolean(titleTrim) || showSearch;
  const showSubtitle = Boolean(subtitleTrim);
  const showHeaderBlock = showTitleRow || showSubtitle;

  return (
    <div
      className={`${layoutPageRootClassName} ${className}`.trim()}
      data-test-id={dataTestId ?? "collection-page-layout-root"}
    >
      {showHeaderBlock ? (
        <header className={layoutPageHeaderClassName} data-test-id="collection-page-layout-header">
          {showTitleRow ? (
            <>
              <div className="flex w-full items-center gap-2 py-0">
                {hasAddSlot ? (
                  <div className="flex shrink-0 items-center" data-test-id="collection-page-layout-add-wrap">
                    {hasCustomAdd ? (
                      addAction
                    ) : hasDefaultAdd ? (
                      <IconButton
                        icon="Plus"
                        variant="action"
                        size="md"
                        ariaLabel={addButtonAriaLabel}
                        onClick={onAddClick!}
                        data-test-id="collection-page-layout-add"
                      />
                    ) : null}
                  </div>
                ) : null}
                {titleTrim ? (
                  <h1 className={`whitespace-nowrap ${layoutPageTitleClassName}`}>
                    {titleTrim}
                  </h1>
                ) : null}
                <div className="min-w-0 flex-1" aria-hidden />
                {showSearch ? (
                  <div className="hidden items-center gap-2 sm:flex">
                    <TextField
                      label={searchLabel}
                      name={searchParamName}
                      value={searchInput}
                      onChange={handleSearchChange}
                      placeholder={searchPlaceholder}
                      startAdornment={
                        <Search className="h-4 w-4 shrink-0 text-secondary" aria-hidden />
                      }
                      className="w-full sm:w-64"
                      data-test-id="collection-page-layout-search"
                    />
                  </div>
                ) : null}
              </div>
              {showSearch ? (
                <div className="mt-0 flex items-start justify-end gap-2 sm:hidden">
                  <div className="flex min-w-0 max-w-xs flex-1 items-start">
                    <div className="flex w-full items-start gap-2">
                      <label htmlFor={`${searchParamName}-mobile`} className="sr-only">
                        {searchLabel}
                      </label>
                      <TextField
                        label="Buscar"
                        placeholder={searchPlaceholder}
                        name={`${searchParamName}-mobile`}
                        value={searchInput}
                        onChange={handleSearchChange}
                        startAdornment={
                          <Search className="h-4 w-4 shrink-0 text-secondary" aria-hidden />
                        }
                        className="w-full text-sm"
                        data-test-id="collection-page-layout-search-mobile"
                      />
                    </div>
                  </div>
                </div>
              ) : null}
            </>
          ) : null}
          {showSubtitle ? (
            <p className={layoutPageSubtitleClassName}>{subtitleTrim}</p>
          ) : null}
        </header>
      ) : null}

      <section
        className={layoutPageContentClassName}
        data-test-id="collection-page-layout-content"
      >
        {renderContentSection(
          contentItems,
          contentGridColumns,
          contentGridGapClassName,
          contentGridClassName,
          contentEmptyMessage,
          contentGridItemsAlign,
          children
        )}
      </section>
    </div>
  );
}

function CollectionPageLayoutFallback({
  title,
  subtitle,
  addAction,
  onAddClick,
  showSearch = true,
  children,
  contentItems,
  contentGridColumns,
  contentGridGapClassName,
  contentGridClassName,
  contentEmptyMessage,
  contentGridItemsAlign,
  className,
}: CollectionPageLayoutProps) {
  const titleTrim = title?.trim() ?? "";
  const subtitleTrim = subtitle?.trim() ?? "";
  const hasContentItems = contentItems != null;
  const hasGrid = hasContentItems && contentItems.length > 0;
  const isContentEmpty = hasContentItems && contentItems.length === 0;
  const hasCustomAdd = addAction !== undefined;
  const hasDefaultAdd = !hasCustomAdd && onAddClick != null;
  const hasAddSlot = hasCustomAdd || hasDefaultAdd;
  const showTitleRow = hasAddSlot || Boolean(titleTrim) || showSearch;
  const showSubtitle = Boolean(subtitleTrim);
  const showHeaderBlock = showTitleRow || showSubtitle;
  const emptyText = (contentEmptyMessage ?? DEFAULT_CONTENT_EMPTY_MESSAGE).trim() || DEFAULT_CONTENT_EMPTY_MESSAGE;
  const fallbackAlignClass = contentGridItemsAlign === "stretch" ? "items-stretch" : "items-start";
  return (
    <div className={`${layoutPageRootClassName} ${className ?? ""}`.trim()}>
      {showHeaderBlock ? (
        <header className={layoutPageHeaderClassName} data-test-id="collection-page-layout-header">
          {showTitleRow ? (
            <>
              <div className="flex w-full items-center gap-2 py-0">
                {hasAddSlot ? (
                  <div
                    className="h-10 w-10 shrink-0 animate-pulse rounded-md bg-neutral/60"
                    aria-hidden
                    data-test-id="collection-page-layout-add-skeleton"
                  />
                ) : null}
                {titleTrim ? (
                  <div
                    className="h-7 w-40 max-w-[60%] shrink-0 animate-pulse rounded-md bg-neutral/60"
                    aria-hidden
                  />
                ) : null}
                <div className="min-w-0 flex-1" aria-hidden />
                {showSearch ? (
                  <div
                    className="hidden h-10 w-64 max-w-[40%] shrink-0 animate-pulse rounded-md bg-neutral/60 sm:block"
                    aria-hidden
                  />
                ) : null}
              </div>
              {showSearch ? (
                <div
                  className="mt-1 h-10 w-full max-w-xs animate-pulse rounded-md bg-neutral/60 sm:hidden"
                  aria-hidden
                />
              ) : null}
            </>
          ) : null}
          {showSubtitle ? (
            <div className="mt-2 h-4 w-72 max-w-full animate-pulse rounded-md bg-neutral/60" aria-hidden />
          ) : null}
        </header>
      ) : null}
      <section className={layoutPageContentClassName}>
        {hasGrid && contentItems ? (
          <div
            className={`grid w-full min-w-0 ${fallbackAlignClass} ${buildContentGridClassNames(
              contentGridColumns ?? 1
            )} ${contentGridGapClassName?.trim() || "gap-4"} ${contentGridClassName ?? ""}`.trim()}
          >
            {contentItems.map((_, i) => (
              <div
                key={i}
                className="h-32 min-w-0 animate-pulse rounded-md bg-neutral/60"
                aria-hidden
                data-test-id={`collection-page-layout-cell-skeleton-${i}`}
              />
            ))}
          </div>
        ) : isContentEmpty ? (
          <div
            className="flex min-h-[12rem] w-full min-w-0 flex-col items-center justify-center py-10"
            data-test-id="collection-page-layout-empty"
          >
            <p className="text-center text-sm text-muted-foreground">{emptyText}</p>
          </div>
        ) : (
          children
        )}
      </section>
    </div>
  );
}

/**
 * Plantilla de página de colección/índice: cabecera alineada al DataGrid (fila `+` · título · búsqueda
 * a la derecha en `sm+`, búsqueda en fila aparte en móvil), subtítulo debajo, y búsqueda en query.
 * `addAction` sustituye el `IconButton` integrado. `contentItems` + `contentGridColumns` para la grilla.
 * Incluye `Suspense` por `useSearchParams`.
 */
export function CollectionPageLayout(props: CollectionPageLayoutProps) {
  return (
    <Suspense fallback={<CollectionPageLayoutFallback {...props} />}>
      <CollectionPageLayoutView {...props} />
    </Suspense>
  );
}

export default CollectionPageLayout;
