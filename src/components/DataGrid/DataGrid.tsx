'use client'
import React from 'react';
import Header from './components/Header';
import dataGridChrome from './dataGridChrome.module.css';
import Body from './components/Body';
import Footer from './components/Footer';
import { ColHeader } from './components/ColHeader';
import {
  calculateColumnStyles,
  DataGridStyles,
  useScreenSize,
  type ColumnStyle,
  type DataGridCellOverflow,
} from './utils/columnStyles';

export type { DataGridCellOverflow };
import {
  DATA_GRID_TAB_LAYOUT_FALLBACK_EXTRA_PX,
  dataGridFillViewportFallbackHeight,
  useDataGridFillViewportHeight,
} from './utils/useDataGridFillViewportHeight';
import { useState, useEffect, useRef, useLayoutEffect, useMemo } from 'react';
import { useSearchParams, useRouter } from '../../navigation';
import type { DataGridPaginationChange } from './components/Pagination';

export type DataGridColumnType =
  | 'string'
  | 'number'
  | 'date'
  | 'dateTime'
  | 'boolean'
  | 'id';

export interface DataGridColumn {
  field: string;
  headerName: string;
  width?: number;
  minWidth?: number;
  maxWidth?: number;
  flex?: number;
  type?: DataGridColumnType;
  sortable?: boolean;
  editable?: boolean;
  filterable?: boolean; // Nueva propiedad para controlar si la columna es filtrable
  // Use serializable render hints instead of passing functions from server
  renderCell?: (params: any) => React.ReactNode;
  renderType?: 'currency' | 'badge' | 'dateString';
  valueGetter?: (params: any) => any;
  align?: 'left' | 'right' | 'center';
  headerAlign?: 'left' | 'right' | 'center';
  hide?: boolean;
  /**
   * Contenido largo en la celda: `truncate` (ellipsis, default), `wrap`, `clip` u `visible`.
   * Aplica a texto por defecto, `renderCell` y `actionComponent` (contenedor con `min-w-0`).
   */
  cellOverflow?: DataGridCellOverflow;
  sticky?: boolean; // Fijar columna al lado derecho (compatibilidad hacia atrás)
  /**
   * Acciones de fila. Norma: columna con `field` típ. `'actions'`, `headerName: ''`, `filterable: false`, `sortable: false`;
   * en el componente, IconButtons con `variant="action"` y `size="sm"`.
   */
  actionComponent?: React.ComponentType<{ row: any; column: DataGridColumn }>;
}

/** Variante del botón de alta en el header del DataGrid. */
export type DataGridAddButtonVariant = 'icon' | 'pillOutlined';

export type DataGridRowAppearance = {
  /** Color de fondo de la fila (celdas + columna expand). CSS válido. */
  backgroundColor?: string;
  /** Clases extra en cada celda de la fila. */
  className?: string;
  /** Slug en `data-row-appearance` (tests / analytics). */
  variant?: string;
};

export type DataGridRowAppearanceContext = {
  row: unknown;
  rowIndex: number;
  isSelected: boolean;
  isExpanded: boolean;
};

export interface DataGridProps {
  columns: DataGridColumn[];
  title?: string;
  rows?: any[];
  /** Estado de carga (server-side). En `paginationMode="controlled"` controla el skeleton del cuerpo. */
  loading?: boolean;
  enableSearch?: boolean; // Deprecated: usar showSearch en su lugar
  enablePagination?: boolean; // Deprecated: siempre habilitado
  sort?: 'asc' | 'desc';
  sortField?: string;
  search?: string;
  filters?: string;
  height?: number | string;
  /**
   * Ajusta la altura del grid para que el pie quede al borde inferior del viewport;
   * el cuerpo (filas) hace scroll dentro del espacio restante. Ignora `height` si está activo.
   */
  fillViewport?: boolean;
  /**
   * Grid bajo {@link TabPageLayout} (fila de pestañas encima): el fallback CSS resta altura extra
   * hasta medir; la altura en px sigue usando el `top` real del contenedor.
   */
  fillViewportInTabLayout?: boolean;
  /** Margen inferior en px al calcular `fillViewport` (p. ej. `pb-6` del `<main>`). Default 24. */
  viewportBottomInset?: number;
  totalRows?: number;
  totalGeneral?: number;
  createForm?: React.ReactNode;
  createFormTitle?: string;
  onAddClick?: () => void; // Callback para el botón + (abre diálogo externo)
  addDisabled?: boolean; // Deshabilita el botón + sin ocultarlo
  /** `icon` (default): IconButton +. `pillOutlined`: ButtonPill outlined con icono + y texto (p. ej. "Recepción"). */
  addButtonVariant?: DataGridAddButtonVariant;
  /** Texto del pill outlined (el icono + va aparte, sin prefijo en el texto). */
  addButtonLabel?: string;
  ["data-test-id"]?: string;
  excelUrl?: string; // Absolute URL for Excel export endpoint
  excelFields?: string;
  limit?: number;
  onExportExcel?: () => Promise<void>; // Callback para exportar a Excel
  showBorder?: boolean;
  showSortButton?: boolean;
  showFilterButton?: boolean;
  showExportButton?: boolean;
  showSearch?: boolean;
  onSearchChange?: (value: string) => void;
  // Expandable rows
  expandable?: boolean; // Habilita filas expandibles
  expandableRowContent?: (row: any) => React.ReactNode; // Contenido del panel expandido
  defaultExpandedRowIds?: (string | number)[]; // IDs de filas expandidas por defecto
  // Header slots
  headerActions?: React.ReactNode; // Componentes adicionales en el header (ej: filtros externos)
  // Sticky actions column
  pinActionsColumn?: boolean;
  actionsColumnField?: string;
  /** Clic en la fila (p. ej. cambiar selección vía URL). */
  onRowClick?: (row: any) => void;
  /** Resalta la fila cuyo `row.id` coincide (selección controlada desde fuera). */
  selectedRowId?: string | number | null;
  /**
   * Apariencia condicional por fila (fondo, clases, variant para tests).
   * Prioridad de fondo: selección > apariencia custom > hover > default.
   */
  getRowAppearance?: (
    ctx: DataGridRowAppearanceContext,
  ) => DataGridRowAppearance | null | undefined;
  /** Oculta pie de paginación (listas cortas estáticas). */
  showFooter?: boolean;
  /**
   * `url` (default): `page` y `limit` en la URL de la página.
   * `controlled`: paginación vía props (diálogos, paneles embebidos).
   */
  paginationMode?: 'url' | 'controlled';
  page?: number;
  onPaginationChange?: (next: DataGridPaginationChange) => void;
}

