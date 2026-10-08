/**
 * Contrato tipográfico compartido (@kai/ui).
 * Clases Tailwind reutilizables — importar en layouts, features y showcases del Design System.
 */

/** Título principal de página (h1 en layouts oficiales). */
export const typographyPageTitleClassName =
  'text-lg font-semibold tracking-tight text-foreground';

/** Subtítulo bajo el h1 en layouts oficiales. */
export const typographyPageSubtitleClassName = 'mt-1 text-sm text-muted-foreground';

/** Título de sección (h2) dentro de una vista. */
export const typographySectionTitleClassName =
  'text-xl font-semibold text-foreground';

/** Título de subsección (h3), cards y paneles. */
export const typographySubsectionTitleClassName =
  'text-sm font-semibold text-foreground';

/** Cuerpo de texto estándar. */
export const typographyBodyClassName = 'text-sm text-foreground';

/** Texto secundario, hints y metadatos. */
export const typographyMutedClassName = 'text-sm text-muted-foreground';

/** Caption / overline (kickers, encabezados de tabla compactos). */
export const typographyCaptionClassName =
  'text-xs font-semibold uppercase tracking-wide text-muted-foreground';

/** Código, variables CSS y snippets técnicos. */
export const typographyMonoClassName = 'font-mono text-xs text-foreground';

/** Enlaces de navegación y acciones inline. */
export const typographyLinkClassName =
  'font-medium text-link hover:underline';

/** Números tabulares — grillas ERP, montos alineados. */
export const typographyTabularNumberClassName =
  'tabular-nums text-sm font-medium text-foreground';

/** Montos neutros (sin signo semántico). */
export const typographyMoneyNeutralClassName =
  'tabular-nums text-sm font-medium text-foreground';

/** Montos positivos (ingresos, saldos a favor). */
export const typographyMoneyPositiveClassName =
  'tabular-nums text-sm font-medium text-text-positive';

/** Montos negativos (egresos, deuda) — signo explícito en UI. */
export const typographyMoneyNegativeClassName =
  'tabular-nums text-sm font-medium text-text-negative';

/** Texto sobre fondos primary (botones, badges). */
export const typographyOnColorClassName = 'text-text-on-primary';

/** Texto deshabilitado o no interactivo. */
export const typographyDisabledClassName = 'text-text-disabled';

/** Label de campo de formulario. */
export const typographyFieldLabelClassName =
  'text-sm font-medium text-foreground';

/** Hint bajo campo o ayuda contextual. */
export const typographyFieldHintClassName = 'text-xs text-muted-foreground';

/** Error de validación inline. */
export const typographyFieldErrorClassName = 'text-xs text-text-error';

/** Confirmación inline de campo. */
export const typographyFieldSuccessClassName = 'text-xs text-text-positive';

/** Placeholder de input. */
export const typographyPlaceholderClassName = 'text-sm text-text-disabled';

/** Cantidad numérica (stock, unidades). */
export const typographyQuantityClassName =
  'tabular-nums text-sm font-medium text-foreground';

/** SKU / código de producto. */
export const typographySkuClassName = 'font-mono text-xs text-muted-foreground';

/** Identificador técnico (UUID corto, folio). */
export const typographyIdClassName = 'font-mono text-xs text-foreground';

/** Timestamp / fecha compacta. */
export const typographyTimestampClassName =
  'text-xs tabular-nums text-muted-foreground';

/** Estado success en copy. */
export const typographyStatusSuccessClassName = 'text-sm text-text-positive';

/** Estado info en copy. */
export const typographyStatusInfoClassName = 'text-sm text-text-info';

/** Estado warning en copy. */
export const typographyStatusWarningClassName = 'text-sm text-text-warning';

/** Estado error en copy. */
export const typographyStatusErrorClassName = 'text-sm text-text-error';

/** Encabezado de columna DataGrid. */
export const typographyTableHeaderClassName = typographyCaptionClassName;

/** Celda estándar de tabla. */
export const typographyTableCellClassName = typographyBodyClassName;

/** Celda secundaria / metadato en tabla. */
export const typographyTableCellMutedClassName = typographyMutedClassName;

