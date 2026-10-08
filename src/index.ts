// Primitives
export { default as Alert } from "./components/Alert";
export type { AlertVariant } from "./components/Alert/Alert";
export { Button, ButtonPill, ButtonGroup, ButtonGroupItem, ButtonGroupToggle } from "./components/Button";
export type {
  ButtonGroupProps,
  ButtonGroupItemProps,
  ButtonGroupToggleProps,
  ButtonGroupToggleOption,
  ButtonGroupDensity,
} from "./components/Button";
export { Badge } from "./components/Badge";
export type { BadgeVariant } from "./components/Badge/Badge";
export { default as Switch } from "./components/Switch";
export type { SwitchOptionLabels, SwitchDensity } from "./components/Switch";
export { default as IconButton } from "./components/IconButton";
export { default as DotProgress } from "./components/DotProgress";
export type { DotProgressVariant } from "./components/DotProgress/DotProgress";
export { Toast, ToastViewport } from "./components/Toast";
export type { ToastProps, ToastVariant, ToastViewportProps } from "./components/Toast";

// Inputs
export { TextField, default } from "./components/TextField/TextField";
export { default as TextFieldDefault } from "./components/TextField/TextField";
export { Select } from "./components/Select";
export type { Option as SelectOption, Option } from "./components/Select";
export { default as SelectDefault } from "./components/Select/Select";
export { default as AutoComplete } from "./components/AutoComplete";
export { default as DropdownList } from "./components/DropdownList";
export { default as NumberStepper } from "./components/NumberStepper";
export {
  resolveTouchInputMode,
  type TouchInputMode,
} from "./components/TextField/resolve-touch-input-mode";

// Dialog
export { default as Dialog } from "./components/Dialog";
export { DeleteDialog, type DeleteDialogProps } from "./components/Dialog/DeleteDialog";
export { default as DialogToPrint } from "./components/Dialog/DialogToPrint";
export {
  PrintDialog,
  usePrint,
  type PrintDialogProps,
  type PageOrientation,
  type PageSize,
} from "./components/PrintDialog";

// Data display
export { default as DataGrid } from "./components/DataGrid";
export { default as DataGridTable } from "./components/DataGrid/DataGrid";
export { RowActions } from "./components/DataGrid";
export type { DataGridColumn, DataGridCellOverflow, DataGridProps } from "./components/DataGrid";
export {
  getCellOverflowClassNames,
  resolveColumnCellOverflow,
  useScreenSize,
  calculateColumnStyles,
  DataGridStyles,
  type ColumnStyle,
} from "./components/DataGrid/utils/columnStyles";
export { resolveRowCellBackgroundColor } from "./components/DataGrid/utils/rowAppearance";
export {
  useDataGridFillViewportHeight,
  dataGridFillViewportFallbackHeight,
  DATA_GRID_TAB_LAYOUT_FALLBACK_EXTRA_PX,
} from "./components/DataGrid/utils/useDataGridFillViewportHeight";
export {
  default as CollectionGrid,
  type CollectionGridProps,
  buildContentGridClassNames as buildCollectionGridClassNames,
} from "./components/CollectionGrid";

// Navigation & layout primitives
export { default as Tabs } from "./components/Tabs";
export type { TabItem, TabsProps } from "./components/Tabs";
export { Card, StatisticsCard } from "./components/Cards";
export type {
  CardAction,
  CardIconAction,
  CardProps,
  CardTextAction,
  LucideIconName,
  StatisticsCardProps,
  StatisticsValueTone,
} from "./components/Cards";
export { Stepper } from "./components/Stepper";
export type { StepperProps, StepperStepItem } from "./components/Stepper";
export {
  Calendar,
  MonthView,
  WeekView,
  addDaysIso,
  DEFAULT_CALENDAR_END_HOUR,
  DEFAULT_CALENDAR_START_HOUR,
  DEFAULT_LOCALE,
  DEFAULT_TIME_ZONE,
  formatDayHeader,
  formatIsoDate,
  formatMonthTitle,
  formatWeekRangeLabel,
  getTodayIso,
  getWeekDays,
  getWeekStart,
  groupEventsByDate,
  hourLabels,
  nowLineOffsetPercent,
  parseIsoDate,
  parseIsoDateForDisplay,
  resolveReferenceIso,
  timeToMinutes,
  timeToOffsetPercent,
  toLocalIsoDate,
} from "./components/Calendar";
export type {
  CalendarColumnsFrom,
  CalendarEvent,
  CalendarProps,
  CalendarView,
  MonthViewProps,
  WeekViewProps,
} from "./components/Calendar";
export { default as RangeSlider } from "./components/RangeSlider";
export { default as LoadingState } from "./components/LoadingState";
export type { LoadingStateProps } from "./components/LoadingState";
export { Skeleton } from "./components/Skeleton";
export type { SkeletonProps } from "./components/Skeleton";

