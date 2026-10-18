'use client';
import React, { useState, useEffect, useRef, useMemo, useCallback, memo } from 'react';
import dynamic from 'next/dynamic';
import { AlertCircle } from 'lucide-react';

import 'leaflet/dist/leaflet.css';

const MapContainer = dynamic(() => import('react-leaflet').then((mod) => mod.MapContainer), {
  ssr: false,
});
const TileLayer = dynamic(() => import('react-leaflet').then((mod) => mod.TileLayer), { ssr: false });
const Marker = dynamic(() => import('react-leaflet').then((mod) => mod.Marker), { ssr: false });

import './location-picker.css';

declare const L: any;

const MapContainerAny: any = MapContainer;
const TileLayerAny: any = TileLayer;
const MarkerAny: any = Marker;

let customIcon: any = null;
let draggingIcon: any = null;

if (typeof window !== 'undefined') {
  import('leaflet').then((L) => {
    delete (L.Icon.Default.prototype as any)._getIconUrl;
    L.Icon.Default.mergeOptions({
      iconRetinaUrl:
        'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.7.1/images/marker-icon-2x.png',
      iconUrl: 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.7.1/images/marker-icon.png',
      shadowUrl: '',
    });

    customIcon = new L.Icon({
      iconUrl: 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.7.1/images/marker-icon.png',
      iconRetinaUrl:
        'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.7.1/images/marker-icon-2x.png',
      shadowUrl: '',
      iconSize: [25, 41],
      iconAnchor: [12, 41],
      popupAnchor: [1, -34],
      shadowSize: [0, 0],
    });

    draggingIcon = new L.Icon({
      iconUrl: 'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.7.1/images/marker-icon.png',
      iconRetinaUrl:
        'https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.7.1/images/marker-icon-2x.png',
      shadowUrl: '',
      iconSize: [30, 49],
      iconAnchor: [15, 49],
      popupAnchor: [1, -34],
      shadowSize: [0, 0],
    });
  });
}

type LocationPickerVariant = 'default' | 'flat' | 'borderless';
type LocationPickerRounded = 'none' | 'sm' | 'md' | 'lg' | 'full';
type LocationPickerMode = 'viewer' | 'edit' | 'update';

export interface LocationPickerProps {
  onChange?: (coordinates: { lat: number; lng: number } | null) => void;
  initialLat?: number;
  initialLng?: number;
  variant?: LocationPickerVariant;
  rounded?: LocationPickerRounded;
  className?: string;
  draggable?: boolean;
  mode?: LocationPickerMode;
  zoom?: number;
  height?: number;
  fillContainer?: boolean;
  externalPosition?: { lat: number; lng: number };
  recenterOnExternalChange?: boolean;
}

const roundedClasses: Record<LocationPickerRounded, string> = {
  none: 'rounded-none',
  sm: 'rounded-sm',
  md: 'rounded-md',
  lg: 'rounded-lg',
  full: 'rounded-xl',
};

const variantClasses: Record<LocationPickerVariant, string> = {
  default: 'border border-border',
  flat: '',
  borderless: '',
};

const DraggableMarker = memo(
  function DraggableMarker({
    position,
    draggable,
    onDragEnd,
  }: {
    position: { lat: number; lng: number };
    draggable: boolean;
    onDragEnd: (newPos: { lat: number; lng: number }) => void;
  }) {
    const onDragEndRef = useRef(onDragEnd);
    onDragEndRef.current = onDragEnd;

    const eventHandlers = useMemo(
      () => ({
        dragstart: () => {
          if (draggingIcon) {
            // Leaflet will swap icon by instance.
          }
        },
        dragend: (e: { target: L.Marker }) => {
          const marker = e.target;
          if (customIcon) marker.setIcon(customIcon);
          const latlng = marker.getLatLng();
          onDragEndRef.current({ lat: latlng.lat, lng: latlng.lng });
        },
      }),
      [],
    );

    return (
      <MarkerAny
        position={[position.lat, position.lng]}
        draggable={draggable}
        eventHandlers={eventHandlers}
      />
    );
  },
);

function MapController({
  positionToCenter,
  zoom,
  hasCenteredRef,
}: {
  positionToCenter: { lat: number; lng: number } | null;
  zoom: number;
  hasCenteredRef: React.MutableRefObject<boolean>;
}) {
  const map = (require('react-leaflet') as any).useMap();
  useEffect(() => {
    if (positionToCenter && !hasCenteredRef.current && map) {
      map.setView([positionToCenter.lat, positionToCenter.lng], zoom);
      hasCenteredRef.current = true;
    }
  }, [positionToCenter, zoom, map, hasCenteredRef]);
  return null;
}

