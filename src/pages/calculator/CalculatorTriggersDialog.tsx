import { useMemo, useState } from "react";
import AppCustomWindow from "../../components/AppCustomWindow";
import TimeTriggerEditorDialog from "../../components/templates/TimeTriggerEditorDialog";
import type { TimeTriggerDto } from "../../bridge/hostApi.types";
import { useT } from "../../i18n";
import { MODE_OPTS, type TriggerRow } from "./calculatorTypes";

type Props = {
  open: boolean;
  busy: boolean;
  draft: TriggerRow[];
  onChange: (next: TriggerRow[]) => void;
  onSave: () => void;
  onClose: () => void;
};

function stripSeconds(time: string | null | undefined): string {
  const s = String(time ?? "09:00");
  const m = /^(\d{1,2}):(\d{2})/.exec(s);
  if (m) return `${m[1].padStart(2, "0")}:${m[2]}`;
  return "09:00";
}

function rowToEditorInitial(row: TriggerRow): TimeTriggerDto | null {
  if (!row.mode || row.mode === "None") return null;
  const type = row.mode;
  const totalMins = Math.max(0, row.intervalHours) * 60 + Math.max(0, row.intervalMinutes);
  return {
    id: "",
    template_id: "",
    type,
    trigger_type: type,
    is_enabled: true,
    scheduled_date_time: row.scheduledDateTime || null,
    daily_time: type === "Daily" ? `${stripSeconds(row.time)}:00` : null,
    weekly_day: type === "Weekly" ? row.weeklyDay || "Monday" : null,
    weekly_time:
      type === "Weekly" ? `${stripSeconds(row.weeklyTime || row.time)}:00` : null,
    interval_minutes: type === "Interval" ? totalMins || 60 : null,
    last_executed_at: null,
    created_at: new Date().toISOString(),
    next_execution_at: row.nextExecution || null,
    last_execution_status: null,
  };
}

function editorToRowPatch(dto: TimeTriggerDto): Partial<TriggerRow> {
  const type = String(dto.type ?? dto.trigger_type ?? "Daily");
  const patch: Partial<TriggerRow> = {
    mode: type,
    scheduledDateTime: "",
    weeklyDay: "Monday",
    weeklyTime: "09:00",
    time: "09:00",
    intervalHours: 0,
    intervalMinutes: 0,
  };
  if (type === "Once") {
    patch.scheduledDateTime = String(dto.scheduled_date_time ?? "");
  } else if (type === "Daily") {
    patch.time = stripSeconds(dto.daily_time);
  } else if (type === "Weekly") {
    patch.weeklyDay = String(dto.weekly_day || "Monday");
    patch.weeklyTime = stripSeconds(dto.weekly_time);
    patch.time = patch.weeklyTime;
  } else if (type === "Interval") {
    const mins = Math.max(1, Number(dto.interval_minutes ?? 60));
    patch.intervalHours = Math.floor(mins / 60);
    patch.intervalMinutes = mins % 60;
  }
  return patch;
}

function summarizeRow(
  row: TriggerRow,
  t: (key: string, params?: Record<string, string | number>) => string,
): string {
  const modeLabel =
    MODE_OPTS.find((o) => o.value === row.mode)?.label ??
    t("calculator.tables.triggersSummaryNone");
  if (!row.mode || row.mode === "None") {
    return t("calculator.tables.triggersSummaryNone");
  }
  if (row.mode === "Once") {
    return `${modeLabel}: ${row.scheduledDateTime || "—"}`;
  }
  if (row.mode === "Daily") {
    return `${modeLabel}: ${stripSeconds(row.time)}`;
  }
  if (row.mode === "Weekly") {
    const dayKey = `templates.triggers.weekdays.${row.weeklyDay || "Monday"}`;
    const day = t(dayKey);
    return `${modeLabel}: ${day} ${stripSeconds(row.weeklyTime || row.time)}`;
  }
  if (row.mode === "Interval") {
    const mins = Math.max(0, row.intervalHours) * 60 + Math.max(0, row.intervalMinutes);
    return `${modeLabel}: ${mins} ${t("calculator.tables.triggersMinutes")}`;
  }
  return modeLabel;
}

export function CalculatorTriggersDialog({
  open,
  busy,
  draft,
  onChange,
  onSave,
  onClose,
}: Props) {
  const t = useT();
  const [editIdx, setEditIdx] = useState<number | null>(null);

  const editing = useMemo(
    () => (editIdx != null ? draft[editIdx] : null),
    [draft, editIdx],
  );

  if (!open) return null;

  function patchRow(idx: number, patch: Partial<TriggerRow>) {
    const next = [...draft];
    next[idx] = { ...next[idx], ...patch };
    onChange(next);
  }

  function clearRow(idx: number) {
    patchRow(idx, {
      mode: "None",
      time: "09:00",
      intervalHours: 1,
      intervalMinutes: 0,
      scheduledDateTime: "",
      weeklyDay: "Monday",
      weeklyTime: "09:00",
      nextExecution: "",
    });
  }

  return (
    <>
      <AppCustomWindow
        title={t("calculator.tables.triggersDialogTitle")}
        onClose={onClose}
        width={720}
        bodyClassName="wb-triggers-dialog-body"
        footer={
          <>
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy}
              onClick={onSave}
            >
              {t("common.save")}
            </button>
            <button type="button" className="btn btn-outline" onClick={onClose}>
              {t("common.cancel")}
            </button>
          </>
        }
      >
        <p className="calc-hint">{t("calculator.tables.triggersHint")}</p>
        <div className="wb-triggers-list">
          {draft.length === 0 && (
            <p className="muted">{t("calculator.tables.triggersEmpty")}</p>
          )}
          {draft.map((row, idx) => (
            <div key={row.tableKey} className="plate plate--row wb-trigger-row">
              <div className="wb-trigger-name">{row.displayName}</div>
              <div className="wb-trigger-summary muted">{summarizeRow(row, t)}</div>
              <span className="muted">
                {t("calculator.tables.triggersNext", {
                  when:
                    row.nextExecution ||
                    (row.mode === "None"
                      ? t("calculator.tables.triggersNextDash")
                      : t("calculator.tables.triggersNextAfterSave")),
                })}
              </span>
              <div className="wb-trigger-actions">
                <button
                  type="button"
                  className="btn btn-outline btn-sm"
                  disabled={busy}
                  onClick={() => setEditIdx(idx)}
                >
                  {t("calculator.tables.triggersConfigure")}
                </button>
                {row.mode !== "None" ? (
                  <button
                    type="button"
                    className="btn btn-ghost btn-sm"
                    disabled={busy}
                    onClick={() => clearRow(idx)}
                  >
                    {t("calculator.tables.triggersClear")}
                  </button>
                ) : null}
              </div>
            </div>
          ))}
        </div>
      </AppCustomWindow>

      {editing && editIdx != null ? (
        <TimeTriggerEditorDialog
          initial={rowToEditorInitial(editing)}
          onCancel={() => setEditIdx(null)}
          onSave={(dto) => {
            patchRow(editIdx, editorToRowPatch(dto));
            setEditIdx(null);
          }}
        />
      ) : null}
    </>
  );
}
