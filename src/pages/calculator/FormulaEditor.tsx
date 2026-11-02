import { useMemo, type CSSProperties } from "react";
import {
  CONTENT_TOKENS,
  DIGITS,
  OPS,
  SETTINGS_TOKENS,
} from "./calculatorTypes";
import { parseFormulaSegments, spaceFormulaOperators } from "./formulaTokens";
import { t } from "../../i18n";

function FormulaColoredView({ value }: { value: string }) {
  const segments = useMemo(() => {
    const spaced = spaceFormulaOperators(value);
    return parseFormulaSegments(spaced);
  }, [value]);
  if (!value?.trim()) {
    return (
      <div className="input wb-formula-colored muted">{t("calculator.formula.empty")}</div>
    );
  }
  return (
    <div className="input wb-formula-colored" aria-hidden>
      {segments.map((seg, i) =>
        seg.kind === "token" ? (
          <span
            key={`${seg.tag}-${i}`}
            className="chip"
            style={{ "--chip-color": seg.color } as CSSProperties}
          >
            {seg.display}
          </span>
        ) : (
          <span key={`t-${i}`} className="wb-formula-op">
            {seg.text}
          </span>
        ),
      )}
    </div>
  );
}

export function FormulaEditor({
  label,
  value,
  disabled,
  onChange,
  onBlur,
  onClear,
}: {
  label: string;
  value: string;
  disabled?: boolean;
  onChange: (v: string) => void;
  onBlur: (v: string) => void;
  onClear: () => void;
}) {
  function commitSpaced() {
    const spaced = spaceFormulaOperators(value);
    if (spaced !== value) onChange(spaced);
    onBlur(spaced);
  }
  return (
    <div className="wb-formula-editor">
      <div className="wb-formula-row">
        <span>{label}</span>
        <button type="button" className="btn btn-outline btn-sm" disabled={disabled} onClick={onClear}>
          {t("common.clear")}
        </button>
      </div>
      <FormulaColoredView value={value} />
      <input
        className="input-compact wb-formula-input"
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        onBlur={commitSpaced}
        aria-label={label}
        spellCheck={false}
      />
    </div>
  );
}

function TokenChipButton({
  label,
  color,
  disabled,
  onClick,
}: {
  label: string;
  color: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className="chip chip-btn"
      style={{ "--chip-color": color } as CSSProperties}
      disabled={disabled}
      onClick={onClick}
    >
      {label}
    </button>
  );
}

export function FormulaTokens({
  busy,
  onToken,
}: {
  busy?: boolean;
  onToken: (t: string) => void;
}) {
  return (
    <div className="wb-token-block">
      <strong>{t("calculator.formula.groups.settings")}</strong>
      <div className="wb-token-row">
        {SETTINGS_TOKENS.map((t) => (
          <TokenChipButton
            key={t.tag}
            label={t.label}
            color={t.color}
            disabled={busy}
            onClick={() => onToken(t.insert)}
          />
        ))}
      </div>
      <strong>{t("calculator.formula.groups.content")}</strong>
      <div className="wb-token-row">
        {CONTENT_TOKENS.map((t) => (
          <TokenChipButton
            key={t.tag}
            label={t.label}
            color={t.color}
            disabled={busy}
            onClick={() => onToken(t.insert)}
          />
        ))}
      </div>
      <strong>{t("calculator.formula.groups.ops")}</strong>
      <div className="wb-token-row">
        {OPS.map((t) => (
          <button
            key={t}
            type="button"
            className="btn btn-outline btn-sm"
            disabled={busy}
            onClick={() => onToken(t)}
          >
            {t}
          </button>
        ))}
      </div>
      <strong>{t("calculator.formula.groups.numbers")}</strong>
      <div className="wb-token-row">
        {DIGITS.map((t) => (
          <button
            key={t}
            type="button"
            className="btn btn-outline btn-sm"
            disabled={busy}
            onClick={() => onToken(t)}
          >
            {t}
          </button>
        ))}
      </div>
    </div>
  );
}
