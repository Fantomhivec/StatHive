import type { TemplateDto } from "../../bridge/hostApi";
import DateRangePicker from "../DateRangePicker";
import { useT } from "../../i18n";
import {
  calculateDateRangeFromTemplate,
  DATE_TEMPLATES,
} from "./dateTemplates";

type Props = {
  tpl: TemplateDto;
  /** Период from–to; иначе только date_from (tariffs_box). */
  period: boolean;
  disabled?: boolean;
  onPatch: (partial: Partial<TemplateDto>) => void;
};

export default function TemplateDateRangeCard({
  tpl,
  period,
  disabled,
  onPatch,
}: Props) {
  const t = useT();
  const active = (tpl.date_template_type ?? "").trim();

  function applyPreset(tag: string) {
    const range = calculateDateRangeFromTemplate(tag);
    if (period) {
      onPatch({
        date_template_type: tag,
        date_from: range.from,
        date_to: range.to,
      });
    } else {
      onPatch({
        date_template_type: tag,
        date_from: range.to,
        date_to: "",
      });
    }
  }

  return (
    <div className="tpl-dialog-card">
      <h3 className="tpl-dialog-card-header">{period ? t("common.period") : t("common.date")}</h3>
      <div className="tpl-date-presets">
        {DATE_TEMPLATES.map((d) => (
          <button
            key={d.tag}
            type="button"
            className={`btn btn-outline btn-sm${active === d.tag ? " btn-active-ok" : ""}`}
            disabled={disabled}
            onClick={() => applyPreset(d.tag)}
          >
            {d.label}
          </button>
        ))}
      </div>
      <DateRangePicker
        mode={period ? "range" : "single"}
        valueFrom={tpl.date_from}
        valueTo={tpl.date_to}
        disabled={disabled}
        singleLabel={t("common.date")}
        fromLabel={t("common.dateFrom")}
        toLabel={t("common.dateTo")}
        onChange={({ from, to }) =>
          onPatch({
            date_from: from,
            date_to: period ? to : "",
            date_template_type: "",
          })
        }
      />
    </div>
  );
}
