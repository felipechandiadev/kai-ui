import type { TypographyScaleEntry } from './typographyTokens';

export type TypographyRoleGroup = {
  id: string;
  title: string;
  description: string;
  entryIds: string[];
};

export type TypographyUsageRule = {
  id: string;
  category:
    | 'hierarchy'
    | 'color'
    | 'numbers'
    | 'forms'
    | 'tables'
    | 'pos'
    | 'eshop'
    | 'accessibility';
  rule: string;
  do?: string;
  dont?: string;
};

export type TypographyAntipattern = {
  id: string;
  wrong: string;
  wrongClassName: string;
  right: string;
  rightClassName: string;
  why: string;
};

/** Orden de secciones en el Design System (excluye familias y ejemplos compuestos). */
export const typographyRoleGroups: TypographyRoleGroup[] = [
  {
    id: 'hierarchy',
    title: 'Jerarquía',
    description: 'Títulos y cuerpo — un solo h1 por vista (layout oficial).',
    entryIds: [
      'page-title',
      'section-title',
      'subsection-title',
      'page-subtitle',
      'body',
      'muted',
      'caption',
      'mono',
    ],
  },
  {
    id: 'forms',
    title: 'Formularios',
    description: 'Labels, hints, validación inline y placeholders.',
    entryIds: ['field-label', 'field-hint', 'field-error', 'field-success', 'placeholder'],
  },
  {
    id: 'data',
    title: 'Datos y números',
    description: 'SKU, IDs, timestamps y cantidades tabulares.',
    entryIds: ['quantity', 'sku', 'id', 'timestamp', 'tabular'],
  },
  {
    id: 'money',
    title: 'Montos',
    description: 'Negativos con signo explícito (- $48.200); tabular-nums y alineación derecha en tablas.',
    entryIds: ['money-neutral', 'money-positive', 'money-negative'],
  },
  {
    id: 'status',
    title: 'Estados / feedback',
    description: 'Texto semántico success, info, warning, error — siempre con copy o icono.',
    entryIds: ['status-success', 'status-info', 'status-warning', 'status-error'],
  },
  {
    id: 'table',
    title: 'Tablas / DataGrid',
    description: 'Headers compactos, celdas densas, truncado con title.',
    entryIds: ['table-header', 'table-cell', 'table-cell-muted', 'table-cell-truncate'],
  },
  {
    id: 'pos',
    title: 'POS touch',
    description: 'Variantes más grandes para pantallas táctiles; no inventar tamaños por feature.',
    entryIds: ['pos-touch-label', 'pos-touch-value', 'pos-touch-total'],
  },
  {
    id: 'eshop',
    title: 'eShop',
    description: 'Títulos de producto y precios (oferta, compare-at tachado).',
    entryIds: ['product-title', 'price', 'sale-price', 'compare-at-price'],
  },
  {
    id: 'interaction',
    title: 'Interacción',
    description: 'Enlaces, disabled y texto sobre fondos de color.',
    entryIds: ['link', 'disabled', 'on-color', 'on-success', 'on-warning', 'on-error'],
  },
];

export const typographyUsageRules: TypographyUsageRule[] = [
  {
    id: 'single-h1',
    category: 'hierarchy',
    rule: 'Un solo h1 por vista — lo provee el layout de página.',
    dont: 'Repetir h1 en hijos o modales largos (usar h2).',
  },
  {
    id: 'muted-foreground',
    category: 'color',
    rule: 'Texto secundario: text-muted-foreground o typographyMutedClassName.',
    dont: 'text-muted, text-foreground/60 ni hex sueltos.',
  },
  {
    id: 'links',
    category: 'color',
    rule: 'Enlaces: typographyLinkClassName / text-link.',
    dont: 'text-primary hover:underline como patrón de link.',
  },
  {
    id: 'tabular-nums',
    category: 'numbers',
    rule: 'Montos y cantidades: tabular-nums + alineación derecha en tablas.',
    do: 'typographyTabularNumberClassName o variantes money.',
  },
  {
    id: 'negative-sign',
    category: 'numbers',
    rule: 'Montos negativos en UI: signo explícito (- $48.200).',
    dont: 'Paréntesis contables en grillas ERP (reservado para reportes).',
  },
  {
    id: 'number-format',
    category: 'numbers',
    rule: 'Formateo con Intl.NumberFormat("es-CL") en lógica; UI solo aplica clases.',
  },
  {
    id: 'form-labels',
    category: 'forms',
    rule: 'Label medium + hint xs muted; error xs text-text-error.',
    dont: 'Mezclar hint y error en el mismo estilo.',
  },
  {
    id: 'table-density',
    category: 'tables',
    rule: 'DataGrid: headers caption/tableHeader; celdas text-sm; truncado con title.',
    dont: 'text-3xl en títulos de listado ERP.',
  },
  {
    id: 'pos-touch',
    category: 'pos',
    rule: 'POS: usar tokens posTouch*; html[data-pos-tablet] escala global opcional.',
    dont: 'font-size px sueltos por pantalla POS.',
  },
  {
    id: 'eshop-prices',
    category: 'eshop',
    rule: 'Precios eShop: salePrice + compareAtPrice line-through.',
  },
  {
    id: 'on-color',
    category: 'color',
    rule: 'Texto sobre fondos sólidos: text-text-on-*; no #fff hardcodeado.',
  },
  {
    id: 'status-copy',
    category: 'accessibility',
    rule: 'Estados: color semántico + texto o icono; no solo color.',
  },
  {
    id: 'font-weight',
    category: 'hierarchy',
    rule: 'semibold en títulos; medium en énfasis de cuerpo; bold solo KPIs.',
  },
  {
    id: 'uppercase',
    category: 'hierarchy',
    rule: 'Uppercase solo en captions/overlines cortos.',
    dont: 'Párrafos o labels de formulario en mayúsculas.',
  },
  {
    id: 'no-hex-text',
    category: 'color',
    rule: 'Prohibido hex de color de texto en JSX/CSS de features.',
    do: 'var(--color-text-*) y clases del contrato.',
  },
  {
    id: 'text-xs-essential',
    category: 'accessibility',
    rule: 'No usar text-xs para contenido esencial legible.',
    dont: 'Cuerpo principal en xs.',
  },
];

