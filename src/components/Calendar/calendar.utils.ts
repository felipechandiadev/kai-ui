/** Helpers TZ-safe para calendarios genéricos (date-only + grilla horaria). */

export const DEFAULT_TIME_ZONE = "America/Santiago";
export const DEFAULT_LOCALE = "es-CL";
export const DEFAULT_CALENDAR_START_HOUR = 8;
export const DEFAULT_CALENDAR_END_HOUR = 20;

export function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

/** Parsea `YYYY-MM-DD` como UTC medianoche (para aritmética de días). */
export function parseIsoDate(dateStr: string): Date {
  const [y, m, d] = dateStr.split("-").map(Number);
  return new Date(Date.UTC(y ?? 1970, (m ?? 1) - 1, d ?? 1));
}

/** Mediodía UTC evita que Intl con America/Santiago muestre el día anterior. */
export function parseIsoDateForDisplay(dateStr: string): Date {
  const [y, m, d] = dateStr.split("-").map(Number);
  return new Date(Date.UTC(y ?? 1970, (m ?? 1) - 1, d ?? 1, 12, 0, 0));
}

export function formatIsoDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

/** Construye `YYYY-MM-DD` desde partes locales del Date (sin UTC). */
export function toLocalIsoDate(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

export function addDaysIso(dateStr: string, days: number): string {
  const d = parseIsoDate(dateStr);
  d.setUTCDate(d.getUTCDate() + days);
  return formatIsoDate(d);
}

export function getTodayIso(timeZone: string = DEFAULT_TIME_ZONE): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(new Date());
  const y = parts.find((p) => p.type === "year")?.value ?? "1970";
  const m = parts.find((p) => p.type === "month")?.value ?? "01";
  const d = parts.find((p) => p.type === "day")?.value ?? "01";
  return `${y}-${m}-${d}`;
}

/** Lunes como inicio de semana por defecto (`weekStartsOn = 1`). */
export function getWeekStart(dateStr: string, weekStartsOn: 0 | 1 = 1): string {
  const d = parseIsoDate(dateStr);
  const weekday = d.getUTCDay(); // 0=Sun..6=Sat
  let offset: number;
  if (weekStartsOn === 1) {
    offset = weekday === 0 ? -6 : 1 - weekday;
  } else {
    offset = -weekday;
  }
  d.setUTCDate(d.getUTCDate() + offset);
  return formatIsoDate(d);
}

export function getWeekDays(weekStart: string): string[] {
  return Array.from({ length: 7 }, (_, i) => addDaysIso(weekStart, i));
}

export function formatWeekRangeLabel(
  weekStart: string,
  options?: { locale?: string; timeZone?: string },
): string {
  const locale = options?.locale ?? DEFAULT_LOCALE;
  const timeZone = options?.timeZone ?? DEFAULT_TIME_ZONE;
  const end = addDaysIso(weekStart, 6);
  const startDate = parseIsoDateForDisplay(weekStart);
  const endDate = parseIsoDateForDisplay(end);
  const fmt = new Intl.DateTimeFormat(locale, {
    timeZone,
    day: "numeric",
    month: "short",
  });
  const year = new Intl.DateTimeFormat(locale, {
    timeZone,
    year: "numeric",
  }).format(startDate);
  return `Semana del ${fmt.format(startDate)} al ${fmt.format(endDate)} ${year}`;
}

export function formatDayHeader(
  dateStr: string,
  options?: { locale?: string; timeZone?: string },
): string {
  const locale = options?.locale ?? DEFAULT_LOCALE;
  const timeZone = options?.timeZone ?? DEFAULT_TIME_ZONE;
  const d = parseIsoDateForDisplay(dateStr);
  const weekday = new Intl.DateTimeFormat(locale, {
    timeZone,
    weekday: "short",
  }).format(d);
  const dayNum = new Intl.DateTimeFormat(locale, {
    timeZone,
    day: "numeric",
  }).format(d);
  return `${weekday} ${dayNum}`;
}

export function formatMonthTitle(
  month: Date,
  options?: { locale?: string },
): string {
  const locale = options?.locale ?? DEFAULT_LOCALE;
  return new Intl.DateTimeFormat(locale, { month: "long", year: "numeric" })
    .format(month)
    .replace(/^\w/, (c) => c.toUpperCase());
}

export function timeToMinutes(time: string): number {
  const [h, m] = time.split(":").map(Number);
  return (h ?? 0) * 60 + (m ?? 0);
}

export function timeToOffsetPercent(
  time: string,
  minHour: number = DEFAULT_CALENDAR_START_HOUR,
  maxHour: number = DEFAULT_CALENDAR_END_HOUR,
): number {
  const mins = timeToMinutes(time);
  const start = minHour * 60;
  const end = maxHour * 60;
  const span = end - start;
  if (span <= 0) return 0;
  const clamped = Math.max(start, Math.min(end, mins));
  return ((clamped - start) / span) * 100;
}

export function getNowMinutes(timeZone: string = DEFAULT_TIME_ZONE): number {
  const nowParts = new Intl.DateTimeFormat("en-GB", {
    timeZone,
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).formatToParts(new Date());
  const hour = Number(nowParts.find((p) => p.type === "hour")?.value ?? 0);
  const minute = Number(nowParts.find((p) => p.type === "minute")?.value ?? 0);
  return hour * 60 + minute;
}

export function nowLineOffsetPercent(options?: {
  timeZone?: string;
  minHour?: number;
  maxHour?: number;
}): number | null {
  const timeZone = options?.timeZone ?? DEFAULT_TIME_ZONE;
  const minHour = options?.minHour ?? DEFAULT_CALENDAR_START_HOUR;
  const maxHour = options?.maxHour ?? DEFAULT_CALENDAR_END_HOUR;
  const mins = getNowMinutes(timeZone);
  if (mins < minHour * 60 || mins > maxHour * 60) return null;
  const hour = Math.floor(mins / 60);
  const minute = mins % 60;
  return timeToOffsetPercent(
    `${pad2(hour)}:${pad2(minute)}`,
    minHour,
    maxHour,
  );
}

export function hourLabels(minHour: number, maxHour: number): number[] {
  if (maxHour < minHour) return [];
  return Array.from({ length: maxHour - minHour + 1 }, (_, i) => minHour + i);
}

export function resolveReferenceIso(
  referenceDate: Date | string | undefined,
  timeZone: string = DEFAULT_TIME_ZONE,
): string {
  if (!referenceDate) return getTodayIso(timeZone);
  if (typeof referenceDate === "string") {
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(referenceDate.trim());
    if (m) return `${m[1]}-${m[2]}-${m[3]}`;
    const d = new Date(referenceDate);
    return Number.isNaN(d.getTime()) ? getTodayIso(timeZone) : formatIsoDate(d);
  }
  return toLocalIsoDate(referenceDate);
}

export function groupEventsByDate<T extends { date: string }>(
  events: T[],
): Map<string, T[]> {
  const map = new Map<string, T[]>();
  for (const event of events) {
    const key = event.date.slice(0, 10);
    const list = map.get(key) ?? [];
    list.push(event);
    map.set(key, list);
  }
  return map;
}
