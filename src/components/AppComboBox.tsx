import { useEffect, useId, useMemo, useRef, useState } from "react";
import { useT } from "../i18n";

export type AppComboOption = {
  value: string;
  label: string;
  secondary?: string;
};

export type AppComboGroup = {
  label: string;
  options: AppComboOption[];
};

type Props = {
  value: string;
  options?: AppComboOption[];
  groups?: AppComboGroup[];
  onChange: (value: string) => void;
  editable?: boolean;
  disabled?: boolean;
  title?: string;
  className?: string;
  placeholder?: string;
  displayValue?: string;
};

/** Тематический combobox (в т.ч. grouped). */
export default function AppComboBox({
  value,
  options = [],
  groups,
  onChange,
  editable = false,
  disabled = false,
  title,
  className,
  placeholder,
  displayValue,
}: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const rootRef = useRef<HTMLDivElement>(null);
  const listId = useId();

  const flatOptions = useMemo(() => {
    if (groups && groups.length > 0) return groups.flatMap((g) => g.options);
    return options;
  }, [groups, options]);

  useEffect(() => {
    if (!open) return;
    function onDoc(e: MouseEvent) {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  useEffect(() => {
    if (!open || !groups?.length || !value) return;
    const g = groups.find((x) => x.options.some((o) => o.value === value));
    if (!g) return;
    setExpanded((prev) => {
      if (prev.has(g.label)) return prev;
      const next = new Set(prev);
      next.add(g.label);
      return next;
    });
  }, [open, groups, value]);

  const selected = flatOptions.find((o) => o.value === value);
  const closedLabel =
    displayValue ??
    (selected
      ? selected.secondary
        ? `${selected.label} ${selected.secondary}`
        : selected.label
      : value);

  function toggleGroup(label: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(label)) next.delete(label);
      else next.add(label);
      return next;
    });
  }

  function renderOption(o: AppComboOption) {
    const isSelected = o.value === value || o.label === value;
    return (
      <li
        key={o.value}
        role="option"
        aria-selected={isSelected}
        className={
          "app-combo-item" +
          (isSelected ? " selected" : "") +
          (groups ? " app-combo-item-script" : "")
        }
        onMouseDown={(e) => {
          e.preventDefault();
          onChange(editable ? o.label : o.value);
          setOpen(false);
        }}
      >
        <span className="app-combo-item-label">{o.label}</span>
        {o.secondary ? <span className="app-combo-item-secondary">{o.secondary}</span> : null}
      </li>
    );
  }

  return (
    <div
      ref={rootRef}
      className={`app-combo${open ? " open" : ""}${disabled ? " disabled" : ""}${
        groups?.length ? " app-combo-grouped" : ""
      } ${className ?? ""}`.trim()}
      title={title}
    >
      <div className="app-combo-main">
        {editable ? (
          <input
            className="app-combo-input"
            value={value}
            disabled={disabled}
            placeholder={placeholder}
            onChange={(e) => onChange(e.target.value)}
            onFocus={() => !disabled && setOpen(true)}
            onClick={() => !disabled && setOpen(true)}
            aria-expanded={open}
            aria-controls={listId}
            role="combobox"
          />
        ) : (
          <button
            type="button"
            className="app-combo-display"
            disabled={disabled}
            onClick={() => !disabled && setOpen((v) => !v)}
            aria-expanded={open}
            aria-controls={listId}
            aria-haspopup="listbox"
          >
            {closedLabel || placeholder || ""}
          </button>
        )}
        <button
          type="button"
          className="app-combo-arrow"
          tabIndex={-1}
          disabled={disabled}
          aria-label={t("common.openListAria")}
          onClick={() => !disabled && setOpen((v) => !v)}
        >
          <span className="app-combo-arrow-glyph" />
        </button>
      </div>

      {open && !disabled ? (
        <ul id={listId} className="popover app-combo-dropdown" role="listbox">
          {groups && groups.length > 0 ? (
            groups.map((g) => {
              const isOpen = expanded.has(g.label);
              return (
                <li key={g.label} className="app-combo-group" role="presentation">
                  <button
                    type="button"
                    className="app-combo-group-header"
                    onMouseDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      toggleGroup(g.label);
                    }}
                  >
                    <span className="app-combo-group-chevron" aria-hidden>
                      {isOpen ? "▼" : "▶"}
                    </span>
                    <span className="app-combo-group-name">{g.label}</span>
                  </button>
                  {isOpen ? (
                    <ul className="app-combo-group-items">{g.options.map(renderOption)}</ul>
                  ) : null}
                </li>
              );
            })
          ) : options.length === 0 ? (
            <li className="app-combo-item muted">{t("common.noItems")}</li>
          ) : (
            options.map(renderOption)
          )}
        </ul>
      ) : null}
    </div>
  );
}
