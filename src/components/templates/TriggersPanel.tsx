import { useState } from "react";
import type { TimeTriggerDto } from "../../bridge/hostApi";
import { t as tr, useT } from "../../i18n";
import TimeTriggerEditorDialog from "./TimeTriggerEditorDialog";
import "./templates-dialogs.css";

type Props = {
  triggers: TimeTriggerDto[];
  onChange: (next: TimeTriggerDto[]) => void;
  disabled?: boolean;
  /** Заголовок рисует родительская card. */
  hideHeading?: boolean;
};

function typeLabel(trigger: TimeTriggerDto): string {
  const typ = trigger.type_display_name || trigger.trigger_type || trigger.type || "";
  const map: Record<string, string> = {
    Once: tr("templates.triggers.typeOnce"),
    Daily: tr("templates.triggers.typeDaily"),
    Weekly: tr("templates.triggers.typeWeekly"),
    Interval: tr("templates.triggers.interval"),
  };
  return map[String(typ)] || String(typ) || "—";
}

function settingsLabel(trigger: TimeTriggerDto): string {
  if (trigger.settings_display_name) return trigger.settings_display_name;
  if (trigger.scheduled_date_time) return String(trigger.scheduled_date_time);
  if (trigger.daily_time) return tr("templates.triggers.settingsDaily", { time: trigger.daily_time });
  if (trigger.weekly_day) {
    return `${trigger.weekly_day} ${trigger.weekly_time || ""}`;
  }
  if (trigger.interval_minutes != null) {
    return tr("templates.triggers.settingsInterval", { minutes: trigger.interval_minutes });
  }
  return "—";
}

export default function TriggersPanel({
  triggers,
  onChange,
  disabled,
  hideHeading = false,
}: Props) {
  const t = useT();
  const [editor, setEditor] = useState<{ initial: TimeTriggerDto | null } | null>(null);
  const enabled = triggers.filter((row) => row.is_enabled !== false).length;
  const info =
    triggers.length === 0
      ? t("templates.triggers.notConfigured")
      : t("templates.triggers.configured", { enabled, total: triggers.length });

  return (
    <div className="tpl-triggers">
      <div className={"tpl-trigger-heading-row" + (hideHeading ? " tpl-trigger-heading-row--actions" : "")}>
        {hideHeading ? null : <h3 className="tpl-trigger-heading">{t("templates.editor.timeTriggers")}</h3>}
        <button
          type="button"
          className="btn btn-outline btn-sm"
          disabled={disabled}
          onClick={() => setEditor({ initial: null })}
        >
          {t("templates.triggers.add")}
        </button>
      </div>
      <div className="tpl-triggers-table-wrap">
        <table className="data-table data-table--head-bg data-table--no-last-border tpl-triggers-table">
          <thead>
            <tr>
              <th>{t("common.type")}</th>
              <th>{t("common.settings")}</th>
              <th>{t("common.enabledShort")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {triggers.length === 0 ? (
              <tr>
                <td colSpan={4} className="page-sub">
                  {t("templates.triggers.none")}
                </td>
              </tr>
            ) : (
              triggers.map((row) => (
                <tr key={row.id}>
                  <td>{typeLabel(row)}</td>
                  <td>{settingsLabel(row)}</td>
                  <td>{row.is_enabled === false ? t("common.noWord") : t("common.yesWord")}</td>
                  <td>
                    <div className="toolbar tpl-trigger-actions">
                      <button
                        type="button"
                        className="btn btn-ghost"
                        disabled={disabled}
                        onClick={() => setEditor({ initial: row })}
                      >
                        {t("common.edit")}
                      </button>
                      <button
                        type="button"
                        className="btn btn-danger"
                        disabled={disabled}
                        onClick={() => onChange(triggers.filter((x) => x.id !== row.id))}
                      >
                        {t("common.delete")}
                      </button>
                    </div>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      <p className="page-sub">{info}</p>

      {editor ? (
        <TimeTriggerEditorDialog
          initial={editor.initial}
          onCancel={() => setEditor(null)}
          onSave={(tr) => {
            const idx = triggers.findIndex((x) => x.id === tr.id);
            if (idx >= 0) {
              const next = [...triggers];
              next[idx] = tr;
              onChange(next);
            } else {
              onChange([...triggers, tr]);
            }
            setEditor(null);
          }}
        />
      ) : null}
    </div>
  );
}
