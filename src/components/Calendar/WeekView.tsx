"use client";

import { useMemo } from "react";
import { Badge } from "../Badge";
import type { CalendarColumnsFrom, CalendarEvent } from "./Calendar.types";
import {
  DEFAULT_CALENDAR_END_HOUR,
  DEFAULT_CALENDAR_START_HOUR,
  DEFAULT_LOCALE,
  DEFAULT_TIME_ZONE,
  formatDayHeader,
  getTodayIso,
  getWeekDays,
  groupEventsByDate,
  hourLabels,
  nowLineOffsetPercent,
  timeToOffsetPercent,
} from "./calendar.utils";

const GRID_HEIGHT_REM = 36;

function columnsVisibilityClasses(columnsFrom: CalendarColumnsFrom): {
  grid: string;
  mobile: string | null;
} {
  switch (columnsFrom) {
    case "always":
      return { grid: "block", mobile: null };
    case "sm":
      return { grid: "hidden sm:block", mobile: "sm:hidden" };
    case "md":
      return { grid: "hidden md:block", mobile: "md:hidden" };
    case "lg":
    default:
      return { grid: "hidden lg:block", mobile: "lg:hidden" };
  }
}

export type WeekViewProps = {
  weekStart: string;
  events: CalendarEvent[];
  locale?: string;
  timeZone?: string;
  minHour?: number;
  maxHour?: number;
  columnsFrom?: CalendarColumnsFrom;
  onSelectDate?: (isoDate: string) => void;
  onSelectSlot?: (isoDate: string, hour?: number) => void;
  emptySlotLabel?: string;
  className?: string;
};

