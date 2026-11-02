import { useMemo, useState } from "react";
import AppComboBox from "../AppComboBox";
import AppCustomWindow from "../AppCustomWindow";
import DateRangePicker from "../DateRangePicker";
import { toIsoFromDate } from "../dateIso";
import type { TimeTriggerDto } from "../../bridge/hostApi";
import "./templates-dialogs.css";
import { t, useT } from "../../i18n";

type Props = {
  initial?: TimeTriggerDto | null;
  onCancel: () => void;
  onSave: (trigger: TimeTriggerDto) => void;
};

const TYPE_OPTS = [
  { value: "Once", label: t("templates.triggers.typeOnceLong") },
  { value: "Daily", label: t("templates.triggers.typeDaily") },
  { value: "Weekly", label: t("templates.triggers.typeWeekly") },
  { value: "Interval", label: t("templates.triggers.typeIntervalLong") },
];

const DAY_OPTS = [
  { value: "Monday", label: t("templates.triggers.weekdays.Monday") },
  { value: "Tuesday", label: t("templates.triggers.weekdays.Tuesday") },
  { value: "Wednesday", label: t("templates.triggers.weekdays.Wednesday") },
  { value: "Thursday", label: t("templates.triggers.weekdays.Thursday") },
  { value: "Friday", label: t("templates.triggers.weekdays.Friday") },
  { value: "Saturday", label: t("templates.triggers.weekdays.Saturday") },
  { value: "Sunday", label: t("templates.triggers.weekdays.Sunday") },
];

const UNIT_OPTS = [
  { value: "Minutes", label: t("templates.triggers.unitMinutes") },
  { value: "Hours", label: t("templates.triggers.unitHours") },
];

function typeToString(v: unknown): string {
  if (typeof v === "number") return ["Once", "Daily", "Weekly", "Interval"][v] ?? "Once";
  const s = String(v ?? "Once");
  if (/^\d+$/.test(s)) return ["Once", "Daily", "Weekly", "Interval"][Number(s)] ?? "Once";
  return s;
}

function timeSpanToHhMm(v: unknown): string {
  if (!v) return "09:00";
  const s = String(v);
  const m = /^(\d{1,2}):(\d{2})/.exec(s);
  if (m) return `${m[1].padStart(2, "0")}:${m[2]}`;
  return "09:00";
}

function datePart(v: unknown): string {
  if (!v) {
    const d = new Date();
    d.setDate(d.getDate() + 1);
    return toIsoFromDate(d);
  }
  const s = String(v);
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(s);
  if (m) return `${m[1]}-${m[2]}-${m[3]}`;
  const d = new Date(s);
  if (Number.isNaN(d.getTime())) {
    const n = new Date();
    n.setDate(n.getDate() + 1);
    return toIsoFromDate(n);
  }
  return toIsoFromDate(d);
}

