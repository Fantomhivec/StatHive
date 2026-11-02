import { useMemo, useState } from "react";
import "./templates-dialogs.css";
import { useT } from "../../i18n";

export type CabinetOption = {
  id: string;
  cabinet: string;
};

type Props = {
  options: CabinetOption[];
  selected: string[];
  onCancel: () => void;
  onApply: (cabinets: string[]) => void;
};

/** Выбор кабинетов — та же раскладка, что у FieldSelectorDialog (список flex:1, футер снизу). */
export default function CabinetSelectorDialog({
  options,
  selected,
  onCancel,
  onApply,
}: Props) {
  const t = useT();
  const [picked, setPicked] = useState(() => new Set(selected));
  const [hideUnselected, setHideUnselected] = useState(false);

  const visible = useMemo(
    () => (hideUnselected ? options.filter((o) => picked.has(o.cabinet)) : options),
    [options, hideUnselected, picked],
  );

  function toggle(name: string) {
    setPicked((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }

  function selectAll(on: boolean) {
    setPicked(on ? new Set(options.map((o) => o.cabinet)) : new Set());
  }

  return (
    <div className="overlay overlay--win overlay--nested" role="presentation" onClick={onCancel}>
      <div
        className="dialog-panel dialog-panel--win tpl-dialog tpl-dialog--picker"
        role="dialog"
        aria-labelledby="tpl-cabs-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-header">
          <h2 id="tpl-cabs-title">{t("common.cabinets")}</h2>
          <button type="button" className="btn btn-ghost" onClick={onCancel}>
            {t("common.close")}
          </button>
        </div>
        <p className="tpl-picker-hint">{t("templates.cabinetsDialog.hint")}</p>
        <div className="tpl-picker-list tpl-picker-list--fill">
          {options.length === 0 ? (
            <p className="page-sub">{t("templates.cabinetsDialog.noCabinetsGoKeys")}</p>
          ) : visible.length === 0 ? (
            <p className="page-sub">{t("templates.cabinetsDialog.noSelected")}</p>
          ) : (
            visible.map((o) => (
              <label key={o.id} className="pick-row tpl-picker-item tpl-picker-item--static">
                <input
                  type="checkbox"
                  className="tpl-picker-check"
                  checked={picked.has(o.cabinet)}
                  onChange={() => toggle(o.cabinet)}
                />
                <span className="toggle-track" aria-hidden />
                <span className="tpl-picker-label">{o.cabinet}</span>
              </label>
            ))
          )}
        </div>
        <div className="dialog-footer tpl-modal-footer tpl-field-selector-footer">
          <label className="toggle-switch tpl-toggle-row tpl-hide-toggle">
            <input
              type="checkbox"
              checked={hideUnselected}
              onChange={(e) => setHideUnselected(e.target.checked)}
            />
            <span className="toggle-track" />
            <span className="tpl-toggle-label">{t("common.hideUnselected")}</span>
          </label>
          <button type="button" className="btn btn-outline" onClick={() => selectAll(true)}>
            {t("common.all")}
          </button>
          <button type="button" className="btn btn-outline" onClick={() => selectAll(false)}>
            {t("common.deselect")}
          </button>
          <button type="button" className="btn btn-outline" onClick={onCancel}>
            {t("common.cancel")}
          </button>
          <button type="button" className="btn btn-primary" onClick={() => onApply([...picked])}>
            {t("templates.cabinetsDialog.applyCount", { count: picked.size })}
          </button>
        </div>
      </div>
    </div>
  );
}