/** Celda con truncado y title. */
export const typographyTableCellTruncateClassName =
  'truncate text-sm text-foreground';

/** Label touch POS. */
export const typographyPosTouchLabelClassName =
  'text-base font-medium text-foreground';

/** Valor touch POS (línea, cantidad). */
export const typographyPosTouchValueClassName =
  'text-base tabular-nums font-medium text-foreground';

/** Total touch POS (checkout). */
export const typographyPosTouchTotalClassName =
  'text-2xl font-semibold tabular-nums text-foreground';

/** Título de producto eShop. */
export const typographyProductTitleClassName =
  'text-lg font-semibold text-foreground font-display';

/** Precio listado eShop. */
export const typographyPriceClassName =
  'tabular-nums text-base font-semibold text-foreground';

/** Precio oferta eShop. */
export const typographySalePriceClassName =
  'tabular-nums text-lg font-semibold text-text-positive';

/** Precio tachado (compare-at). */
export const typographyCompareAtPriceClassName =
  'tabular-nums text-sm text-muted-foreground line-through';

/** Texto sobre fondo success. */
export const typographyOnSuccessClassName = 'text-text-on-success';

/** Texto sobre fondo warning. */
export const typographyOnWarningClassName = 'text-text-on-warning';

/** Texto sobre fondo error. */
export const typographyOnErrorClassName = 'text-text-on-error';

export type TypographyScaleEntry = {
  id: string;
  name: string;
  className: string;
  sample: string;
  usage: string;
  fontFamily?: 'sans' | 'mono' | 'display';
};

