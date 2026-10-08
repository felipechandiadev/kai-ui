'use client'
import React, { useMemo, useState } from 'react';
import {
  calculateColumnStyles,
  DATA_GRID_COLUMN_ROW_CLASS,
  getCellOverflowClassNames,
  resolveColumnCellOverflow,
  DataGridZIndex,
  DataGridCellMetrics,
  getColumnAlignClassNames,
  resolveColumnAlign,
  type ColumnStyle,
} from '../utils/columnStyles';
import type {
  DataGridColumn,
  DataGridRowAppearance,
  DataGridRowAppearanceContext,
} from '../DataGrid';
import { resolveRowCellBackgroundColor } from '../utils/rowAppearance';
import IconButton from '../../IconButton';
import dataGridChrome from '../dataGridChrome.module.css';

const CELL_BASE_CLASS =
  `${DataGridCellMetrics.paddingX} py-1 ${dataGridChrome.subtleLineBottom} text-xs flex min-w-0 box-border items-center`;

function cellOverflowClasses(column: DataGridColumn) {
  return getCellOverflowClassNames(resolveColumnCellOverflow(column));
}

interface BodyProps {
  columns?: DataGridColumn[];
  rows?: any[];
  filterMode?: boolean;
  /** Estilos de columna precalculados en DataGrid (misma fuente que ColHeader). */
  computedColumnStyles?: ColumnStyle[];
  screenWidth?: number;
  expandable?: boolean;
  expandedRowIds?: Set<string | number>;
  onToggleExpand?: (rowId: string | number) => void;
  expandableRowContent?: (row: any) => React.ReactNode;
  pinActionsColumn?: boolean;
  actionsColumnField?: string;
  /** Si está definido y la fila está expandida, la fila de datos queda `sticky` bajo el header al hacer scroll. */
  stickyExpandedRowTopPx?: number;
  onRowClick?: (row: any) => void;
  selectedRowId?: string | number | null;
  getRowAppearance?: (
    ctx: DataGridRowAppearanceContext,
  ) => DataGridRowAppearance | null | undefined;
}

