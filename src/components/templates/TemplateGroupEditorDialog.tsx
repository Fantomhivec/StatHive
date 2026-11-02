import { useCallback, useEffect, useMemo, useState } from "react";
import type { TemplateDto, TemplateGroupDto, TimeTriggerDto } from "../../bridge/hostApi";
import { hostApi } from "../../bridge/hostApi";
import TriggersPanel from "./TriggersPanel";
import "./templates-dialogs.css";
import { useT } from "../../i18n";

type Props = {
  marketplace: "wb" | "ozon";
  tabName: string;
  group: TemplateGroupDto | null;
  tabTemplates: TemplateDto[];
  onClose: () => void;
  onSaved: () => void;
};

export default function TemplateGroupEditorDialog({
  marketplace,
  tabName,
  group,
  tabTemplates,
  onClose,
  onSaved,
}: Props) {
  const t = useT();
  const [name, setName] = useState(group?.name ?? t("templates.groupEditor.newGroupDefault"));
  const [ids, setIds] = useState(() => new Set(group?.template_ids ?? []));
  const [triggers, setTriggers] = useState<TimeTriggerDto[]>(() => group?.time_triggers ?? []);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const sorted = useMemo(
    () => [...tabTemplates].sort((a, b) => a.name.localeCompare(b.name, "ru")),
    [tabTemplates],
  );

  function toggle(id: string) {
    setIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  const save = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      await hostApi.templatesSaveGroup({
        id: group?.id ?? crypto.randomUUID(),
        name: name.trim(),
        marketplace,
        tab_name: tabName,
        template_ids: [...ids],
        is_expanded: group?.is_expanded ?? true,
        last_run_status: group?.last_run_status ?? "",
        last_run_at: group?.last_run_at ?? "",
        time_triggers: triggers,
      });
      onSaved();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  }, [
    group,
    name,
    marketplace,
    tabName,
    ids,
    triggers,
    onSaved,
    onClose,
  ]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (!saving && name.trim()) void save();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose, save, saving, name]);

  return (
    <div className="overlay overlay--win" role="presentation" onClick={onClose}>
      <div
        className="dialog-panel dialog-panel--win tpl-dialog"
        role="dialog"
        aria-labelledby="tpl-grp-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-header">
          <h2 id="tpl-grp-title">{group ? t("templates.groupEditor.editTitle") : t("templates.groupEditor.newGroupDefault")}</h2>
          <button type="button" className="btn btn-ghost" onClick={onClose}>
            {t("common.close")}
          </button>
        </div>
        <div className="tpl-modal-body">
          {error ? <p className="error">{error}</p> : null}
          <div className="field">
            <label htmlFor="grp-name">{t("templates.groupEditor.groupName")}</label>
            <input
              id="grp-name"
              value={name}
              disabled={saving}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <p className="page-sub">{t("templates.groupEditor.tabTemplates", { tab: tabName })}</p>
          <p className="tpl-picker-hint">{t("templates.groupEditor.pickHint")}</p>
          <div className="tpl-picker-list tpl-picker-list--fill">
            {sorted.length === 0 ? (
              <p className="page-sub">{t("templates.groupEditor.noTemplatesOnTab")}</p>
            ) : (
              sorted.map((t) => (
                <label key={t.id} className="pick-row tpl-picker-item tpl-picker-item--static">
                  <input
                    type="checkbox"
                    className="tpl-picker-check"
                    checked={ids.has(t.id)}
                    disabled={saving}
                    onChange={() => toggle(t.id)}
                  />
                  <span className="toggle-track" aria-hidden />
                  <span className="tpl-picker-label">{t.name}</span>
                </label>
              ))
            )}
          </div>

          <TriggersPanel
            triggers={triggers}
            disabled={saving}
            onChange={setTriggers}
          />
        </div>
        <div className="dialog-footer tpl-modal-footer">
          <button type="button" className="btn btn-ghost" disabled={saving} onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={saving || !name.trim()}
            onClick={() => void save()}
          >
            {saving ? t("common.saving") : t("common.save")}
          </button>
        </div>
      </div>
    </div>
  );
}
