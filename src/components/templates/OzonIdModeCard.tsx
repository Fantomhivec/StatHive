import type { TemplateDto } from "../../bridge/hostApi";
import { t, useT } from "../../i18n";

type Props = {
  tpl: TemplateDto;
  disabled?: boolean;
  onPatch: (partial: Partial<TemplateDto>) => void;
};

const MODES: Array<{ value: string; label: string }> = [
  { value: "sku", label: t("templates.ozonIdMode.sku") },
  { value: "offer_id", label: t("templates.ozonIdMode.offerId") },
  { value: "product_id", label: t("templates.ozonIdMode.productId") },
];

/** Карточка режима ID Ozon (один режим). */
export default function OzonIdModeCard({ tpl, disabled, onPatch }: Props) {
  const t = useT();
  const mode = (tpl.id_mode ?? "").trim() || "sku";

  return (
    <div className="tpl-dialog-card">
      <h3 className="tpl-dialog-card-header">{t("templates.ozonIdMode.title")}</h3>
      <p className="tpl-info">{t("templates.ozonIdMode.hint")}</p>
      {MODES.map((m) => (
        <label key={m.value} className="tpl-check tpl-check--stack">
          <input
            type="radio"
            name="ozon-id-mode"
            disabled={disabled}
            checked={mode === m.value}
            onChange={() => onPatch({ id_mode: m.value })}
          />
          <span>{m.label}</span>
        </label>
      ))}
    </div>
  );
}