const Body: React.FC<BodyProps> = ({
  columns = [],
  rows = [],
  filterMode = false,
  computedColumnStyles,
  screenWidth = 1024,
  expandable = false,
  expandedRowIds = new Set(),
  onToggleExpand,
  expandableRowContent,
  pinActionsColumn = false,
  actionsColumnField = 'actions',
  stickyExpandedRowTopPx,
  onRowClick,
  selectedRowId = null,
  getRowAppearance,
}) => {
  const [hoveredRowId, setHoveredRowId] = useState<string | number | null>(null);
  const visibleColumns = columns.filter((c) => !c.hide);

  // Usar utilidad centralizada para calcular estilos
  const computedStyles = useMemo(
    () => computedColumnStyles ?? calculateColumnStyles(columns, screenWidth),
    [computedColumnStyles, columns, screenWidth],
  );

  return (
    <div className="flex-1" data-test-id="data-grid-body">
      {/* Renderizar por filas para sincronizar alturas */}
      {rows.map((row, rowIndex) => {
        const rowId = row.id ?? rowIndex;
        const isExpanded = expandedRowIds.has(rowId);
        const isSelected =
          selectedRowId != null && selectedRowId !== '' && String(rowId) === String(selectedRowId);
        const rowAppearance = getRowAppearance?.({
          row,
          rowIndex,
          isSelected,
          isExpanded,
        });
        const isHovered = hoveredRowId === rowId;
        const rowBackgroundColor = resolveRowCellBackgroundColor({
          isSelected,
          isHovered,
          isExpanded,
          appearance: rowAppearance,
        });
        const rowAppearanceClassName = rowAppearance?.className?.trim() ?? '';
        /** Si solo hay clases Tailwind `bg-*`, no pisarlas con `backgroundColor` inline (hover/default).
            Excepción: una fila expandida siempre fuerza el fondo blanco inline. */
        const rowAppearanceUsesBgClass =
          !isExpanded &&
          Boolean(rowAppearanceClassName) &&
          /\bbg-/.test(rowAppearanceClassName) &&
          !rowAppearance?.backgroundColor;

        return (
          <React.Fragment key={rowId}>
            <div
              role={onRowClick ? 'button' : undefined}
              tabIndex={onRowClick ? 0 : undefined}
              onKeyDown={
                onRowClick
                  ? (e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        onRowClick(row);
                      }
                    }
                  : undefined
              }
              onClick={onRowClick ? () => onRowClick(row) : undefined}
              className={`${DATA_GRID_COLUMN_ROW_CLASS} items-stretch data-grid-row ${
                onRowClick ? 'cursor-pointer' : ''
              } ${
                expandable && isExpanded && stickyExpandedRowTopPx != null
                  ? `sticky ${dataGridChrome.subtleLineBottom} bg-background shadow-sm`
                  : ''
              }`}
              style={{
                ...(expandable && isExpanded && stickyExpandedRowTopPx != null
                  ? { top: stickyExpandedRowTopPx, zIndex: DataGridZIndex.expandedStickyRow }
                  : undefined),
              }}
              data-test-id="data-grid-row"
              data-row-appearance={rowAppearance?.variant}
              aria-selected={onRowClick ? isSelected : undefined}
            >
              {/* Expand/Collapse button */}
              {expandable && (
                <div
                  className={`w-10 min-w-[40px] px-1 py-1 ${dataGridChrome.subtleLineBottom} flex items-center justify-center ${rowAppearanceClassName}`}
                  style={
                    rowAppearanceUsesBgClass && !isSelected
                      ? undefined
                      : { backgroundColor: rowBackgroundColor }
                  }
                  onMouseEnter={() => setHoveredRowId(rowId)}
                  onMouseLeave={() => setHoveredRowId(null)}
                >
                  <IconButton
                    icon="ChevronDown"
                    variant="action"
                    size="sm"
                    onClick={() => onToggleExpand?.(rowId)}
                    className={`transition-transform duration-200 ${isExpanded ? 'rotate-180' : ''}`}
                    ariaLabel={isExpanded ? 'Colapsar fila' : 'Expandir fila'}
                  />
                </div>
              )}
              {visibleColumns.map((column, colIndex) => {
                // compute raw value from row field
                const rawValue = row[column.field];
                const style = computedStyles[colIndex];
                const isPinnedActionsColumn =
                  pinActionsColumn && column.field === actionsColumnField;

                const cellStyle = {
                  ...style,
                  ...(rowAppearanceUsesBgClass && !isSelected
                    ? {}
                    : { backgroundColor: rowBackgroundColor }),
                  ...(isPinnedActionsColumn
                    ? {
                        position: 'sticky' as const,
                        right: 0,
                        zIndex: DataGridZIndex.bodyPinnedCell,
                        flex:
                          typeof column.width === 'number'
                            ? `0 0 ${column.width}px`
                            : '0 0 auto',
                        width: column.width,
                        minWidth: column.minWidth ?? column.width,
                        maxWidth: column.maxWidth ?? column.width,
                      }
                    : {}),
                };

                // if a valueGetter is provided, use it to derive the value
                const value = column.valueGetter
                  ? column.valueGetter({ row, value: rawValue, column, rowIndex })
                  : rawValue;

                const alignClasses = getColumnAlignClassNames(
                  resolveColumnAlign(column, 'body'),
                );
                const overflow = cellOverflowClasses(column);
                const overflowMode = resolveColumnCellOverflow(column);
                const cellClassName = `${CELL_BASE_CLASS} ${alignClasses.cell} ${
                  overflowMode === 'wrap' ? 'items-start' : 'items-center'
                } ${overflow.cell} ${rowAppearanceClassName}`.trim();
                const displayText =
                  value !== null && value !== undefined ? String(value) : '-';
                const cellTitle =
                  resolveColumnCellOverflow(column) === 'truncate' && displayText !== '-'
                    ? displayText
                    : undefined;

                // Renderizar actionComponent si existe
                if (column.actionComponent) {
                  const ActionComponent = column.actionComponent;
                  return (
                    <div
                      key={`${column.field}-${rowId}`}
                      className={cellClassName}
                      style={cellStyle}
                      onMouseEnter={() => setHoveredRowId(rowId)}
                      onMouseLeave={() => setHoveredRowId(null)}
                    >
                      <div className={`${overflow.content} ${alignClasses.content}`}>
                        <ActionComponent row={row} column={column} />
                      </div>
                    </div>
                  );
                }

                // Usar renderCell personalizado si existe
                if (column.renderCell) {
                  return (
                    <div
                      key={`${column.field}-${rowId}`}
                      className={cellClassName}
                      style={cellStyle}
                      onMouseEnter={() => setHoveredRowId(rowId)}
                      onMouseLeave={() => setHoveredRowId(null)}
                    >
                      <div className={`${overflow.content} ${alignClasses.content}`}>
                        {column.renderCell({ row, value, column })}
                      </div>
                    </div>
                  );
                }

                return (
                  <div
                    key={`${column.field}-${rowId}`}
                    className={cellClassName}
                    style={cellStyle}
                    onMouseEnter={() => setHoveredRowId(rowId)}
                    onMouseLeave={() => setHoveredRowId(null)}
                  >
                    <span className={`${overflow.content} ${alignClasses.content}`} title={cellTitle}>
                      {displayText}
                    </span>
                  </div>
                );
              })}
            </div>
            {/* Expanded content panel */}
            {expandable && isExpanded && expandableRowContent && (
              <div
                className={`sticky left-0 w-full min-w-0 max-w-full overflow-x-hidden ${dataGridChrome.subtleLineBottom} bg-neutral/50`}
                style={{ zIndex: DataGridZIndex.expandedContent }}
                data-test-id="data-grid-expanded-row"
              >
                <div className="box-border w-full min-w-0 max-w-full overflow-x-hidden p-4">
                  {expandableRowContent(row)}
                </div>
              </div>
            )}
          </React.Fragment>
        );
      })}
    </div>
  );
};

export default Body;
