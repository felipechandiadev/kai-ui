/** Datos de ejemplo para bloques compuestos del showcase tipográfico. */

export const typographyPageAnatomyExample = {
  kicker: 'Catálogo',
  title: 'Productos',
  subtitle: 'Listado de artículos del catálogo central',
  body: 'Filtra por categoría o busca por SKU. Los cambios se sincronizan con POS y eShop.',
} as const;

export const typographyDataGridRowExample = {
  name: 'Aceite oliva extra virgen 500 ml',
  sku: 'SKU-00482',
  stock: 128,
  amountPositive: '+ $125.400',
  amountNegative: '- $48.200',
} as const;

export const typographyKpiRowExample = {
  label: 'Ventas del día',
  value: '$ 842.500',
  variation: '+12,4%',
  variationHint: 'vs ayer',
} as const;

export const typographyFormFieldExample = {
  label: 'Razón social',
  hint: 'Como aparece en factura electrónica',
  error: 'El RUT no es válido',
  placeholder: 'Distribuidora Norte SPA',
} as const;

export const typographyStatusStackExample = [
  { id: 'success', label: 'Pago confirmado' },
  { id: 'info', label: 'Sincronización en curso' },
  { id: 'warning', label: 'Stock bajo umbral' },
  { id: 'error', label: 'No se pudo emitir DTE' },
] as const;

export const typographyPosTotalExample = {
  label: 'Total a pagar',
  lines: '3 × $ 2.990',
  total: '$ 842.500',
} as const;

export const typographyEshopPriceExample = {
  title: 'Camiseta algodón premium',
  salePrice: '$ 14.990',
  compareAtPrice: '$ 19.990',
} as const;

export const typographyShowcaseNav = [
  { id: 'typo-page-anatomy', label: 'Anatomía de página' },
  { id: 'typo-datagrid-row', label: 'Fila DataGrid' },
  { id: 'typo-kpi-row', label: 'KPI' },
  { id: 'typo-form-field', label: 'Campo formulario' },
  { id: 'typo-status-stack', label: 'Estados' },
  { id: 'typo-pos-total', label: 'POS total' },
  { id: 'typo-eshop-price', label: 'Precio eShop' },
  { id: 'typo-correct-vs-wrong', label: 'Correcto vs incorrecto' },
] as const;
