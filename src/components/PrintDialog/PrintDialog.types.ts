import type { CSSProperties, ReactNode } from 'react';

export type PageSize = 'A4' | 'Letter' | 'A3' | 'A5';
export type PageOrientation = 'portrait' | 'landscape';

export interface PrintDialogProps {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  title?: string;
  fileName?: string;
  disablePrint?: boolean;
  printLoading?: boolean;
  showPrintButton?: boolean;
  /** Si es true, el botón de imprimir es un `IconButton` (variante básica secundaria) en lugar del botón de texto. */
  printIconButton?: boolean;
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'xxl' | 'custom';
  customSize?: Partial<Record<'xs' | 'sm' | 'md' | 'lg' | 'xl', number>>;
  maxWidth?: number | string;
  fullWidth?: boolean;
  scroll?: 'body' | 'paper';
  zIndex?: number;
  contentStyle?: CSSProperties;
  extraActions?: ReactNode;
  pageSize?: PageSize;
  pageOrientation?: PageOrientation;
}
