'use client'
import React from 'react';
import Pagination, { type DataGridPaginationChange } from './Pagination';
import { useScreenSize } from '../utils/columnStyles';
import dataGridChrome from '../dataGridChrome.module.css';
import { Select, type Option as SelectOption } from '../../Select';
import { useSearchParams, useRouter } from '../../../navigation';

interface FooterProps {
  total?: number;
  totalGeneral?: number;
  paginationMode?: 'url' | 'controlled';
  page?: number;
  limit?: number;
  onPaginationChange?: (next: DataGridPaginationChange) => void;
}

const Footer: React.FC<FooterProps> = ({
  total = 0,
  totalGeneral,
  paginationMode = 'url',
  page: controlledPage,
  limit: controlledLimit,
  onPaginationChange,
}) => {
  const { isMobile } = useScreenSize();

  const searchParams = useSearchParams();
  const router = useRouter();
  const page =
    paginationMode === 'controlled'
      ? Math.max(1, controlledPage ?? 1)
      : parseInt(searchParams.get('page') || '1', 10);
  const limit =
    paginationMode === 'controlled'
      ? Math.max(1, controlledLimit ?? 25)
      : parseInt(searchParams.get('limit') || '25', 10);

  const updateSearchParams = (newParams: Record<string, string>) => {
    const params = new URLSearchParams(searchParams.toString());
    Object.entries(newParams).forEach(([key, value]) => {
      params.set(key, value);
    });
    router.push(`?${params.toString()}`);
  };

  const handleLimitChange = (newLimit: number) => {
    if (paginationMode === 'controlled') {
      onPaginationChange?.({ page: 1, limit: newLimit });
      return;
    }
    updateSearchParams({ limit: newLimit.toString(), page: '1' });
  };

  const paginationProps = {
    total,
    totalGeneral,
    paginationMode,
    page: controlledPage,
    limit: controlledLimit,
    onPaginationChange,
  };

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

  if (isMobile) {
    return (
      <div className={`flex min-h-0 flex-col gap-2 p-0 py-2 ${dataGridChrome.subtleLineTop}`} data-test-id="data-grid-footer">
        {/* Fila superior: Paginación */}
        <div className="flex justify-center">
          <Pagination {...paginationProps} mobileMode={true} />
        </div>
        {/* Fila inferior: Selector de filas y recuento */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3 flex-shrink-0">
            <span className="text-xs text-foreground whitespace-nowrap">Filas por página:</span>
            <Select
              options={limitOptions}
              placeholder=""
              value={limit}
              onChange={(newLimit: string | number | null) => newLimit && handleLimitChange(Number(newLimit))}
              variant="minimal"
              className="min-w-[112px]"
            />
          </div>
          <div className="text-xs text-muted-foreground">
            {totalGeneral && totalGeneral !== total ? `Filtrados: ${total} de ${totalGeneral}` : `Total: ${total}`}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={`p-0 ${dataGridChrome.subtleLineTop}`} data-test-id="data-grid-footer">
      <Pagination {...paginationProps} />
    </div>
  );
};

export default Footer;
