import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  MONTH_NAMES_RU,
  WEEKDAYS_RU,
  addMonths,
  formatDisplayDate,
  isBeforeDay,
  parseIso,
  sameDay,
  startOfMonth,
  toIsoDate,
  toIsoFromDate,
} from "./dateIso";
import "./DateRangePicker.css";
import { t } from "../i18n";

export type DateRangeValue = { from: string; to: string };

type Props = {
  /** `range` — С/По; `single` — одна дата (в `from`). */
  mode?: "single" | "range";
  valueFrom?: string;
  valueTo?: string;
  /** Всегда ISO `yyyy-MM-dd` (пустая строка = очищено). */
  onChange: (next: DateRangeValue) => void;
  disabled?: boolean;
  fromLabel?: string;
  toLabel?: string;
  singleLabel?: string;
  className?: string;
};

type OpenField = "from" | "to" | null;

type DayCell = {
  date: Date;
  iso: string;
  outside: boolean;
};

function buildMonthCells(view: Date): DayCell[] {
  const first = startOfMonth(view);
  const startPad = (first.getDay() + 6) % 7; // Mon=0
  const gridStart = new Date(first);
  gridStart.setDate(first.getDate() - startPad);
  const cells: DayCell[] = [];
  for (let i = 0; i < 42; i++) {
    const d = new Date(gridStart);
    d.setDate(gridStart.getDate() + i);
    cells.push({
      date: d,
      iso: toIsoFromDate(d),
      outside: d.getMonth() !== view.getMonth(),
    });
  }
  return cells;
}

const VIEWPORT_PAD = 8;

