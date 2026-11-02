import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import type { CalcTip } from "./buildCalcTips";
import { formatCalcDisplay, unformatCalcNumber } from "./formatCalcNumber";
import { t } from "../../i18n";

export function Section({
  title,
  tone,
  success,
  children,
}: {
  title: string;
  tone: string;
  success?: boolean;
  children: ReactNode;
}) {
  return (
    <section
      className={`calc-section calc-section--${tone}${success ? " calc-section--success" : ""}`}
    >
      <div className="calc-section-title">{title}</div>
      {children}
    </section>
  );
}

function MetricTipButton({
  tipId,
  tip,
  openTipId,
  setOpenTipId,
}: {
  tipId: string;
  tip: CalcTip;
  openTipId: string | null;
  setOpenTipId: (id: string | null) => void;
}) {
  const open = openTipId === tipId;
  const tipRootRef = useRef<HTMLDivElement>(null);
  const btnRef = useRef<HTMLButtonElement>(null);
  const popRef = useRef<HTMLDivElement>(null);
  const formula = tip.layout === "formula";

  useLayoutEffect(() => {
    if (!open || !btnRef.current || !popRef.current) return;
    const btn = btnRef.current.getBoundingClientRect();
    const pop = popRef.current;
    const pad = 8;
    const maxW = Math.min(formula ? 720 : 420, window.innerWidth - pad * 2);
    pop.style.position = "fixed";
    pop.style.maxWidth = `${maxW}px`;
    pop.style.width = "max-content";
    pop.style.right = "auto";
    pop.style.top = "0";
    pop.style.left = "0";
    const w = Math.min(pop.offsetWidth || maxW, maxW);
    let left = btn.right - w;
    if (left < pad) left = pad;
    if (left + w > window.innerWidth - pad) left = Math.max(pad, window.innerWidth - pad - w);
    let top = btn.bottom + 4;
    const h = pop.offsetHeight;
    if (top + h > window.innerHeight - pad) {
      top = Math.max(pad, btn.top - 4 - h);
    }
    pop.style.left = `${left}px`;
    pop.style.top = `${top}px`;
    pop.style.maxWidth = `${maxW}px`;
  }, [open, tip, formula]);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpenTipId(null);
    }
    function onPointer(e: MouseEvent) {
      const root = tipRootRef.current;
      if (root && !root.contains(e.target as Node)) setOpenTipId(null);
    }
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onPointer);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onPointer);
    };
  }, [open, setOpenTipId]);

  return (
    <div
      className={`calc-metric-tip${open ? " calc-metric-tip--open" : ""}`}
      ref={tipRootRef}
    >
      <button
        ref={btnRef}
        type="button"
        className="btn-tip"
        aria-label={t("calculator.view.howCalculatedAria")}
        title={t("calculator.view.howCalculatedAria")}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setOpenTipId(open ? null : tipId);
        }}
      >
        ?
      </button>
      {open && (
        <div
          ref={popRef}
          className={`popover popover--pad calc-metric-tip-popover${formula ? " calc-metric-tip-popover--formula" : ""}`}
          role="dialog"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="calc-metric-tip-title">{tip.title}</div>
          <ul className={`calc-metric-tip-lines${formula ? " calc-metric-tip-lines--formula" : ""}`}>
            {tip.lines.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

export type TipProps = {
  tipId?: string;
  tip?: CalcTip;
  openTipId?: string | null;
  setOpenTipId?: (id: string | null) => void;
};

export function Metric({
  label,
  value,
  numeric = true,
  maxFractionDigits,
  suffix,
  tipId,
  tip,
  openTipId,
  setOpenTipId,
}: {
  label: string;
  value?: string;
  numeric?: boolean;
  /** Как остатки: 0 = целое без дробной части. */
  maxFractionDigits?: number;
  /** Суффикс после значения (например `%` для скидки). */
  suffix?: string;
} & TipProps) {
  const formatted = numeric
    ? (formatCalcDisplay(value, maxFractionDigits != null ? { maxFractionDigits } : undefined) ??
      value)
    : value;
  const trimmed = formatted?.trim() ?? "";
  const display = trimmed ? `${trimmed}${suffix ?? ""}` : "";
  return (
    <div className="plate plate--tint calc-metric">
      {tip && tipId && setOpenTipId != null ? (
        <MetricTipButton
          tipId={tipId}
          tip={tip}
          openTipId={openTipId ?? null}
          setOpenTipId={setOpenTipId}
        />
      ) : null}
      <div className="calc-metric-label">{label}</div>
      <div className={"calc-metric-value" + (numeric ? " calc-metric-value--num" : "")}>
        {display || "—"}
      </div>
    </div>
  );
}

export function EditableMetric({
  label,
  value,
  dirty,
  suffix,
  disabled,
  onChange,
  tipId,
  tip,
  openTipId,
  setOpenTipId,
}: {
  label: string;
  value?: string;
  dirty?: boolean;
  suffix?: string;
  disabled?: boolean;
  onChange: (next: string) => void;
} & TipProps) {
  const [focused, setFocused] = useState(false);
  const formatted = formatCalcDisplay(value) ?? value ?? "";
  const display = focused
    ? (value ?? "")
    : formatted
      ? `${formatted}${suffix ?? ""}`
      : "";
  return (
    <label
      className={
        "plate plate--tint calc-metric calc-metric--editable" +
        (dirty ? " plate--dirty" : "")
      }
    >
      {tip && tipId && setOpenTipId != null ? (
        <MetricTipButton
          tipId={tipId}
          tip={tip}
          openTipId={openTipId ?? null}
          setOpenTipId={setOpenTipId}
        />
      ) : null}
      <div className="calc-metric-label">{label}</div>
      <input
        className="input-plain calc-metric-input"
        type="text"
        inputMode="decimal"
        value={display}
        placeholder="—"
        disabled={disabled}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        onChange={(e) => onChange(unformatCalcNumber(e.target.value))}
      />
    </label>
  );
}

export function ProfitCell({
  title,
  value,
  tipId,
  tip,
  openTipId,
  setOpenTipId,
}: {
  title: string;
  value: string;
} & TipProps) {
  return (
    <div className="plate calc-profit-block">
      {tip && tipId && setOpenTipId != null ? (
        <MetricTipButton
          tipId={tipId}
          tip={tip}
          openTipId={openTipId ?? null}
          setOpenTipId={setOpenTipId}
        />
      ) : null}
      <div className="calc-profit-label">{title}</div>
      <div className="calc-profit-value">{value}</div>
    </div>
  );
}

export function fmtDim(
  raw: string | undefined,
  unit: string,
  opts?: { maxFractionDigits?: number },
): string | undefined {
  const t = (raw ?? "").trim();
  if (!t) return undefined;
  const formatted = formatCalcDisplay(t, opts) ?? t;
  return `${formatted} ${unit}`;
}
