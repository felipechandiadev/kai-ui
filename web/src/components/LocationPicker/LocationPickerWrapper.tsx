'use client';

import dynamic from 'next/dynamic';
import { LoadingState } from '../LoadingState';

const LocationPickerComponent = dynamic(() => import('./LocationPicker'), {
  ssr: false,
  loading: () => (
    <LoadingState className="flex items-center justify-center p-4" label="Cargando mapa" />
  ),
});

export default LocationPickerComponent;

