import { toIsoFromDate } from "../dateIso";
import { t } from "../../i18n";

/** Пресеты дат для шаблонов. */

export const DATE_TEMPLATES: Array<{ tag: string; label: string }> = [
  { tag: "1week", label: t("dateTemplates.1week") },
  { tag: "2weeks", label: t("dateTemplates.2weeks") },
  { tag: "3weeks", label: t("dateTemplates.3weeks") },
  { tag: "4week", label: t("dateTemplates.4weeks") },
  { tag: "2months", label: t("dateTemplates.2months") },
  { tag: "3months", label: t("dateTemplates.3months") },
  { tag: "4months", label: t("dateTemplates.4months") },
  { tag: "5months", label: t("dateTemplates.5months") },
  { tag: "1year", label: t("dateTemplates.1year") },
];

export function calculateDateRangeFromTemplate(
  templateType: string,
  includeCurrentDay = false,
  shiftPlusOneDay = false,
): { from: string; to: string } {
  const today = startOfLocalDay(new Date());
  const yesterday = addDays(today, -1);

  let from: Date;
  let to: Date;
  switch (templateType) {
    case "1week":
      from = addDays(yesterday, -6);
      to = yesterday;
      break;
    case "2weeks":
      from = addDays(yesterday, -13);
      to = yesterday;
      break;
    case "3weeks":
      from = addDays(yesterday, -20);
      to = yesterday;
      break;
    case "4week":
      from = addDays(yesterday, -27);
      to = yesterday;
      break;
    case "1month":
      from = addDays(yesterday, -29);
      to = yesterday;
      break;
    case "2months":
      from = addDays(yesterday, -59);
      to = yesterday;
      break;
    case "3months":
      from = addDays(yesterday, -89);
      to = yesterday;
      break;
    case "4months":
      from = addDays(yesterday, -119);
      to = yesterday;
      break;
    case "5months":
      from = addDays(yesterday, -149);
      to = yesterday;
      break;
    case "1year":
      from = addDays(yesterday, -364);
      to = yesterday;
      break;
    default:
      from = addDays(yesterday, -6);
      to = yesterday;
  }

  if (includeCurrentDay) {
    from = addDays(from, 1);
    to = addDays(to, 1);
  } else if (shiftPlusOneDay) {
    to = addDays(to, 1);
  }

  return { from: toIsoFromDate(from), to: toIsoFromDate(to) };
}

function startOfLocalDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

function addDays(d: Date, n: number): Date {
  const x = new Date(d);
  x.setDate(x.getDate() + n);
  return x;
}