export const typographyAntipatterns: TypographyAntipattern[] = [
  {
    id: 'muted-vs-muted-foreground',
    wrong: 'Texto secundario con text-muted',
    wrongClassName: 'text-sm text-muted',
    right: 'text-muted-foreground',
    rightClassName: 'text-sm text-muted-foreground',
    why: 'text-muted es tinte de fondo (--color-muted), no color de texto.',
  },
  {
    id: 'primary-as-link',
    wrong: 'Link con text-primary',
    wrongClassName: 'text-sm font-medium text-primary hover:underline',
    right: 'text-link (typographyLinkClassName)',
    rightClassName: 'text-sm font-medium text-link hover:underline',
    why: 'Enlaces deben usar token semántico --color-link.',
  },
  {
    id: 'list-title-3xl',
    wrong: 'Título de listado ERP text-3xl',
    wrongClassName: 'text-3xl font-bold text-foreground',
    right: 'layoutPageTitleClassName (text-lg)',
    rightClassName: 'text-lg font-semibold tracking-tight text-foreground',
    why: 'Listados usan jerarquía compacta del layout oficial.',
  },
  {
    id: 'money-no-tabular',
    wrong: 'Monto sin tabular-nums',
    wrongClassName: 'text-sm font-medium text-foreground',
    right: 'tabular-nums en montos',
    rightClassName: 'tabular-nums text-sm font-medium text-foreground',
    why: 'Cifras desalineadas en columnas numéricas.',
  },
  {
    id: 'negative-parentheses',
    wrong: 'Negativo contable en UI',
    wrongClassName: 'tabular-nums text-sm text-text-negative',
    right: 'Signo explícito - $48.200',
    rightClassName: 'tabular-nums text-sm font-medium text-text-negative',
    why: 'Convención UI: signo -; paréntesis solo en reportes contables.',
  },
  {
    id: 'foreground-opacity',
    wrong: 'Secundario con opacidad',
    wrongClassName: 'text-sm text-foreground/60',
    right: 'text-muted-foreground',
    rightClassName: 'text-sm text-muted-foreground',
    why: 'Opacidad ad hoc rompe contraste predecible.',
  },
  {
    id: 'white-on-primary',
    wrong: 'text-white en botón primary',
    wrongClassName: 'text-sm font-medium text-white',
    right: 'text-text-on-primary',
    rightClassName: 'text-sm font-medium text-text-on-primary',
    why: 'Token on-primary permite theming por PWA.',
  },
  {
    id: 'error-only-color',
    wrong: 'Error solo rojo sin copy',
    wrongClassName: 'text-text-error',
    right: 'Mensaje explícito',
    rightClassName: 'text-xs text-text-error',
    why: 'Accesibilidad: no depender solo del color.',
  },
];

export function getTypographyEntriesByGroup(
  entries: TypographyScaleEntry[],
  groupId: string,
): TypographyScaleEntry[] {
  const group = typographyRoleGroups.find((g) => g.id === groupId);
  if (!group) return [];
  const byId = new Map(entries.map((e) => [e.id, e]));
  return group.entryIds.map((id) => byId.get(id)).filter(Boolean) as TypographyScaleEntry[];
}

export const typographyCssTokenRows = [
  { token: '--font-sans / --font-mono / --font-display', usage: 'Familias tipográficas' },
  { token: '--font-weight-* / --line-height-* / --letter-spacing-*', usage: 'Pesos, interlineado y tracking' },
  { token: '--color-text-primary / --color-text-secondary', usage: 'Texto principal y secundario' },
  { token: '--color-text-link', usage: 'Enlaces → text-link' },
  { token: '--color-text-disabled', usage: 'Texto no interactivo' },
  { token: '--color-text-on-primary / on-success / on-warning / on-error', usage: 'Texto sobre fondos sólidos' },
  { token: '--color-text-positive / negative / info / warning / error', usage: 'Estados y montos semánticos' },
] as const;
