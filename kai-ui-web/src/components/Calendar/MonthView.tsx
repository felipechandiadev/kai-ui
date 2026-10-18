"use client";

import { useMemo } from "react";
import type { CalendarEvent } from "./Calendar.types";
import {
  DEFAULT_LOCALE,
  formatMonthTitle,
  pad2,
  toLocalIsoDate,
} from "./calendar.utils";

export type MonthViewProps = {
  month?: Date;
  events: CalendarEvent[];
  locale?: string;
  headerRight?: React.ReactNode;
  onSelectDate?: (isoDate: string) => void;
  className?: string;
};

function parseDateOnly(isoOrAny: string): Date | null {
  const s = (isoOrAny ?? "").trim();
  if (!s) return null;
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
  if (m) {
    const y = Number(m[1]);
    const mo = Number(m[2]) - 1;
    const da = Number(m[3]);
    const d = new Date(y, mo, da);
    return Number.isNaN(d.getTime()) ? null : d;
  }
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? null : d;
}

function startOfMonth(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), 1);
}

function endOfMonth(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth() + 1, 0);
}

/** Monday-first calendar grid */
function mondayIndex(jsDay: number): number {
  return (jsDay + 6) % 7;
}

function addDays(d: Date, n: number): Date {
  const x = new Date(d);
  x.setDate(x.getDate() + n);
  return x;
}

function sameMonth(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth();
}

export function MonthView({
  month = new Date(),
  events,
  locale = DEFAULT_LOCALE,
  headerRight,
  onSelectDate,
  className = "",
}: MonthViewProps) {
  const monthStart = useMemo(() => startOfMonth(month), [month]);
  const monthEnd = useMemo(() => endOfMonth(month), [month]);

  const gridStart = useMemo(() => {
    const offset = mondayIndex(monthStart.getDay());
    return addDays(monthStart, -offset);
  }, [monthStart]);

  const gridEnd = useMemo(() => {
    const offset = 6 - mondayIndex(monthEnd.getDay());
    return addDays(monthEnd, offset);
  }, [monthEnd]);

  const days = useMemo(() => {
    const out: Date[] = [];
    for (let d = new Date(gridStart); d <= gridEnd; d = addDays(d, 1)) {
      out.push(new Date(d));
    }
    return out;
  }, [gridStart, gridEnd]);

  const eventsByDate = useMemo(() => {
    const m = new Map<string, CalendarEvent[]>();
    for (const it of events) {
      const d = parseDateOnly(it.date);
      if (!d) continue;
      const key = toLocalIsoDate(d);
      const arr = m.get(key) ?? [];
      arr.push(it);
      m.set(key, arr);
    }
    return m;
  }, [events]);

  const title = useMemo(
    () => formatMonthTitle(monthStart, { locale }),
    [monthStart, locale],
  );

  const todayKey = toLocalIsoDate(new Date());
  const dayLabels = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];

  return (
    <div
      className={[
        "flex h-full min-h-0 flex-col rounded-md border border-border bg-background",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      data-test-id="calendar-month-view"
    >
      <div className="flex shrink-0 items-center justify-between gap-3 border-b border-border px-3 py-2">
        <div className="text-sm font-semibold">{title}</div>
        <div className="flex items-center gap-2">{headerRight}</div>
      </div>

      <div className="grid shrink-0 grid-cols-7 border-b border-border bg-muted/30 text-xs text-muted-foreground">
        {dayLabels.map((d) => (
          <div key={d} className="px-2 py-2 font-medium">
            {d}
          </div>
        ))}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden">
        <div className="grid grid-cols-7">
          {days.map((d) => {
            const key = `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
            const inMonth = sameMonth(d, monthStart);
            const isToday = key === todayKey;
            const isMonthStart = inMonth && d.getDate() === 1;
            const dayEvents = eventsByDate.get(key) ?? [];
            return (
              <div
                key={key}
                onClick={() => onSelectDate?.(key)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    onSelectDate?.(key);
                  }
                }}
                role="button"
                tabIndex={0}
                className={[
                  "min-h-28 border-b border-r border-border px-2 py-2 text-left align-top",
                  "focus:outline-none focus:ring-2 focus:ring-[color:var(--color-accent)]/40",
                  inMonth
                    ? "bg-background hover:bg-[color:var(--color-hover)]"
                    : "bg-[color:var(--color-neutral)] text-muted-foreground hover:bg-[color:var(--color-neutral)]",
                  isMonthStart ? "border-l-2 border-l-[color:var(--color-accent)]" : "",
                ].join(" ")}
              >
                <div className="flex items-center justify-between">
                  <div
                    className={[
                      "text-xs font-medium tabular-nums",
                      isToday
                        ? "rounded-full bg-[color:var(--color-accent)] px-2 py-0.5 text-white"
                        : !inMonth
                          ? "opacity-70"
                          : "",
                    ].join(" ")}
                  >
                    {d.getDate()}
                  </div>
                </div>

                <div
                  className={["mt-2 flex flex-col gap-1", !inMonth ? "opacity-80" : ""].join(
                    " ",
                  )}
                >
                  {dayEvents.map((it) => (
                    <div key={it.id} className="min-w-0">
                      {it.content}
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

export default MonthView;
