import { useMemo, useRef, useState, type ReactNode } from "react";
import type { FieldRow } from "./templateNormalize";
import { moveIdToIndex, usePointerSortableList } from "./usePointerSortableList";
import "./templates-dialogs.css";
import { useT } from "../../i18n";

type Props = {
  items: FieldRow[];
  onCancel: () => void;
  onApply: (ordered: FieldRow[]) => void;
};

export default function FieldSelectorDialog({ items, onCancel, onApply }: Props) {
  const t = useT();
  const [rows, setRows] = useState(() => items.map((x) => ({ ...x })));
  const [hideUnselected, setHideUnselected] = useState(false);
  const listRef = useRef<HTMLDivElement | null>(null);

  const visible = useMemo(
    () => (hideUnselected ? rows.filter((r) => r.selected) : rows),
    [rows, hideUnselected],
  );
  const visibleIds = useMemo(() => visible.map((r) => r.name), [visible]);

  const { activeId, onItemPointerDown } = usePointerSortableList(listRef, {
    ids: visibleIds,
    onReorder: (fromId, toIndex) => {
      setRows((prev) => reorderByVisibleIndex(prev, hideUnselected, fromId, toIndex));
    },
  });

  const indexByName = useMemo(() => {
    const m = new Map<string, number>();
    rows.forEach((r, i) => m.set(r.name, i));
    return m;
  }, [rows]);

  function renderRow(r: FieldRow) {
    const fullIdx = indexByName.get(r.name) ?? -1;
    const isPlaceholder = activeId === r.name;
    return (
      <label
        key={r.name}
        data-sortable-id={r.name}
        data-field-name={r.name}
        className={
          "pick-row tpl-picker-item tpl-picker-item--sortable" +
          (isPlaceholder ? " tpl-picker-item--placeholder pick-item--placeholder" : "")
        }
        onPointerDown={(e) => onItemPointerDown(e, r.name)}
      >
        <input
          type="checkbox"
          className="tpl-picker-check"
          checked={r.selected}
          onChange={() =>
            setRows((prev) =>
              prev.map((x, i) => (i === fullIdx ? { ...x, selected: !x.selected } : x)),
            )
          }
        />
        <span className="toggle-track" aria-hidden />
        <span className="tpl-picker-label">{r.title || r.name}</span>
      </label>
    );
  }

  const nodes: ReactNode[] = visible.map((r) => renderRow(r));

  return (
    <div className="overlay overlay--win overlay--nested" role="presentation" onClick={onCancel}>
      <div
        className="dialog-panel dialog-panel--win tpl-dialog tpl-dialog--picker"
        role="dialog"
        aria-labelledby="tpl-fields-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-header">
          <h2 id="tpl-fields-title">{t("templates.fieldsDialog.title")}</h2>
          <button type="button" className="btn btn-ghost" onClick={onCancel}>
            {t("common.close")}
          </button>
        </div>
        <p className="tpl-picker-hint">
          {t("templates.fieldsDialog.hint")}
        </p>
        <div
          ref={listRef}
          className={
            "tpl-picker-list tpl-picker-list--fill" +
            (activeId ? " tpl-picker-list--dragging" : "")
          }
        >
          {nodes}
          {visible.length === 0 ? <p className="page-sub">{t("templates.fieldsDialog.noFields")}</p> : null}
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
          <button
            type="button"
            className="btn btn-outline"
            onClick={() => setRows((prev) => prev.map((x) => ({ ...x, selected: true })))}
          >
            {t("common.all")}
          </button>
          <button
            type="button"
            className="btn btn-outline"
            onClick={() => setRows((prev) => prev.map((x) => ({ ...x, selected: false })))}
          >
            {t("common.deselect")}
          </button>
          <button type="button" className="btn btn-outline" onClick={onCancel}>
            {t("common.cancel")}
          </button>
          <button type="button" className="btn btn-primary" onClick={() => onApply(rows)}>
            {t("common.apply")}
          </button>
        </div>
      </div>
    </div>
  );
}

function reorderByVisibleIndex(
  prev: FieldRow[],
  hideUnselected: boolean,
  movedName: string,
  toVisibleIndex: number,
): FieldRow[] {
  const visibleNames = prev.filter((r) => !hideUnselected || r.selected).map((r) => r.name);
  const nextVis = moveIdToIndex(visibleNames, movedName, toVisibleIndex);
  if (nextVis === visibleNames) return prev;

  const byName = new Map(prev.map((r) => [r.name, r]));
  const used = new Set<string>();
  const result: FieldRow[] = [];
  let vi = 0;
  for (const row of prev) {
    const isVis = !hideUnselected || row.selected;
    if (!isVis) {
      result.push(row);
      continue;
    }
    const name = nextVis[vi++];
    const item = byName.get(name);
    if (item && !used.has(name)) {
      result.push(item);
      used.add(name);
    }
  }
  while (vi < nextVis.length) {
    const name = nextVis[vi++];
    const item = byName.get(name);
    if (item && !used.has(name)) {
      result.push(item);
      used.add(name);
    }
  }
  return result;
}
