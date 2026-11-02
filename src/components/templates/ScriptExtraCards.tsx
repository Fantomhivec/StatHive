import { useState } from "react";
import type { TemplateDto } from "../../bridge/hostApi";
import type { SubjectsListFiltersDto } from "../../bridge/hostApi.types";
import AppComboBox from "../AppComboBox";
import AppCustomWindow from "../AppCustomWindow";
import type { ScriptVisibility } from "./scriptVisibility";
import { useT } from "../../i18n";

const REPORT_PERIOD_OPTS = [
  { value: "weekly", label: "weekly" },
  { value: "daily", label: "daily" },
];

type Props = {
  visibility: ScriptVisibility;
  tpl: TemplateDto;
  disabled?: boolean;
  onPatch: (partial: Partial<TemplateDto>) => void;
  onOpenSubjectsFilters: () => void;
};

/** Карточки режимов / подсказок для скриптов 041–049. */
export default function ScriptExtraCards({
  visibility,
  tpl,
  disabled,
  onPatch,
  onOpenSubjectsFilters,
}: Props) {
  const t = useT();
  return (
    <>
      {visibility.showCardsMoveTrashHint ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.extraCards.moveToTrash")}</h3>
          <p className="tpl-info">{t("templates.extraCards.moveToTrashHint")}</p>
        </div>
      ) : null}

      {visibility.showCardsUpdateModes ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.extraCards.cardsUpdate")}</h3>
          {(
            [
              ["cards_update_dimensions_mode", t("templates.extraCards.modeDimensions")],
              ["cards_update_vendor_code_mode", t("templates.extraCards.modeVendorCode")],
              ["cards_update_vat_mode", t("templates.extraCards.modeVat")],
              ["cards_update_brand_mode", t("templates.extraCards.modeBrand")],
            ] as const
          ).map(([field, label]) => (
            <label key={field} className="toggle-switch tpl-toggle-row">
              <input
                type="checkbox"
                checked={!!tpl[field]}
                disabled={disabled}
                onChange={(e) => onPatch({ [field]: e.target.checked })}
              />
              <span className="toggle-track" />
              <span className="tpl-toggle-label">{label}</span>
            </label>
          ))}
          <p className="tpl-info">
            {t("templates.extraCards.cardsUpdateHint")}
          </p>
        </div>
      ) : null}

      {visibility.showTnvedCustomIds || visibility.showObjectCharcsCustomIds ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">
            {visibility.showObjectCharcsCustomIds
              ? t("templates.extraCards.objectCharcs")
              : t("templates.extraCards.tnved")}
          </h3>
          <label className="toggle-switch tpl-toggle-row">
            <input
              type="checkbox"
              checked={!!tpl.use_custom_subject_ids}
              disabled={disabled}
              onChange={(e) => onPatch({ use_custom_subject_ids: e.target.checked })}
            />
            <span className="toggle-track" />
            <span className="tpl-toggle-label">
              {visibility.showObjectCharcsCustomIds
                ? t("templates.extraCards.objectCharcsToggle")
                : t("templates.extraCards.tnvedCustomToggle")}
            </span>
          </label>
          <p className="tpl-info">
            {visibility.showObjectCharcsCustomIds
              ? t("templates.extraCards.objectCharcsHint")
              : t("templates.extraCards.tnvedCustomHint")}
          </p>
        </div>
      ) : null}

      {visibility.showPromotionsAllPromo ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.extraCards.promotions")}</h3>
          <label className="toggle-switch tpl-toggle-row">
            <input
              type="checkbox"
              checked={!!tpl.promotions_all_promo}
              disabled={disabled}
              onChange={(e) => onPatch({ promotions_all_promo: e.target.checked })}
            />
            <span className="toggle-track" />
            <span className="tpl-toggle-label">{t("templates.extraCards.promotionsAllToggle")}</span>
          </label>
          <p className="tpl-info">{t("templates.extraCards.promotionsAllHint")}</p>
        </div>
      ) : null}

      {visibility.showSubjectsFilters ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.extraCards.subjectsFilters")}</h3>
          <p className="tpl-info">
            {t("templates.extraCards.subjectsFiltersHint")}
          </p>
          <button
            type="button"
            className="btn btn-outline"
            disabled={disabled}
            onClick={onOpenSubjectsFilters}
          >
            {t("templates.extraCards.configureFilters")}
          </button>
        </div>
      ) : null}

      {visibility.showReportPeriod ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.extraCards.reportPeriod")}</h3>
          <div className="field">
            <label>{t("templates.extraCards.reportPeriodLabel")}</label>
            <AppComboBox
              disabled={disabled}
              value={(tpl.report_realization_v5_period ?? "weekly").toLowerCase()}
              options={REPORT_PERIOD_OPTS}
              onChange={(v) => onPatch({ report_realization_v5_period: v })}
            />
          </div>
        </div>
      ) : null}

      {visibility.showPricesUploadRanges ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.extraCards.priceRaise")}</h3>
          <div className="field">
            <label htmlFor="tpl-prices-raise">{t("templates.extraCards.priceRaiseLabel")}</label>
            <input
              id="tpl-prices-raise"
              className="input"
              disabled={disabled}
              value={tpl.prices_raise_percent ?? ""}
              placeholder="0"
              onChange={(e) => onPatch({ prices_raise_percent: e.target.value })}
            />
          </div>
          <p className="tpl-info">{t("templates.extraCards.pricesSetHint")}</p>
        </div>
      ) : null}

      {visibility.showSymbolsCheckCard ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.extraCards.symbolsCheck")}</h3>
          <div className="field">
            <label htmlFor="tpl-symbols-range">{t("templates.extraCards.symbolsCheckRange")}</label>
            <input
              id="tpl-symbols-range"
              className="input"
              disabled={disabled}
              value={tpl.symbols_check_range ?? ""}
              placeholder="A1:Z100"
              onChange={(e) => onPatch({ symbols_check_range: e.target.value })}
            />
          </div>
          <p className="tpl-info">{t("templates.extraCards.symbolsCheckHint")}</p>
        </div>
      ) : null}

      {visibility.showCellsCountCard ? (
        <div className="tpl-dialog-card">
          <h3 className="tpl-dialog-card-header">{t("templates.extraCards.cellsCount")}</h3>
          <label className="toggle-switch tpl-toggle-row">
            <input
              type="checkbox"
              checked={!!tpl.cells_count_count_filled}
              disabled={disabled}
              onChange={(e) => onPatch({ cells_count_count_filled: e.target.checked })}
            />
            <span className="toggle-track" />
            <span className="tpl-toggle-label">{t("templates.extraCards.cellsCountFilled")}</span>
          </label>
        </div>
      ) : null}
    </>
  );
}