const DataGrid: React.FC<DataGridProps> = ({
  columns,
  title,
  rows,
  loading = false,
  enableSearch, // deprecated: map to showSearch
  enablePagination, // deprecated: always enabled
  sort,
  sortField,
  search,
  filters,
  height = '70vh',
  fillViewport = false,
  fillViewportInTabLayout = false,
  viewportBottomInset = 24,
  totalRows,
  totalGeneral,
  createForm,
  createFormTitle,
  onAddClick,
  addDisabled,
  addButtonVariant = 'icon',
  addButtonLabel,
  ["data-test-id"]: dataTestId,
  excelUrl,
  excelFields,
  limit = 25,
  onExportExcel,
  showBorder = false,
  showSortButton = true,
  showFilterButton = true,
  showExportButton = true,
  showSearch,
  onSearchChange,
  expandable = false,
  expandableRowContent,
  defaultExpandedRowIds = [],
  headerActions,
  pinActionsColumn = false,
  actionsColumnField = 'actions',
  onRowClick,
  selectedRowId = null,
  getRowAppearance,
  showFooter = true,
  paginationMode = 'url',
  page: controlledPage,
  onPaginationChange,
}) => {
  // Map deprecated props to new ones
  const effectiveShowSearch = showSearch !== undefined ? showSearch : (enableSearch !== undefined ? enableSearch : true);
  const searchParams = useSearchParams();
  const router = useRouter();
  const [data, setData] = useState<any[]>(rows || []);
  const [total, setTotal] = useState(totalRows || (rows ? rows.length : 0));
  const [expandedRowIds, setExpandedRowIds] = useState<Set<string | number>>(new Set(defaultExpandedRowIds));

  useEffect(() => {
    if (defaultExpandedRowIds.length === 0) {
      return;
    }
    setExpandedRowIds(new Set(defaultExpandedRowIds));
  }, [defaultExpandedRowIds.join("|")]);
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const columnHeaderRowRef = useRef<HTMLDivElement>(null);
  /** Wrapper (sin scrollbar) de la fila de encabezados; su scrollLeft se sincroniza con el body. */
  const columnHeaderScrollRef = useRef<HTMLDivElement>(null);
  const fillViewportHeightPx = useDataGridFillViewportHeight(
    fillViewport,
    containerRef,
    viewportBottomInset,
  );
  const [gridWidth, setGridWidth] = useState(1024);

  useLayoutEffect(() => {
    const scrollEl = scrollAreaRef.current;
    if (!scrollEl) {
      return;
    }

    const measureGridWidth = () => {
      const width = scrollEl.clientWidth;
      if (width > 0) {
        setGridWidth(width);
      }
    };

    measureGridWidth();
    const gridWidthObserver = new ResizeObserver(() => {
      requestAnimationFrame(measureGridWidth);
    });
    gridWidthObserver.observe(scrollEl);

    return () => {
      gridWidthObserver.disconnect();
    };
  }, []);

  // Inicializar filterMode basado en si hay filtros activos en la URL
  const [filterMode, setFilterMode] = useState(() => {
    const filtration = searchParams.get('filtration') === 'true';
    return filtration;
  });

  // Hook para detectar tamaño de pantalla
  const { width: screenWidth, isMobile } = useScreenSize();

  const toggleFilterMode = () => setFilterMode((v) => !v);

  // Toggle expandir/colapsar una fila
  const toggleRowExpanded = (rowId: string | number) => {
    setExpandedRowIds(prev => {
      const newSet = new Set(prev);
      if (newSet.has(rowId)) {
        newSet.delete(rowId);
      } else {
        newSet.add(rowId);
      }
      return newSet;
    });
  };

  // Update data when rows prop changes (server-side updates)
  useEffect(() => {
    setData(rows || []);
    setTotal(totalRows || (rows ? rows.length : 0));
  }, [rows, totalRows]);

  // Sincronizar filterMode con la URL
  useEffect(() => {
    const filtration = searchParams.get('filtration') === 'true';
    setFilterMode(filtration);
  }, [searchParams]);

  // Inicializar limit en la URL si no está presente
  useEffect(() => {
    if (paginationMode === 'controlled') {
      return;
    }
    const currentLimit = searchParams.get('limit');
    if (!currentLimit) {
      const params = new URLSearchParams(searchParams.toString());
      params.set('limit', limit.toString());
      router.replace(`?${params.toString()}`, { scroll: false });
    }
  }, [searchParams, limit, router, paginationMode]);

  const containerClasses = [
    DataGridStyles.container,
    DataGridStyles.responsive.minWidth,
    fillViewport ? 'min-h-0' : '',
    showBorder ? 'border border-border' : '',
  ]
    .filter(Boolean)
    .join(' ');
  const visibleColumns = columns.filter((c) => !c.hide);
  const computedStyles = useMemo(
    () => calculateColumnStyles(columns, gridWidth),
    [columns, gridWidth],
  );

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
      height: typeof height === 'number' ? `${height}px` : height,
    };
  }, [
    fillViewport,
    fillViewportHeightPx,
    height,
    viewportBottomInset,
    tabLayoutFallbackExtra,
  ]);

  return (
    <div
      ref={containerRef}
      className={containerClasses}
      style={containerStyle}
      data-test-id={dataTestId || "data-grid-root"}
    >
      <div className="shrink-0">
        <Header
        title={title} 
        filterMode={filterMode} 
        onToggleFilterMode={toggleFilterMode}
        columns={columns}
        createForm={createForm}
        createFormTitle={createFormTitle}
        onAddClick={onAddClick}
        addDisabled={addDisabled}
        addButtonVariant={addButtonVariant}
        addButtonLabel={addButtonLabel}
        screenWidth={screenWidth}
        onExportExcel={onExportExcel}
        headerActions={headerActions}
        showSortButton={showSortButton}
        showFilterButton={showFilterButton}
        showExportButton={showExportButton}
        showSearch={effectiveShowSearch}
        onSearchChange={onSearchChange}
        />
      </div>
      {/* Fila de encabezados de columna: FUERA del área con scroll del body.
          Queda fija arriba; el scroll horizontal se sincroniza con el body (onScroll). */}
      <div ref={columnHeaderScrollRef} className="shrink-0 overflow-hidden">
        <div
          ref={columnHeaderRowRef}
          className={DataGridStyles.headerRow}
        >
          {/* Expand column header placeholder */}
          {expandable && (
            <div
              className={`${dataGridChrome.subtleLineBottom} w-10 min-w-[40px] shrink-0 px-1`}
              style={{
                height: '56px',
                minHeight: '56px',
                maxHeight: '56px',
              }}
            />
          )}
          {visibleColumns.map((column, i) => {
            const style = computedStyles[i];
            const isPinnedActionsColumn =
              pinActionsColumn && column.field === actionsColumnField;

            return (
              <ColHeader
                key={column.field}
                column={column}
                computedStyle={style}
                filterMode={filterMode}
                isPinned={isPinnedActionsColumn}
                sortingEnabled={showSortButton}
              />
            );
          })}
        </div>
      </div>
      {/* Cuerpo: único contenedor con scroll (vertical y horizontal). */}
      <div
        ref={scrollAreaRef}
        className={`${DataGridStyles.scrollContainer} relative`}
        onScroll={(e) => {
          const headerEl = columnHeaderScrollRef.current;
          if (headerEl) {
            headerEl.scrollLeft = (e.currentTarget as HTMLDivElement).scrollLeft;
          }
        }}
      >
        <Body 
          columns={columns} 
          rows={loading ? [] : data}
          filterMode={filterMode}
          computedColumnStyles={computedStyles}
          expandable={expandable}
          expandedRowIds={expandedRowIds}
          onToggleExpand={toggleRowExpanded}
          expandableRowContent={expandableRowContent}
          pinActionsColumn={pinActionsColumn}
          actionsColumnField={actionsColumnField}
          stickyExpandedRowTopPx={expandable ? 0 : undefined}
          onRowClick={onRowClick}
          selectedRowId={selectedRowId}
          getRowAppearance={getRowAppearance}
        />
      </div>
      {showFooter ? (
        <div className="shrink-0">
          <Footer
            total={total}
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
};

export default DataGrid;
