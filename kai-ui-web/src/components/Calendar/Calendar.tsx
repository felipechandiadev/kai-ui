"use client";

import { useMemo } from "react";
import { Button } from "../Button/Button";
import IconButton from "../IconButton";
import type { CalendarProps } from "./Calendar.types";
import {
  addDaysIso,
  DEFAULT_CALENDAR_END_HOUR,
  DEFAULT_CALENDAR_START_HOUR,
  DEFAULT_LOCALE,
  DEFAULT_TIME_ZONE,
  formatMonthTitle,
  formatWeekRangeLabel,
  getTodayIso,
  getWeekStart,
  parseIsoDateForDisplay,
  resolveReferenceIso,
} from "./calendar.utils";
import { MonthView } from "./MonthView";
import { WeekView } from "./WeekView";

export type { CalendarEvent, CalendarProps, CalendarView } from "./Calendar.types";

export function Calendar({
  view,
  referenceDate,
  events,
  locale = DEFAULT_LOCALE,
  timeZone = DEFAULT_TIME_ZONE,
  weekStartsOn = 1,
  minHour = DEFAULT_CALENDAR_START_HOUR,
  maxHour = DEFAULT_CALENDAR_END_HOUR,
  columnsFrom = "lg",
  headerRight,
  hideNavigation = false,
  onSelectDate,
  onSelectSlot,
  onNavigate,
  className = "",
  emptySlotLabel = "Agregar",
}: CalendarProps) {
  const todayIso = getTodayIso(timeZone);
  const referenceIso = resolveReferenceIso(referenceDate, timeZone);

  const weekStart = useMemo(
    () => getWeekStart(referenceIso, weekStartsOn),
    [referenceIso, weekStartsOn],
  );

  const monthDate = useMemo(
    () => parseIsoDateForDisplay(referenceIso),
    [referenceIso],
  );

  const title =
    view === "week"
      ? formatWeekRangeLabel(weekStart, { locale, timeZone })
      : formatMonthTitle(monthDate, { locale });

  const navigateTo = (iso: string) => {
    onNavigate?.(iso);
  };

  const handlePrev = () => {
    if (view === "week") {
      navigateTo(addDaysIso(weekStart, -7));
      return;
    }
    const d = new Date(monthDate.getFullYear(), monthDate.getMonth() - 1, 1);
    navigateTo(
      `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`,
    );
  };

  const handleNext = () => {
    if (view === "week") {
      navigateTo(addDaysIso(weekStart, 7));
      return;
    }
    const d = new Date(monthDate.getFullYear(), monthDate.getMonth() + 1, 1);
    navigateTo(
      `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`,
    );
  };

  const handleToday = () => {
    navigateTo(view === "week" ? getWeekStart(todayIso, weekStartsOn) : todayIso);
  };

  const nav = hideNavigation ? null : (
    <div className="flex items-center gap-1">
      <IconButton
        icon="ChevronLeft"
        variant="neutral"
        size="sm"
        ariaLabel={view === "week" ? "Semana anterior" : "Mes anterior"}
        onClick={handlePrev}
      />
      <Button type="button" variant="outlinedSecondary" size="sm" onClick={handleToday}>
        Hoy
      </Button>
      <IconButton
        icon="ChevronRight"
        variant="neutral"
        size="sm"
        ariaLabel={view === "week" ? "Semana siguiente" : "Mes siguiente"}
        onClick={handleNext}
      />
    </div>
  );

  return (
    <div
      className={["flex min-h-0 flex-col gap-3", className].filter(Boolean).join(" ")}
      data-test-id="calendar"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3">
          {nav}
          <h2 className="text-sm font-semibold text-foreground">{title}</h2>
        </div>
        {headerRight ? <div className="flex items-center gap-2">{headerRight}</div> : null}
      </div>

      {view === "week" ? (
        <WeekView
          weekStart={weekStart}
          events={events}
          locale={locale}
          timeZone={timeZone}
          minHour={minHour}
          maxHour={maxHour}
          columnsFrom={columnsFrom}
          onSelectDate={onSelectDate}
          onSelectSlot={onSelectSlot}
          emptySlotLabel={emptySlotLabel}
        />
      ) : (
        <MonthView
          month={monthDate}
          events={events}
          locale={locale}
          onSelectDate={onSelectDate}
        />
      )}
    </div>
  );
}

export default Calendar;