export default function TimeTriggerEditorDialog({ initial, onCancel, onSave }: Props) {
  const t = useT();
  const initType = typeToString(initial?.type ?? initial?.trigger_type);
  const [type, setType] = useState(initType);
  const [enabled, setEnabled] = useState(initial?.is_enabled !== false);
  const [onceDate, setOnceDate] = useState(datePart(initial?.scheduled_date_time));
  const [onceTime, setOnceTime] = useState(() => {
    if (initial?.scheduled_date_time) {
      const d = new Date(String(initial.scheduled_date_time));
      if (!Number.isNaN(d.getTime())) {
        return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
      }
    }
    return "09:00";
  });
  const [dailyTime, setDailyTime] = useState(timeSpanToHhMm(initial?.daily_time));
  const [weeklyDay, setWeeklyDay] = useState(String(initial?.weekly_day || "Monday"));
  const [weeklyTime, setWeeklyTime] = useState(timeSpanToHhMm(initial?.weekly_time));
  const [intervalValue, setIntervalValue] = useState(() => {
    const mins = Number(initial?.interval_minutes ?? 60);
    if (mins >= 60 && mins % 60 === 0) return String(mins / 60);
    return String(mins || 60);
  });
  const [intervalUnit, setIntervalUnit] = useState(() => {
    const mins = Number(initial?.interval_minutes ?? 60);
    return mins >= 60 && mins % 60 === 0 ? "Hours" : "Minutes";
  });
  const [error, setError] = useState<string | null>(null);

  const title = useMemo(() => t("templates.triggers.editorTitle"), []);

  function parseTime(text: string): boolean {
    return /^\d{1,2}:\d{2}$/.test(text.trim());
  }

  function validate(): boolean {
    if (type === "Once") {
      if (!onceDate) {
        setError(t("templates.triggers.errPickDate"));
        return false;
      }
      if (!parseTime(onceTime)) {
        setError(t("templates.triggers.errTimeFormat"));
        return false;
      }
      if (onceDate < toIsoFromDate(new Date())) {
        setError(t("templates.triggers.errPastDate"));
        return false;
      }
    } else if (type === "Daily" && !parseTime(dailyTime)) {
      setError(t("templates.triggers.errTimeFormat"));
      return false;
    } else if (type === "Weekly") {
      if (!weeklyDay) {
        setError(t("templates.triggers.errPickWeekday"));
        return false;
      }
      if (!parseTime(weeklyTime)) {
        setError(t("templates.triggers.errTimeFormat"));
        return false;
      }
    } else if (type === "Interval") {
      const n = Number(intervalValue);
      if (!Number.isFinite(n) || n <= 0) {
        setError(t("templates.triggers.errInterval"));
        return false;
      }
    }
    setError(null);
    return true;
  }

  function build(): TimeTriggerDto {
    const base: TimeTriggerDto = {
      id: initial?.id ?? crypto.randomUUID(),
      template_id: initial?.template_id ?? "",
      type,
      trigger_type: type,
      is_enabled: enabled,
      scheduled_date_time: null,
      daily_time: null,
      weekly_day: null,
      weekly_time: null,
      interval_minutes: null,
      last_executed_at: initial?.last_executed_at ?? null,
      created_at: initial?.created_at ?? new Date().toISOString(),
      next_execution_at: initial?.next_execution_at ?? null,
      last_execution_status: initial?.last_execution_status ?? null,
    };
    if (type === "Once") {
      base.scheduled_date_time = `${onceDate}T${onceTime.padStart(5, "0")}:00`;
    } else if (type === "Daily") {
      base.daily_time = `${dailyTime}:00`;
    } else if (type === "Weekly") {
      base.weekly_day = weeklyDay;
      base.weekly_time = `${weeklyTime}:00`;
    } else if (type === "Interval") {
      const n = Number(intervalValue);
      base.interval_minutes = intervalUnit === "Hours" ? n * 60 : n;
    }
    return base;
  }

  return (
    <AppCustomWindow
      title={title}
      width={560}
      zIndex={1400}
      onClose={onCancel}
      bodyClassName="tpl-trigger-edit-body combo-clip-ok"
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={onCancel}>
            {t("common.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => {
              if (!validate()) return;
              onSave(build());
            }}
          >
            {t("common.save")}
          </button>
        </>
      }
    >
      <div className="tpl-trigger-edit-title-row">
        <h3 className="tpl-trigger-edit-title">{t("templates.triggers.editorHeading")}</h3>
        <label
          className="toggle-switch"
          title={enabled ? t("templates.triggers.enabledTitle") : t("templates.triggers.disabledTitle")}
          aria-label={enabled ? t("templates.triggers.enabledTitle") : t("templates.triggers.disabledTitle")}
        >
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
          />
          <span className="toggle-track" />
        </label>
      </div>
      {error ? <p className="error">{error}</p> : null}

      <div className="tpl-trigger-edit-stack field-flush">
        <div className="field">
          <label>{t("templates.triggers.typeLabel")}</label>
          <AppComboBox options={TYPE_OPTS} value={type} onChange={setType} />
        </div>

        {type === "Once" ? (
          <>
            <DateRangePicker
              mode="single"
              singleLabel={t("common.date")}
              valueFrom={onceDate}
              onChange={({ from }) => setOnceDate(from)}
            />
            <div className="field">
              <label htmlFor="tr-once-time">{t("templates.triggers.timeHm")}</label>
              <input
                id="tr-once-time"
                value={onceTime}
                onChange={(e) => setOnceTime(e.target.value)}
                placeholder="09:00"
              />
            </div>
          </>
        ) : null}

        {type === "Daily" ? (
          <div className="field">
            <label htmlFor="tr-daily">{t("templates.triggers.timeHm")}</label>
            <input
              id="tr-daily"
              value={dailyTime}
              onChange={(e) => setDailyTime(e.target.value)}
              placeholder="09:00"
            />
          </div>
        ) : null}

        {type === "Weekly" ? (
          <>
            <div className="field">
              <label>{t("templates.triggers.weekday")}</label>
              <AppComboBox options={DAY_OPTS} value={weeklyDay} onChange={setWeeklyDay} />
            </div>
            <div className="field">
              <label htmlFor="tr-weekly-time">{t("templates.triggers.timeHm")}</label>
              <input
                id="tr-weekly-time"
                value={weeklyTime}
                onChange={(e) => setWeeklyTime(e.target.value)}
                placeholder="09:00"
              />
            </div>
          </>
        ) : null}

        {type === "Interval" ? (
          <div className="tpl-interval-row field-flush">
            <div className="field">
              <label htmlFor="tr-int">{t("templates.triggers.interval")}</label>
              <input
                id="tr-int"
                value={intervalValue}
                onChange={(e) => setIntervalValue(e.target.value)}
              />
            </div>
            <div className="field">
              <label>{t("templates.triggers.unit")}</label>
              <AppComboBox options={UNIT_OPTS} value={intervalUnit} onChange={setIntervalUnit} />
            </div>
          </div>
        ) : null}
      </div>
    </AppCustomWindow>
  );
}