/** Тематический календарь (CSS vars) вместо native `type="date"`. */
export default function DateRangePicker({
  mode = "range",
  valueFrom = "",
  valueTo = "",
  onChange,
  disabled,
  fromLabel = t("common.dateFrom"),
  toLabel = t("common.dateTo"),
  singleLabel = t("common.date"),
  className,
}: Props) {
  const uid = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const popRef = useRef<HTMLDivElement>(null);
  const triggerRefs = useRef<Partial<Record<"from" | "to", HTMLButtonElement | null>>>({});
  const fromIso = toIsoDate(valueFrom);
  const toIso = toIsoDate(valueTo);
  const [open, setOpen] = useState<OpenField>(null);
  const [view, setView] = useState(() => {
    const seed = parseIso(fromIso) ?? parseIso(toIso) ?? new Date();
    return startOfMonth(seed);
  });
  const [rangeDraft, setRangeDraft] = useState<string | null>(null);

  const cells = useMemo(() => buildMonthCells(view), [view]);
  const today = useMemo(() => {
    const n = new Date();
    return new Date(n.getFullYear(), n.getMonth(), n.getDate());
  }, []);

  useEffect(() => {
    if (!open) {
      setRangeDraft(null);
      return;
    }
    const seed =
      open === "to"
        ? parseIso(toIso) ?? parseIso(fromIso)
        : parseIso(fromIso) ?? parseIso(toIso);
    if (seed) setView(startOfMonth(seed));
  }, [open, fromIso, toIso]);

  useLayoutEffect(() => {
    if (!open || !popRef.current) return;
    const btn = triggerRefs.current[open];
    const pop = popRef.current;
    if (!btn) return;

    const rect = btn.getBoundingClientRect();
    pop.style.position = "fixed";
    pop.style.visibility = "hidden";
    pop.style.top = "0";
    pop.style.left = "0";
    pop.style.right = "auto";
    pop.style.bottom = "auto";

    const pad = VIEWPORT_PAD;
    const w = pop.offsetWidth;
    const h = pop.offsetHeight;
    let left = rect.left;
    if (left + w > window.innerWidth - pad) {
      left = Math.max(pad, window.innerWidth - pad - w);
    }
    if (left < pad) left = pad;

    let top = rect.bottom + 6;
    if (top + h > window.innerHeight - pad) {
      top = Math.max(pad, rect.top - 6 - h);
    }

    pop.style.left = `${left}px`;
    pop.style.top = `${top}px`;
    pop.style.visibility = "visible";
  }, [open, view, cells]);

  useEffect(() => {
    if (!open) return;
    function onDoc(e: MouseEvent) {
      const t = e.target as Node;
      if (rootRef.current?.contains(t) || popRef.current?.contains(t)) return;
      setOpen(null);
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(null);
    }
    function onReposition() {
      // scroll/resize — закрыть, иначе календарь «уедет» от поля
      setOpen(null);
    }
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    window.addEventListener("resize", onReposition);
    window.addEventListener("scroll", onReposition, true);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
      window.removeEventListener("resize", onReposition);
      window.removeEventListener("scroll", onReposition, true);
    };
  }, [open]);

  function pickDay(iso: string) {
    if (mode === "single") {
      onChange({ from: iso, to: "" });
      setOpen(null);
      return;
    }
    if (open === "to" && fromIso && !rangeDraft) {
      let from = fromIso;
      let to = iso;
      if (isBeforeDay(parseIso(to)!, parseIso(from)!)) {
        [from, to] = [to, from];
      }
      onChange({ from, to });
      setOpen(null);
      return;
    }
    if (!rangeDraft) {
      setRangeDraft(iso);
      onChange({ from: iso, to: "" });
      return;
    }
    let from = rangeDraft;
    let to = iso;
    if (isBeforeDay(parseIso(to)!, parseIso(from)!)) {
      [from, to] = [to, from];
    }
    onChange({ from, to });
    setRangeDraft(null);
    setOpen(null);
  }

  function dayClass(cell: DayCell): string {
    const parts = ["cal-day", "drp-day"];
    if (cell.outside) parts.push("is-outside");
    const d = cell.date;
    if (sameDay(d, today)) parts.push("is-today");

    const from = parseIso(fromIso);
    const to = parseIso(toIso);
    const draft = rangeDraft ? parseIso(rangeDraft) : null;

    if (mode === "single") {
      if (from && sameDay(d, from)) parts.push("is-selected");
      return parts.join(" ");
    }

    const start = draft ?? from;
    const end = draft ? null : to;
    if (start && sameDay(d, start)) {
      parts.push("is-selected", "is-range-start");
      if (end && sameDay(d, end)) parts.push("is-range-end");
    } else if (end && sameDay(d, end)) {
      parts.push("is-selected", "is-range-end");
    } else if (start && end && isBeforeDay(start, d) && isBeforeDay(d, end)) {
      parts.push("is-in-range");
    }
    return parts.join(" ");
  }

  function renderField(field: "from" | "to", label: string, iso: string, placeholder: string) {
    const isOpen = open === field;
    return (
      <div className="field field-flush drp-field">
        <label htmlFor={`${uid}-${field}`}>{label}</label>
        <button
          id={`${uid}-${field}`}
          ref={(el) => {
            triggerRefs.current[field] = el;
          }}
          type="button"
          className={`input-trigger drp-trigger${isOpen ? " is-open" : ""}`}
          disabled={disabled}
          aria-haspopup="dialog"
          aria-expanded={isOpen}
          onClick={() => setOpen(isOpen ? null : field)}
        >
          <span className={`drp-trigger-value${iso ? "" : " is-empty"}`}>
            {iso ? formatDisplayDate(iso) : placeholder}
          </span>
          <span className="drp-trigger-icon" aria-hidden>
            ▾
          </span>
        </button>
      </div>
    );
  }

  const openLabel =
    open === "to" ? toLabel : mode === "single" ? singleLabel : fromLabel;

  const calendar =
    open && typeof document !== "undefined"
      ? createPortal(
          <div
            ref={popRef}
            className="popover popover--lg popover--pad drp-popover"
            role="dialog"
            aria-label={openLabel}
          >
            <div className="drp-nav">
              <button
                type="button"
                className="btn btn-square"
                aria-label={t("dateCalendar.prevMonthAria")}
                onClick={() => setView((v) => addMonths(v, -1))}
              >
                ‹
              </button>
              <div className="drp-nav-title">
                {MONTH_NAMES_RU[view.getMonth()]} {view.getFullYear()}
              </div>
              <button
                type="button"
                className="btn btn-square"
                aria-label={t("dateCalendar.nextMonthAria")}
                onClick={() => setView((v) => addMonths(v, 1))}
              >
                ›
              </button>
            </div>
            <div className="drp-weekdays">
              {WEEKDAYS_RU.map((w) => (
                <span key={w}>{w}</span>
              ))}
            </div>
            <div className="drp-grid">
              {cells.map((cell) => (
                <button
                  key={cell.iso + (cell.outside ? "-o" : "")}
                  type="button"
                  className={dayClass(cell)}
                  onClick={() => pickDay(cell.iso)}
                >
                  {cell.date.getDate()}
                </button>
              ))}
            </div>
            {mode === "range" ? (
              <p className="drp-hint">
                {rangeDraft
                  ? t("dateCalendar.pickEndHint")
                  : t("dateCalendar.pickRangeHint")}
              </p>
            ) : null}
          </div>,
          document.body,
        )
      : null;

  return (
    <div className={`drp${className ? ` ${className}` : ""}`} ref={rootRef}>
      {mode === "single" ? (
        renderField("from", singleLabel, fromIso, t("dateCalendar.placeholder"))
      ) : (
        <>
          {renderField("from", fromLabel, fromIso, t("dateCalendar.placeholder"))}
          {renderField("to", toLabel, toIso, t("dateCalendar.placeholder"))}
        </>
      )}
      {calendar}
    </div>
  );
}
