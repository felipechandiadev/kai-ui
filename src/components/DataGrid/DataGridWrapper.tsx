'use client'
import React, { Suspense } from 'react';
import dynamic from 'next/dynamic';
import type { DataGridProps } from './DataGrid';
import DotProgress from '../DotProgress/DotProgress';

// Importar DataGrid dinámicamente para evitar SSR y hydration mismatches
const DataGrid = dynamic(() => import('./DataGrid'), {
  ssr: false,
  loading: () => <div className="flex items-center justify-center h-64"><DotProgress /></div>
});

const DataGridWrapper: React.FC<DataGridProps> = (props) => {
  return (
    <Suspense fallback={<div className="flex items-center justify-center h-64"><DotProgress /></div>}>
      <DataGrid {...props} />
    </Suspense>
  );
};

export default DataGridWrapper;
