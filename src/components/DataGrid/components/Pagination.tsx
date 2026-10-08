"use client"
import React from 'react'
import { useSearchParams, useRouter } from '../../../navigation'
import IconButton from '../../IconButton';
import { Select, type Option as SelectOption } from '../../Select';

export type DataGridPaginationChange = { page: number; limit: number };

interface PaginationProps {
  total: number
  totalGeneral?: number
  mobileMode?: boolean // Nuevo prop para modo móvil
  /** `url` (default): sincroniza con query string. `controlled`: usa props. */
  paginationMode?: 'url' | 'controlled'
  page?: number
  limit?: number
  onPaginationChange?: (next: DataGridPaginationChange) => void
}

const Pagination: React.FC<PaginationProps> = ({
  total,
  totalGeneral,
  mobileMode = false,
  paginationMode = 'url',
  page: controlledPage,
  limit: controlledLimit,
  onPaginationChange,
}) => {
  const searchParams = useSearchParams()
  const router = useRouter()
  
  const page =
    paginationMode === 'controlled'
      ? Math.max(1, controlledPage ?? 1)
      : parseInt(searchParams.get('page') || '1', 10)
  const limit =
    paginationMode === 'controlled'
      ? Math.max(1, controlledLimit ?? 25)
      : parseInt(searchParams.get('limit') || '25', 10)
  
  const totalPages = Math.max(1, Math.ceil(total / limit))

  // Funciones para actualizar la URL
  const updateSearchParams = (newParams: Record<string, string>) => {
    const params = new URLSearchParams(searchParams.toString())
    Object.entries(newParams).forEach(([key, value]) => {
      params.set(key, value)
    })
    router.push(`?${params.toString()}`)
  }

  const handleLimitChange = (newLimit: number) => {
    if (paginationMode === 'controlled') {
      onPaginationChange?.({ page: 1, limit: newLimit })
      return
    }
    updateSearchParams({ limit: newLimit.toString(), page: '1' }) // Reset to page 1 when changing limit
  }

  const handlePageChange = (newPage: number) => {
    if (paginationMode === 'controlled') {
      onPaginationChange?.({ page: newPage, limit })
      return
    }
    updateSearchParams({ page: newPage.toString() })
  }

  // Opciones fijas para el paginador
  const limitOptions: SelectOption[] = [
    { id: '5', label: '5' },
    { id: '10', label: '10' },
    { id: '25', label: '25' },
    { id: '50', label: '50' },
    { id: '75', label: '75' },
    { id: '100', label: '100' },
    { id: '200', label: '200' },
    { id: '300', label: '300' },
    { id: '500', label: '500' }
  ];

  if (mobileMode) {
    return (
      <div className="flex items-center gap-2 text-xs text-foreground" data-test-id="data-grid-pagination">
        <IconButton icon="ChevronsLeft" variant="action" size="sm" className="p-1 cursor-pointer" onClick={() => handlePageChange(1)} aria-label="Primera página" />
        <IconButton icon="ChevronLeft" variant="action" size="sm" className="p-1 cursor-pointer" onClick={() => handlePageChange(Math.max(1, page - 1))} aria-label="Anterior" />
        <div className="px-3 py-1 text-xs font-normal text-foreground w-16 text-center">
          {page} <span className="text-muted-foreground">/ {totalPages}</span>
        </div>
        <IconButton icon="ChevronRight" variant="action" size="sm" className="p-1 cursor-pointer" onClick={() => handlePageChange(Math.min(totalPages, page + 1))} aria-label="Siguiente" />
        <IconButton icon="ChevronsRight" variant="action" size="sm" className="p-1 cursor-pointer" onClick={() => handlePageChange(totalPages)} aria-label="Última página" />
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-0 py-1" data-test-id="data-grid-pagination">
      {/* Izquierda: Filas por página */}
      <div className="flex items-center gap-3 flex-shrink-0">
        <span className="text-xs text-foreground font-normal whitespace-nowrap">Filas por página:</span>
        <Select
          options={limitOptions}
          placeholder=""
          value={limit}
          onChange={(newLimit: string | number | null) => newLimit && handleLimitChange(Number(newLimit))}
            variant="minimal"
            className="min-w-[112px]"
        />
      </div>

      {/* Centro: Leyenda de total/filtrados */}
      <div className="flex-1 text-center">
        <div className="text-xs text-muted-foreground">
          {totalGeneral && totalGeneral !== total ? `Registros filtrados: ${total} de ${totalGeneral}` : `Registros totales: ${total}`}
        </div>
      </div>

      {/* Derecha: Información de paginación y botones */}
      <div className="flex items-center gap-2 text-xs text-foreground">
        <IconButton icon="ChevronsLeft" variant="action" size="sm" className="p-1 cursor-pointer" onClick={() => handlePageChange(1)} aria-label="Primera página" />
        <IconButton icon="ChevronLeft" variant="action" size="sm" className="p-1 cursor-pointer" onClick={() => handlePageChange(Math.max(1, page - 1))} aria-label="Anterior" />
        <div className="px-3 py-1 text-xs font-normal text-foreground w-16 text-center">
          {page} <span className="text-muted-foreground">/ {totalPages}</span>
        </div>
        <IconButton icon="ChevronRight" variant="action" size="sm" className="p-1 cursor-pointer" onClick={() => handlePageChange(Math.min(totalPages, page + 1))} aria-label="Siguiente" />
        <IconButton icon="ChevronsRight" variant="action" size="sm" className="p-1 cursor-pointer" onClick={() => handlePageChange(totalPages)} aria-label="Última página" />
      </div>
    </div>
  )
}

export default Pagination
