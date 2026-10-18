'use client'
import React from 'react';
import IconButton from '../../IconButton';
import type { DataGridColumn } from '../DataGrid';

interface RowActionsProps {
  row: any;
  column: DataGridColumn;
}

/**
 * Acciones de fila del DataGrid — referencia de norma (ver `.instructions/webadmin.instruction`):
 * - Cada `IconButton`: `variant="action"` y `size="sm"` (no usar `text` u otros con colores sueltos).
 * - En la columna, `headerName: ''` (título de encabezado vacío) y `filterable: false` si aplica.
 */
export const RowActions: React.FC<RowActionsProps> = ({ row, column: _column }) => {
  const handleEdit = () => {
    console.log('Editar fila:', row);
  };

  const handleDelete = () => {
    console.log('Eliminar fila:', row);
  };

  const handleView = () => {
    console.log('Ver detalles de fila:', row);
  };

  return (
    <div className="flex items-center gap-1 justify-center" data-test-id={`data-grid-row-actions-${row.id ?? row._id ?? row.key ?? row.index}`}>
      <IconButton
        icon="Eye"
        variant="action"
        size="sm"
        title="Ver detalles"
        ariaLabel="Ver detalles"
        onClick={handleView}
      />
      <IconButton
        icon="Pencil"
        variant="action"
        size="sm"
        title="Editar"
        ariaLabel="Editar"
        onClick={handleEdit}
      />
      <IconButton
        icon="Trash2"
        variant="action"
        size="sm"
        title="Eliminar"
        ariaLabel="Eliminar"
        onClick={handleDelete}
      />
    </div>
  );
};

// Ejemplo de uso en configuración de columnas:
/*
const columns: DataGridColumn[] = [
  { field: 'id', headerName: 'ID', width: 80 },
  { field: 'name', headerName: 'Nombre', flex: 1 },
  {
    field: 'actions',
    headerName: '', // norma: vacío
    width: 120,
    minWidth: 120,
    align: 'center',
    sortable: false,
    filterable: false,
    actionComponent: RowActions,
  },
];
*/

export default RowActions;
