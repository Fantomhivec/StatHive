import type { TemplateDto } from "../../bridge/hostApi";
import { formatDisplayDate } from "../../components/dateIso";
import { calculateDateRangeFromTemplate, DATE_TEMPLATES } from "../../components/templates/dateTemplates";
import { visibilityForScript } from "../../components/templates/scriptVisibility";
import { t as tr } from "../../i18n";

/** ISO `yyyy-MM-dd` → `dd.MM.yyyy` (как sheet_dates). */
export function formatSheetDate(iso?: string): string {
  const s = (iso ?? "").trim();
  if (!s) return "";
  return formatDisplayDate(s) || s;
}

/** RFC3339 / ISO → `dd.MM.yyyy  HH:mm:ss` (локальное время). */
export function formatRunAtDisplay(at?: string): string {
  const s = (at ?? "").trim();
  if (!s) return "";
  const d = new Date(s);
  if (Number.isNaN(d.getTime())) {
    const m = /^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2}):(\d{2})/.exec(s);
    if (!m) return s;
    return `${m[3]}.${m[2]}.${m[1]}  ${m[4]}:${m[5]}:${m[6]}`;
  }
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}.${pad(d.getMonth() + 1)}.${d.getFullYear()}  ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

/** `4 кабинета` / `5 кабинетов` / `1 кабинет`. */
export function formatCabinetsLabel(cabinets: string[] | undefined): string {
  const list = (cabinets ?? []).map((c) => c.trim()).filter(Boolean);
  const n = list.length;
  if (n === 0) return "";
  if (n <= 3) return list.join(", ");
  const mod10 = n % 10;
  const mod100 = n % 100;
  let key = "templates.plural.cabinetMany";
  if (mod10 === 1 && mod100 !== 11) key = "templates.plural.cabinetOne";
  else if (mod10 >= 2 && mod10 <= 4 && (mod100 < 10 || mod100 >= 20)) key = "templates.plural.cabinetFew";
  return tr(key, { count: n });
}

/** Период справа от имени: `ТП: (dd.MM.yyyy)-(dd.MM.yyyy)`. */
export function formatPeriodLabel(tpl: TemplateDto): string | null {
  const vis = visibilityForScript(tpl.script_key);
  if (!vis.requiresDateRange && !vis.requiresSingleDate) return null;

  let from = (tpl.date_from ?? "").trim();
  let to = (tpl.date_to ?? "").trim();
  const tag = (tpl.date_template_type ?? "").trim();
  if (tag && DATE_TEMPLATES.some((d) => d.tag === tag)) {
    const range = calculateDateRangeFromTemplate(tag);
    if (vis.requiresSingleDate) {
      from = range.to;
      to = "";
    } else {
      from = range.from;
      to = range.to;
    }
  }
  if (!from && !to) return null;
  if (vis.requiresSingleDate || (!to && from)) {
    return tr("templates.card.periodSingle", { from: formatSheetDate(from) });
  }
  return tr("templates.card.periodRange", {
    from: formatSheetDate(from),
    to: formatSheetDate(to),
  });
}

/** Только название документа Google Sheets (не ID). */
function tableLabel(tpl: TemplateDto): string {
  return (tpl.spreadsheet_title ?? "").trim();
}

/**
 * Под именем: таблица · лист · кабинеты · время; статус отдельно (цвет).
 */
export function formatTemplateMeta(tpl: TemplateDto, running: boolean): {
  lead: string;
  status: string;
  statusClass: "" | "is-ok" | "is-err";
} {
  const parts: string[] = [];
  const table = tableLabel(tpl);
  if (table) parts.push(table);
  const sheet = (tpl.sheet_name ?? "").trim();
  if (sheet) parts.push(tr("templates.card.sheetPart", { name: sheet }));
  const cabs = formatCabinetsLabel(tpl.cabinets);
  if (cabs) parts.push(cabs);

  if (running) {
    return { lead: parts.join(" · "), status: tr("templates.card.running"), statusClass: "" };
  }

  const when = formatRunAtDisplay(tpl.last_run_at);
  if (when) parts.push(when);

  const status = (tpl.last_run_status ?? "").trim();
  let statusClass: "" | "is-ok" | "is-err" = "";
  if (status) {
    statusClass =
      status.startsWith("error") || status === "cancelled" ? "is-err" : "is-ok";
  } else if (parts.length === 0) {
    return { lead: "", status: tr("templates.card.ready"), statusClass: "" };
  }

  return { lead: parts.join(" · "), status, statusClass };
}
