import { useMemo } from "react";
import type { TemplateDto, TimeTriggerDto } from "../../bridge/hostApi";
import AppComboBox from "../AppComboBox";
import type { ScriptVisibility } from "./scriptVisibility";
import TriggersPanel from "./TriggersPanel";
import { useT } from "../../i18n";

type Props = {
  tpl: TemplateDto;
  visibility: ScriptVisibility;
  saving: boolean;
  onPatch: (partial: Partial<TemplateDto>) => void;
  onOpenExport: () => void;
};

export default function TemplateEditorWriteCards({
  tpl,
  visibility,
  saving,
  onPatch,
  onOpenExport,
}: Props) {
  const t = useT();

  const syncInfo = useMemo(() => {
    const sync = tpl.sync_settings;
    if (!sync?.enable_change_sync) return t("templates.editor.syncOff");
    const fields = tpl.selected_fields ?? [];
    const titleOf = (name?: string) => {
      const n = (name ?? "").trim();
      if (!n) return null;
      const f = fields.find((x) => x.name.toLowerCase() === n.toLowerCase());
      return f ? f.title || f.name : n;
    };
    const keys = [titleOf(sync.key_field_1), titleOf(sync.key_field_2)].filter(
      Boolean,
    ) as string[];
    const keysText = keys.length > 0 ? keys.join(" + ") : t("templates.editor.syncKeysNone");
    const deleteMode = sync.do_not_delete_old_rows
      ? t("templates.editor.syncDeleteKeep")
      : t("templates.editor.syncDeleteRemove");
    return t("templates.editor.syncOn", { keys: keysText, deleteMode });
  }, [tpl.sync_settings, tpl.selected_fields, t]);

  return (
    <>
      {!visibility.hideWriteSettings ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.editor.writeSettings")}</h3>
          <div className="tpl-write-row">
            <div className="field tpl-write-field">
              <label>{t("templates.editor.sortBy")}</label>
              <AppComboBox
                options={[
                  { value: "", label: "—" },
                  ...(tpl.selected_fields ?? [])
                    .filter((f) => f.selected)
                    .map((f) => ({
                      value: f.name,
                      label: f.title || f.name,
                    })),
                ]}
                value={tpl.write_sort_field_name ?? ""}
                disabled={saving}
                onChange={(v) => onPatch({ write_sort_field_name: v })}
              />
            </div>
            <div className="field tpl-write-field tpl-write-field--dir">
              <label>{t("templates.editor.sortOrder")}</label>
              <AppComboBox
                options={[
                  { value: "asc", label: t("templates.editor.sortAsc") },
                  { value: "desc", label: t("templates.editor.sortDesc") },
                ]}
                value={
                  (tpl.write_sort_direction ?? "asc").toLowerCase() === "desc"
                    ? "desc"
                    : "asc"
                }
                disabled={saving || !(tpl.write_sort_field_name ?? "").trim()}
                onChange={(v) => onPatch({ write_sort_direction: v })}
              />
            </div>
          </div>
          {!tpl.export_to_excel ? (
            <label className="toggle-switch tpl-toggle-row tpl-write-clear-toggle">
              <input
                type="checkbox"
                checked={!!tpl.clear_only_written_range}
                disabled={saving}
                onChange={(e) => onPatch({ clear_only_written_range: e.target.checked })}
              />
              <span className="toggle-track" />
              <span className="tpl-toggle-label">
                {t("templates.editor.clearWrittenRangeOnly")}
              </span>
            </label>
          ) : null}
        </div>
      ) : null}

      {!visibility.hideWriteSettings && !tpl.export_to_excel ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.editor.changeSync")}</h3>
          <p className="tpl-info">{syncInfo}</p>
          <button
            type="button"
            className="btn btn-outline"
            disabled={saving}
            onClick={onOpenExport}
          >
            {t("templates.editor.configureSync")}
          </button>
        </div>
      ) : null}

      <div className="tpl-dialog-card">
        <h3 className="tpl-dialog-card-header">{t("templates.editor.timeTriggers")}</h3>
        {tpl.sync_settings?.enable_change_sync && !tpl.export_to_excel ? (
          <p className="page-sub tpl-export-hint">{t("templates.editor.syncTriggersHint")}</p>
        ) : null}
        <TriggersPanel
          hideHeading
          triggers={(tpl.time_triggers ?? []) as TimeTriggerDto[]}
          disabled={saving}
          onChange={(time_triggers) => onPatch({ time_triggers })}
        />
      </div>
    </>
  );
}
