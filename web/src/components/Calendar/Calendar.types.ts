export type CalendarView = "month" | "week";

/** Breakpoint from which the week grid shows day columns (instead of stacked mobile sections). */
export type CalendarColumnsFrom = "sm" | "md" | "lg" | "always";

export type CalendarEvent = {
  id: string;
  /** ISO date `YYYY-MM-DD`. */
  date: string;
  /** Optional start time `HH:mm` (week view positioning). */
  startTime?: string;
  /** Optional end time `HH:mm`. */
  endTime?: string;
  /** Rendered content inside the event cell/card. */
  content: React.ReactNode;
};

export type CalendarProps = {
  view: CalendarView;
  /** Reference date for the visible month/week. Defaults to today. */
  referenceDate?: Date | string;
  events: CalendarEvent[];
  locale?: string;
  timeZone?: string;
  /** 0 = Sunday, 1 = Monday (default). */
  weekStartsOn?: 0 | 1;
  minHour?: number;
  maxHour?: number;
  /**
   * From which breakpoint the week view shows day columns.
   * `"always"` keeps columns at any width (horizontal scroll on narrow screens).
   * Default `"lg"`.
   */
  columnsFrom?: CalendarColumnsFrom;
  /** Extra actions on the right of the navigation header. */
  headerRight?: React.ReactNode;
  /** Hide built-in prev/today/next controls. */
  hideNavigation?: boolean;
  onSelectDate?: (isoDate: string) => void;
  onSelectSlot?: (isoDate: string, hour?: number) => void;
  onNavigate?: (referenceIsoDate: string) => void;
  className?: string;
  /** Label for empty day slot CTA in week view. */
  emptySlotLabel?: string;
};