export function WeekView({
  weekStart,
  events,
  locale = DEFAULT_LOCALE,
  timeZone = DEFAULT_TIME_ZONE,
  minHour = DEFAULT_CALENDAR_START_HOUR,
  maxHour = DEFAULT_CALENDAR_END_HOUR,
  columnsFrom = "lg",
  onSelectDate,
  onSelectSlot,
  emptySlotLabel = "Agregar",
  className = "",
}: WeekViewProps) {
  const weekDays = useMemo(() => getWeekDays(weekStart), [weekStart]);
  const byDate = useMemo(() => groupEventsByDate(events), [events]);
  const todayIso = getTodayIso(timeZone);
  const labels = useMemo(() => hourLabels(minHour, maxHour), [minHour, maxHour]);
  const span = Math.max(1, maxHour - minHour);
  const visibility = columnsVisibilityClasses(columnsFrom);

  return (
    <div className={className} data-test-id="calendar-week-view">
      <div
        className={`${visibility.grid} overflow-x-auto rounded-xl border border-border bg-card`}
        role="grid"
        aria-label="Calendario semanal"
      >
        <div className="flex min-w-[56rem]">
          <div className="sticky left-0 z-20 w-12 shrink-0 border-r border-border/60 bg-background">
            <div className="h-13 border-b border-border/60" />
            <div className="relative" style={{ minHeight: `${GRID_HEIGHT_REM}rem` }}>
              {labels.map((hour) => (
                <div
                  key={hour}
                  className="absolute right-1 -translate-y-1/2 text-[10px] tabular-nums text-muted-foreground"
                  style={{
                    top: `${((hour - minHour) / span) * 100}%`,
                  }}
                >
                  {String(hour).padStart(2, "0")}:00
                </div>
              ))}
            </div>
          </div>
          {weekDays.map((date) => (
            <WeekDayColumn
              key={date}
              date={date}
              events={byDate.get(date) ?? []}
              isToday={date === todayIso}
              locale={locale}
              timeZone={timeZone}
              minHour={minHour}
              maxHour={maxHour}
              emptySlotLabel={emptySlotLabel}
              onSelectDate={onSelectDate}
              onSelectSlot={onSelectSlot}
            />
          ))}
        </div>
      </div>

      {visibility.mobile ? (
        <div className={`space-y-3 ${visibility.mobile}`}>
          {weekDays.map((date) => (
            <WeekDayMobileSection
              key={date}
              date={date}
              events={byDate.get(date) ?? []}
              isToday={date === todayIso}
              locale={locale}
              timeZone={timeZone}
              emptySlotLabel={emptySlotLabel}
              onSelectDate={onSelectDate}
              onSelectSlot={onSelectSlot}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

type DayColumnSharedProps = {
  date: string;
  events: CalendarEvent[];
  isToday: boolean;
  locale: string;
  timeZone: string;
  emptySlotLabel: string;
  onSelectDate?: (isoDate: string) => void;
  onSelectSlot?: (isoDate: string, hour?: number) => void;
};

function WeekDayColumn({
  date,
  events,
  isToday,
  locale,
  timeZone,
  minHour,
  maxHour,
  emptySlotLabel,
  onSelectDate,
  onSelectSlot,
}: DayColumnSharedProps & { minHour: number; maxHour: number }) {
  const nowOffset = isToday
    ? nowLineOffsetPercent({ timeZone, minHour, maxHour })
    : null;
  const span = Math.max(1, maxHour - minHour);

  return (
    <div
      className={`relative flex min-w-[8.5rem] flex-1 flex-col border-l border-border/60 ${
        isToday ? "bg-primary/[0.03]" : ""
      }`}
    >
      <button
        type="button"
        onClick={() => onSelectDate?.(date)}
        className={`sticky top-0 z-20 border-b border-border/60 bg-background/95 px-2 py-2 text-center backdrop-blur-sm ${
          isToday ? "ring-1 ring-inset ring-primary/20" : ""
        }`}
      >
        <p className="text-xs font-medium text-foreground">
          {formatDayHeader(date, { locale, timeZone })}
        </p>
        {isToday ? (
          <Badge variant="primary-outlined" className="mt-0.5 text-[10px]">
            Hoy
          </Badge>
        ) : null}
      </button>
      <div
        className="relative flex-1"
        style={{ minHeight: `${GRID_HEIGHT_REM}rem` }}
        role="row"
      >
        {Array.from({ length: span }, (_, i) => (
          <button
            key={i}
            type="button"
            aria-label={`${date} ${String(minHour + i).padStart(2, "0")}:00`}
            className="absolute left-0 right-0 border-t border-border/30 hover:bg-muted/20"
            style={{
              top: `${(i / span) * 100}%`,
              height: `${(1 / span) * 100}%`,
            }}
            onClick={() => onSelectSlot?.(date, minHour + i)}
          />
        ))}
        {nowOffset != null ? (
          <div
            className="pointer-events-none absolute left-0 right-0 z-30 border-t-2 border-red-500/80"
            style={{ top: `${nowOffset}%` }}
            aria-hidden
          />
        ) : null}
        {events.map((event) => (
          <div
            key={event.id}
            className="absolute left-1 right-1 z-10"
            style={{
              top: `${timeToOffsetPercent(event.startTime ?? "09:00", minHour, maxHour)}%`,
            }}
          >
            {event.content}
          </div>
        ))}
        {events.length === 0 ? (
          <button
            type="button"
            onClick={() => onSelectSlot?.(date)}
            className="absolute inset-x-2 bottom-2 z-10 rounded-lg border border-dashed border-border/80 px-2 py-3 text-center text-xs text-muted-foreground transition-colors hover:border-primary/40 hover:bg-muted/30 hover:text-foreground"
          >
            {emptySlotLabel}
          </button>
        ) : null}
      </div>
    </div>
  );
}

function WeekDayMobileSection({
  date,
  events,
  isToday,
  locale,
  timeZone,
  emptySlotLabel,
  onSelectSlot,
}: DayColumnSharedProps) {
  return (
    <details className="rounded-lg border border-border bg-card" open={isToday}>
      <summary className="flex cursor-pointer list-none items-center justify-between px-4 py-3">
        <span className="text-sm font-medium">
          {formatDayHeader(date, { locale, timeZone })}
          {isToday ? (
            <Badge variant="primary-outlined" className="ml-2 text-[10px]">
              Hoy
            </Badge>
          ) : null}
        </span>
        <span className="text-xs text-muted-foreground">
          {events.length} evento{events.length === 1 ? "" : "s"}
        </span>
      </summary>
      <div className="space-y-2 border-t border-border px-3 py-3">
        {events.length === 0 ? (
          <button
            type="button"
            onClick={() => onSelectSlot?.(date)}
            className="w-full rounded-lg border border-dashed px-3 py-4 text-sm text-muted-foreground hover:border-primary/40 hover:text-foreground"
          >
            {emptySlotLabel}
          </button>
        ) : (
          events.map((event) => (
            <div key={event.id} className="relative min-h-20">
              {event.content}
            </div>
          ))
        )}
      </div>
    </details>
  );
}

export default WeekView;