// Layouts
export {
  BasicPageLayout,
  TabPageLayout,
  PageLayoutHeader,
  buildContentGridClassNames,
  CollectionPageLayout,
  adminFillViewportBelowTopBarClassName,
  dataGridFillViewportTabPageProps,
  layoutPageContentClassName,
  layoutPageHeaderClassName,
  layoutPageRootClassName,
  layoutPageRootClassNameCompact,
  layoutPageSubtitleClassName,
  layoutPageTitleClassName,
  typographyBodyClassName,
  typographyCaptionClassName,
  typographyCompareAtPriceClassName,
  typographyDisabledClassName,
  typographyFieldErrorClassName,
  typographyFieldHintClassName,
  typographyFieldLabelClassName,
  typographyFieldSuccessClassName,
  typographyFontFamilies,
  typographyLinkClassName,
  typographyMoneyNegativeClassName,
  typographyMoneyNeutralClassName,
  typographyMoneyPositiveClassName,
  typographyMonoClassName,
  typographyMutedClassName,
  typographyOnColorClassName,
  typographyOnErrorClassName,
  typographyOnSuccessClassName,
  typographyOnWarningClassName,
  typographyPageSubtitleClassName,
  typographyPageTitleClassName,
  typographyPlaceholderClassName,
  typographyPosTouchLabelClassName,
  typographyPosTouchTotalClassName,
  typographyPosTouchValueClassName,
  typographyPriceClassName,
  typographyProductTitleClassName,
  typographyQuantityClassName,
  typographySalePriceClassName,
  typographyScaleEntries,
  typographySectionTitleClassName,
  typographySkuClassName,
  typographyStatusErrorClassName,
  typographyStatusInfoClassName,
  typographyStatusSuccessClassName,
  typographyStatusWarningClassName,
  typographySubsectionTitleClassName,
  typographyTabularNumberClassName,
  typographyTableCellClassName,
  typographyTableCellMutedClassName,
  typographyTableCellTruncateClassName,
  typographyTableHeaderClassName,
  typographyTimestampClassName,
  typographyIdClassName,
  getTypographyEntriesByGroup,
  typographyAntipatterns,
  typographyCssTokenRows,
  typographyRoleGroups,
  typographyUsageRules,
  typographyDataGridRowExample,
  typographyEshopPriceExample,
  typographyFormFieldExample,
  typographyKpiRowExample,
  typographyPageAnatomyExample,
  typographyPosTotalExample,
  typographyShowcaseNav,
  typographyStatusStackExample,
  type TypographyScaleEntry,
  type TypographyAntipattern,
  type TypographyRoleGroup,
  type TypographyUsageRule,
} from "./components/layouts";
export type {
  BasicPageLayoutProps,
  TabPageLayoutProps,
  PageLayoutHeaderProps,
  CollectionGridColumnConfig,
  CollectionPageLayoutProps,
} from "./components/layouts";

// Hooks
export { useCoarsePointer } from "./hooks/useCoarsePointer";

// Navigation (framework-agnostic; Next via NextNavProvider)
export {
  NavProvider,
  NextNavProvider,
  MemoryNavProvider,
  createMemoryNavAdapter,
  Link as NavLink,
  useRouter,
  usePathname,
  useSearchParams,
  useSetSearchParams,
} from "./navigation";
export type {
  NavRouter,
  NavRouterOptions,
  NavSearchParams,
  NavSearchParamsInput,
  SetNavSearchParams,
  NavContextValue,
  NavProviderProps,
  MemoryNavAdapter,
  MemoryNavProviderProps,
  LinkProps,
} from "./navigation";

// Shared types
export type {
  ButtonVariant,
  ButtonSize,
  TextFieldVariant,
  AutoCompleteOption,
  Tab,
  TabsProps as LegacyTabsProps,
  RangeValue,
  DialogProps,
  NumberStepperProps,
  SwitchProps,
  LocationCoordinates,
  UploadedFile,
  BaseFormField,
  BaseFormFieldType,
} from "./types";

// LocationPicker
export {
  LocationPicker,
  LocationPickerWrapper,
  LocationPreview,
} from "./components/LocationPicker";
export {
  appearanceStyle,
  buildAppearanceCssVars,
  resolveAppearance,
  DEFAULT_APPEARANCE,
  textOnColor,
  normalizeHexColor,
  type AppearanceColors,
} from "./theme/appearance-theme";