/** Escala documentada para el Design System y referencia de producto. */
export const typographyScaleEntries: TypographyScaleEntry[] = [
  {
    id: 'page-title',
    name: 'Página (h1 layout)',
    className: typographyPageTitleClassName,
    sample: 'Productos',
    usage: 'BasicPageLayout, TabPageLayout, CollectionPageLayout',
    fontFamily: 'sans',
  },
  {
    id: 'section-title',
    name: 'Sección (h2)',
    className: typographySectionTitleClassName,
    sample: 'Reglas de uso',
    usage: 'Bloques dentro de una página, modales largos',
    fontFamily: 'sans',
  },
  {
    id: 'subsection-title',
    name: 'Subsección (h3)',
    className: typographySubsectionTitleClassName,
    sample: 'Estados interactivos',
    usage: 'Cards de documentación, paneles colapsables',
    fontFamily: 'sans',
  },
  {
    id: 'page-subtitle',
    name: 'Subtítulo layout',
    className: typographyPageSubtitleClassName,
    sample: 'Listado de artículos del catálogo',
    usage: 'Bajo el h1 en layouts oficiales',
    fontFamily: 'sans',
  },
  {
    id: 'body',
    name: 'Cuerpo',
    className: typographyBodyClassName,
    sample: 'Descripción de la operación o hint de campo.',
    usage: 'Párrafos, labels secundarios',
    fontFamily: 'sans',
  },
  {
    id: 'muted',
    name: 'Metadato / hint',
    className: typographyMutedClassName,
    sample: 'Última actualización hace 2 horas',
    usage: 'Placeholders, timestamps, ayuda contextual',
    fontFamily: 'sans',
  },
  {
    id: 'caption',
    name: 'Caption / overline',
    className: typographyCaptionClassName,
    sample: 'Foundations',
    usage: 'Kickers, encabezados de tabla, badges de sección',
    fontFamily: 'sans',
  },
  {
    id: 'mono',
    name: 'Mono / token',
    className: typographyMonoClassName,
    sample: '--color-primary',
    usage: 'Variables CSS, snippets técnicos',
    fontFamily: 'mono',
  },
  {
    id: 'field-label',
    name: 'Label de campo',
    className: typographyFieldLabelClassName,
    sample: 'Razón social',
    usage: 'TextField, Select, Switch — etiqueta visible',
    fontFamily: 'sans',
  },
  {
    id: 'field-hint',
    name: 'Hint de campo',
    className: typographyFieldHintClassName,
    sample: 'Como aparece en factura electrónica',
    usage: 'Ayuda bajo el input',
    fontFamily: 'sans',
  },
  {
    id: 'field-error',
    name: 'Error de validación',
    className: typographyFieldErrorClassName,
    sample: 'El RUT no es válido',
    usage: 'Mensaje inline bajo campo (Zod + Server Action)',
    fontFamily: 'sans',
  },
  {
    id: 'field-success',
    name: 'Confirmación de campo',
    className: typographyFieldSuccessClassName,
    sample: 'Disponible',
    usage: 'Feedback positivo inline opcional',
    fontFamily: 'sans',
  },
  {
    id: 'placeholder',
    name: 'Placeholder',
    className: typographyPlaceholderClassName,
    sample: 'Buscar producto…',
    usage: 'Texto placeholder en inputs',
    fontFamily: 'sans',
  },
  {
    id: 'quantity',
    name: 'Cantidad',
    className: typographyQuantityClassName,
    sample: '128',
    usage: 'Stock, unidades en tablas',
    fontFamily: 'sans',
  },
  {
    id: 'sku',
    name: 'SKU',
    className: typographySkuClassName,
    sample: 'SKU-00482',
    usage: 'Código de variante en listados',
    fontFamily: 'mono',
  },
  {
    id: 'id',
    name: 'Identificador',
    className: typographyIdClassName,
    sample: 'f6936150-754c',
    usage: 'Folios, IDs técnicos truncados',
    fontFamily: 'mono',
  },
  {
    id: 'timestamp',
    name: 'Timestamp',
    className: typographyTimestampClassName,
    sample: '11/07/2026 14:32',
    usage: 'Fechas compactas en metadatos',
    fontFamily: 'sans',
  },
  {
    id: 'tabular',
    name: 'Numérico tabular',
    className: typographyTabularNumberClassName,
    sample: '1.234.567,89',
    usage: 'DataGrid, totales — alineación vertical de cifras',
    fontFamily: 'sans',
  },
  {
    id: 'money-neutral',
    name: 'Monto neutro',
    className: typographyMoneyNeutralClassName,
    sample: '$ 125.400',
    usage: 'Totales sin signo semántico',
    fontFamily: 'sans',
  },
  {
    id: 'money-positive',
    name: 'Monto positivo',
    className: typographyMoneyPositiveClassName,
    sample: '+ $125.400',
    usage: 'Ingresos, saldos a favor',
    fontFamily: 'sans',
  },
  {
    id: 'money-negative',
    name: 'Monto negativo',
    className: typographyMoneyNegativeClassName,
    sample: '- $48.200',
    usage: 'Egresos, deuda — signo - en UI',
    fontFamily: 'sans',
  },
  {
    id: 'status-success',
    name: 'Estado success',
    className: typographyStatusSuccessClassName,
    sample: 'Pago confirmado',
    usage: 'Copy de éxito inline',
    fontFamily: 'sans',
  },
  {
    id: 'status-info',
    name: 'Estado info',
    className: typographyStatusInfoClassName,
    sample: 'Sincronización en curso',
    usage: 'Información neutral positiva',
    fontFamily: 'sans',
  },
  {
    id: 'status-warning',
    name: 'Estado warning',
    className: typographyStatusWarningClassName,
    sample: 'Stock bajo umbral',
    usage: 'Advertencias operativas',
    fontFamily: 'sans',
  },
  {
    id: 'status-error',
    name: 'Estado error',
    className: typographyStatusErrorClassName,
    sample: 'No se pudo emitir DTE',
    usage: 'Errores de negocio o sistema',
    fontFamily: 'sans',
  },
  {
    id: 'table-header',
    name: 'Header de tabla',
    className: typographyTableHeaderClassName,
    sample: 'Nombre',
    usage: 'ColHeader DataGrid — compacto uppercase',
    fontFamily: 'sans',
  },
  {
    id: 'table-cell',
    name: 'Celda de tabla',
    className: typographyTableCellClassName,
    sample: 'Aceite oliva 500 ml',
    usage: 'Contenido principal de fila',
    fontFamily: 'sans',
  },
  {
    id: 'table-cell-muted',
    name: 'Celda secundaria',
    className: typographyTableCellMutedClassName,
    sample: 'Actualizado ayer',
    usage: 'Metadatos en fila',
    fontFamily: 'sans',
  },
  {
    id: 'table-cell-truncate',
    name: 'Celda truncada',
    className: typographyTableCellTruncateClassName,
    sample: 'Descripción muy larga que se corta…',
    usage: 'truncate + title para tooltip nativo',
    fontFamily: 'sans',
  },
  {
    id: 'pos-touch-label',
    name: 'POS label',
    className: typographyPosTouchLabelClassName,
    sample: 'Total a pagar',
    usage: 'Etiquetas en checkout táctil',
    fontFamily: 'sans',
  },
  {
    id: 'pos-touch-value',
    name: 'POS valor',
    className: typographyPosTouchValueClassName,
    sample: '3 × $ 2.990',
    usage: 'Líneas de carrito POS',
    fontFamily: 'sans',
  },
  {
    id: 'pos-touch-total',
    name: 'POS total',
    className: typographyPosTouchTotalClassName,
    sample: '$ 842.500',
    usage: 'Total destacado en pago',
    fontFamily: 'sans',
  },
  {
    id: 'product-title',
    name: 'Título producto',
    className: typographyProductTitleClassName,
    sample: 'Camiseta algodón premium',
    usage: 'Detalle y cards eShop',
    fontFamily: 'display',
  },
  {
    id: 'price',
    name: 'Precio',
    className: typographyPriceClassName,
    sample: '$ 19.990',
    usage: 'Precio listado estándar',
    fontFamily: 'sans',
  },
  {
    id: 'sale-price',
    name: 'Precio oferta',
    className: typographySalePriceClassName,
    sample: '$ 14.990',
    usage: 'Precio promocional destacado',
    fontFamily: 'sans',
  },
  {
    id: 'compare-at-price',
    name: 'Precio tachado',
    className: typographyCompareAtPriceClassName,
    sample: '$ 19.990',
    usage: 'Compare-at / precio anterior',
    fontFamily: 'sans',
  },
  {
    id: 'link',
    name: 'Enlace',
    className: typographyLinkClassName,
    sample: 'Ver detalle',
    usage: 'Links inline, navegación secundaria',
    fontFamily: 'sans',
  },
  {
    id: 'on-color',
    name: 'Texto sobre primary',
    className: typographyOnColorClassName,
    sample: 'Guardar',
    usage: 'Labels en botones primary',
    fontFamily: 'sans',
  },
  {
    id: 'on-success',
    name: 'Texto sobre success',
    className: typographyOnSuccessClassName,
    sample: 'Confirmado',
    usage: 'Badges y botones success sólidos',
    fontFamily: 'sans',
  },
  {
    id: 'on-warning',
    name: 'Texto sobre warning',
    className: typographyOnWarningClassName,
    sample: 'Pendiente',
    usage: 'Badges warning sólidos',
    fontFamily: 'sans',
  },
  {
    id: 'on-error',
    name: 'Texto sobre error',
    className: typographyOnErrorClassName,
    sample: 'Anular',
    usage: 'Botones destructivos sólidos',
    fontFamily: 'sans',
  },
  {
    id: 'disabled',
    name: 'Deshabilitado',
    className: typographyDisabledClassName,
    sample: 'Campo no editable',
    usage: 'Readonly, acciones no disponibles',
    fontFamily: 'sans',
  },
];

/** Familias tipográficas del contrato (documentación / showcase). */
export const typographyFontFamilies = [
  {
    id: 'sans',
    token: '--font-sans',
    label: 'Sans (Inter)',
    sampleClassName: typographyBodyClassName,
    sample: 'Kai ERP — gestión unificada',
    usage: 'Cuerpo, títulos de página, UI general',
  },
  {
    id: 'mono',
    token: '--font-mono',
    label: 'Mono',
    sampleClassName: typographyMonoClassName,
    sample: 'SKU-00482 · --color-primary',
    usage: 'Código, tokens, identificadores técnicos',
  },
  {
    id: 'display',
    token: '--font-display',
    label: 'Display (alias sans en admin)',
    sampleClassName: `${typographySectionTitleClassName} font-display`,
    sample: 'Título display',
    usage: 'eShop: League Spartan; admin: alias de sans',
  },
] as const;
