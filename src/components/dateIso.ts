import { t } from "../i18n";

/** ISO `yyyy-MM-dd` ↔ отображение `dd.MM.yyyy` для UI-календаря. */

export function toIsoDate(raw: string | undefined | null): string {
  const s = (raw ?? "").trim();
  if (!s) return "";
  if (/^\d{4}-\d{2}-\d{2}/.test(s)) return s.slice(0, 10);
  const m = /^(\d{2})\.(\d{2})\.(\d{4})/.exec(s);
  if (m) return `${m[3]}-${m[2]}-${m[1]}`;
  return "";
}

export function formatDisplayDate(iso: string): string {
  const s = toIsoDate(iso);
  if (!s) return "";
  const [y, m, d] = s.split("-");
  return `${d}.${m}.${y}`;
}

export function parseIso(iso: string): Date | null {
  const s = toIsoDate(iso);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) return null;
  const [y, m, d] = s.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  if (dt.getFullYear() !== y || dt.getMonth() !== m - 1 || dt.getDate() !== d) {
    return null;
  }
  return dt;
}

export function toIsoFromDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export function startOfMonth(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), 1);
}

export function addMonths(d: Date, n: number): Date {
  return new Date(d.getFullYear(), d.getMonth() + n, 1);
}

export function sameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

export function isBeforeDay(a: Date, b: Date): boolean {
  const aa = new Date(a.getFullYear(), a.getMonth(), a.getDate()).getTime();
  const bb = new Date(b.getFullYear(), b.getMonth(), b.getDate()).getTime();
  return aa < bb;
}

export const MONTH_NAMES_RU = [
  t("dateCalendar.months.1"),
  t("dateCalendar.months.2"),
  t("dateCalendar.months.3"),
  t("dateCalendar.months.4"),
  t("dateCalendar.months.5"),
  t("dateCalendar.months.6"),
  t("dateCalendar.months.7"),
  t("dateCalendar.months.8"),
  t("dateCalendar.months.9"),
  t("dateCalendar.months.10"),
  t("dateCalendar.months.11"),
  t("dateCalendar.months.12"),
] as const;

/** Ключи объекта в locales (не JSON-массив: flatten i18n не делает `.0` для array). */
export const WEEKDAYS_RU = [
  t("dateCalendar.weekdaysShort.0"),
  t("dateCalendar.weekdaysShort.1"),
  t("dateCalendar.weekdaysShort.2"),
  t("dateCalendar.weekdaysShort.3"),
  t("dateCalendar.weekdaysShort.4"),
  t("dateCalendar.weekdaysShort.5"),
  t("dateCalendar.weekdaysShort.6"),
] as const;
