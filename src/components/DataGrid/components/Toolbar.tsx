'use client'
import React from 'react';
import IconButton from '../../IconButton';
import { useRouter, useSearchParams } from '../../../navigation';
import type { DataGridColumn } from '../DataGrid';
import './toolbar.css';
// TODO: Create shared/hooks/useAlert hook
// import { useAlert } from '@/shared/hooks/useAlert';

interface ToolbarProps {
  filterMode?: boolean;
  onToggleFilterMode?: () => void;
  columns?: DataGridColumn[];
  title?: string;
  onExportExcel?: () => Promise<void>; // Callback para exportar a Excel
  showSortButton?: boolean;
  showFilterButton?: boolean;
  showExportButton?: boolean;
}

const Toolbar: React.FC<ToolbarProps> = ({
  filterMode = false,
  onToggleFilterMode,
  columns = [],
  title = '',
  onExportExcel,
  showSortButton = true,
  showFilterButton = true,
  showExportButton = true,
}) => {
  const router = useRouter();
  const searchParams = useSearchParams();
  // TODO: Implement useAlert hook
  // const { showAlert } = useAlert();

  // Determine active sort from URL
  const activeSortField = searchParams.get('sortField');
  const activeSortDir = searchParams.get('sort');

  // First visible column field
  const firstVisible = columns.find((c) => !c.hide)?.field;

  const handleQuickSort = () => {
    if (!firstVisible) return;
    const params = new URLSearchParams(searchParams.toString());
    if (isSortActive) {
      // If already active, remove sorting params
      params.delete('sort');
      params.delete('sortField');
    } else {
      // Activate sort on first visible column
      params.set('sort', 'asc');
      params.set('sortField', firstVisible);
      params.set('page', '1');
    }
    router.replace(`?${params.toString()}`);
  };

  const handleExportExcel = async () => {
    if (onExportExcel) {
      try {
        await onExportExcel();
      } catch (error) {
        console.error('Error exporting to Excel:', error);
        // TODO: uncomment when useAlert hook is implemented
        // showAlert({ message: 'Error al exportar a Excel', type: 'error', duration: 4000 });
      }
    } else {
      // TODO: uncomment when useAlert hook is implemented
      // showAlert({ message: 'Exportación no disponible', type: 'warning', duration: 4000 });
    }
  };

  // UI highlight: show active when ANY sort is present
  const isSortActive = Boolean(activeSortField && activeSortDir);

  // UI highlight: show active when filter mode on OR query has filters
  const isFilterActive =
    filterMode || searchParams.get('filtration') === 'true' || searchParams.has('filters');

  return (
    <div className="flex items-center justify-end gap-2 py-0" data-test-id="data-grid-toolbar">
      {/* Quick sort button: sets sort=asc and sortField=first visible column */}
      {showSortButton && firstVisible ? (
        <IconButton
          variant="action"
          size="sm"
          title="Ordenar por primer campo (asc)"
          onClick={handleQuickSort}
          icon="ArrowUpDown"
          className={isSortActive ? 'fs-data-grid-toolbar__icon--active' : 'fs-data-grid-toolbar__icon--inactive'}
          style={{ fontSize: 20, width: 20, height: 20, display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}
        />
      ) : null}

      {/* Material Symbols filter icon - cambia según filterMode */}
      {showFilterButton ? (
        <IconButton
          variant="action"
          size="sm"
          title={filterMode ? 'Desactivar filtros' : 'Filtrar'}
          onClick={() => {
            const params = new URLSearchParams(searchParams.toString());
            if (filterMode) {
              // Clear filters when deactivating
              params.delete('filters');
              params.delete('filtration');
              router.replace(`?${params.toString()}`);
            } else {
              // Activate filtration when enabling filter mode
              params.set('filtration', 'true');
              router.replace(`?${params.toString()}`);
            }
            onToggleFilterMode?.();
          }}
          icon={filterMode ? 'FilterX' : 'Filter'}
          className={isFilterActive ? 'fs-data-grid-toolbar__icon--active' : 'fs-data-grid-toolbar__icon--inactive'}
          style={{ fontSize: 20, width: 20, height: 20, display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}
        />
      ) : null}
      {/* Excel export icon - using Material Symbol for perfect alignment */}
      {showExportButton ? (
        <IconButton
          variant="action"
          size="sm"
          title="Exportar a Excel"
          onClick={handleExportExcel}
          icon="Download"
          style={{ fontSize: 20, width: 20, height: 20, display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}
        />
      ) : null}
    </div>
  );
};

export default Toolbar;