type FiltersDialogProps = {
  filters: SubjectsListFiltersDto;
  onClose: () => void;
  onApply: (f: SubjectsListFiltersDto) => void;
};

export function SubjectsFiltersDialog({ filters, onClose, onApply }: FiltersDialogProps) {
  const t = useT();
  const [locale, setLocale] = useState(filters.locale ?? "ru");
  const [name, setName] = useState(filters.name ?? "");
  const [parentId, setParentId] = useState(
    filters.parent_id != null ? String(filters.parent_id) : "",
  );
  const [offset, setOffset] = useState(
    filters.offset != null ? String(filters.offset) : "0",
  );

  return (
    <AppCustomWindow
      title={t("templates.extraCards.categoryFiltersTitle")}
      onClose={onClose}
      width={420}
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => {
              const pidRaw = parentId.trim();
              const pid = pidRaw === "" ? null : Number(pidRaw);
              const off = offset.trim() === "" ? 0 : Number(offset);
              onApply({
                locale: locale.trim() || "ru",
                name: name.trim(),
                parent_id: pid != null && Number.isFinite(pid) ? pid : null,
                offset: Number.isFinite(off) ? off : 0,
                limit: filters.limit ?? 30,
              });
            }}
          >
            {t("common.apply")}
          </button>
        </>
      }
    >
      <div className="tpl-filters-stack">
        <div className="field">
          <label htmlFor="subj-locale">Locale</label>
          <input
            id="subj-locale"
            className="input"
            value={locale}
            onChange={(e) => setLocale(e.target.value)}
          />
        </div>
        <div className="field">
          <label htmlFor="subj-name">{t("templates.extraCards.searchName")}</label>
          <input
            id="subj-name"
            className="input"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <div className="field">
          <label htmlFor="subj-parent">parentID</label>
          <input
            id="subj-parent"
            className="input"
            value={parentId}
            onChange={(e) => setParentId(e.target.value)}
            placeholder={t("templates.extraCards.emptyMeansAll")}
          />
        </div>
        <div className="field">
          <label htmlFor="subj-offset">{t("templates.extraCards.startOffset")}</label>
          <input
            id="subj-offset"
            className="input"
            value={offset}
            onChange={(e) => setOffset(e.target.value)}
          />
        </div>
      </div>
    </AppCustomWindow>
  );
}