export default function LocationPicker({
  onChange,
  initialLat = -33.4489,
  initialLng = -70.6693,
  variant = 'default',
  rounded = 'md',
  className = '',
  draggable = true,
  mode = 'viewer',
  zoom = 13,
  height,
  fillContainer = false,
  externalPosition,
  recenterOnExternalChange = false,
}: LocationPickerProps) {
  const [position, setPosition] = useState<{ lat: number; lng: number } | null>(null);
  const [locationError, setLocationError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const hasCenteredInitially = useRef(false);
  const prevExternalRef = useRef<{ lat: number; lng: number } | null>(null);
  const [positionToCenter, setPositionToCenter] = useState<{ lat: number; lng: number } | null>(null);

  const isEditable = mode === 'edit' || mode === 'update';
  const extLat = externalPosition?.lat;
  const extLng = externalPosition?.lng;

  useEffect(() => {
    if (mode === 'update' && extLat != null && extLng != null && !Number.isNaN(extLat) && !Number.isNaN(extLng)) {
      const fromParent = { lat: extLat, lng: extLng };
      setPosition(fromParent);
      const externalChanged = prevExternalRef.current?.lat !== extLat || prevExternalRef.current?.lng !== extLng;
      prevExternalRef.current = fromParent;
      if (recenterOnExternalChange && externalChanged) {
        hasCenteredInitially.current = false;
        setPositionToCenter(fromParent);
      } else if (!hasCenteredInitially.current) {
        setPositionToCenter(fromParent);
      }
    } else if (mode === 'edit' && !position) {
      if (typeof window === 'undefined') return;
      const timer = setTimeout(() => {
        if (!navigator.geolocation) return;
        navigator.geolocation.getCurrentPosition(
          (p) => {
            const newPos = { lat: p.coords.latitude, lng: p.coords.longitude };
            setPosition(newPos);
            setPositionToCenter(newPos);
            onChange?.(newPos);
          },
          () => {
            const fallbackPos = { lat: initialLat, lng: initialLng };
            setPosition(fallbackPos);
            setPositionToCenter(fallbackPos);
            onChange?.(fallbackPos);
          },
          { enableHighAccuracy: true, timeout: 10000, maximumAge: 300000 },
        );
      }, 100);
      return () => clearTimeout(timer);
    } else if (initialLat != null && initialLng != null && !position) {
      const initialPos = { lat: initialLat, lng: initialLng };
      setPosition(initialPos);
      setPositionToCenter(initialPos);
    } else if (!position) {
      const defaultPos = { lat: initialLat, lng: initialLng };
      setPosition(defaultPos);
      setPositionToCenter(defaultPos);
      onChange?.(defaultPos);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode, extLat, extLng, initialLat, initialLng, recenterOnExternalChange]);

  const handleMarkerDragEnd = useCallback(
    (newPos: { lat: number; lng: number }) => {
      if (!isEditable) return;
      setPosition(newPos);
      onChange?.(newPos);
    },
    [isEditable, onChange],
  );

  const MapEvents = () => {
    (require('react-leaflet') as any).useMapEvents({
      click: (e: L.LeafletMouseEvent) => {
        if (!isEditable) return;
        const newPosition = { lat: e.latlng.lat, lng: e.latlng.lng };
        setPosition(newPosition);
        onChange?.(newPosition);
        if (containerRef.current) {
          void containerRef.current; // no-op: mantener consistencia con Wrapper visual
        }
      },
    });
    return null;
  };

  const containerClasses = [
    'location-container overflow-hidden relative',
    variantClasses[variant],
    roundedClasses[rounded],
    !isEditable ? 'pointer-events-none' : '',
    className,
  ]
    .filter(Boolean)
    .join(' ');

  const containerStyle: React.CSSProperties = fillContainer
    ? { zIndex: 0, height: '100%', width: '100%' }
    : height
      ? { zIndex: 0, height: `${height}vh`, width: '100%' }
      : { zIndex: 0, height: '200px', width: '100%' };

  const mapInitialCenter = useRef<[number, number] | null>(null);
  if (mapInitialCenter.current === null) {
    mapInitialCenter.current =
      mode === 'update' && externalPosition ? [externalPosition.lat, externalPosition.lng] : [initialLat, initialLng];
  }

  return (
    <div ref={containerRef} className={containerClasses} style={containerStyle}>
      {locationError ? (
        <div className="absolute top-2 right-2 z-1000 max-w-xs">
          <div className="bg-red-50 border border-red-200 rounded-lg p-3 shadow-sm">
            <div className="flex items-center gap-2">
              <AlertCircle className="text-red-500" size={20} />
              <span className="text-red-700 text-sm font-medium">{locationError}</span>
            </div>
          </div>
        </div>
      ) : null}

      <MapContainerAny
        center={mapInitialCenter.current}
        zoom={zoom}
        style={{ height: '100%', width: '100%' }}
        attributionControl={false}
        className={!isEditable ? 'cursor-default' : 'cursor-crosshair'}
        dragging={isEditable}
        zoomControl={isEditable}
        scrollWheelZoom={isEditable}
        doubleClickZoom={isEditable}
        touchZoom={isEditable}
      >
        <TileLayerAny url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png" />
        <MapController positionToCenter={positionToCenter} zoom={zoom} hasCenteredRef={hasCenteredInitially} />
        {isEditable ? <MapEvents /> : null}
        {position ? (
          <DraggableMarker position={position} draggable={isEditable && draggable} onDragEnd={handleMarkerDragEnd} />
        ) : null}
      </MapContainerAny>
    </div>
  );
}

