import { memo } from "react";
import AppComboBox from "../../components/AppComboBox";
import { TABLE_MODE_OPTS } from "./calculatorTypes";
import { t } from "../../i18n";

export type CalculatorTableRowProps = {
  tableKey: string;
  label: string;
  marketplace?: string;
  mode: string;
  link: string;
  openHref: string;
  updated: string;
  rowBusy: boolean;
  onSetMode: (key: string, mode: string) => void;
  onOpen: (key: string, link: string, openHref: string) => void;
  onEditLink: (key: string, label: string, url: string) => void;
  onRefresh: (key: string) => void;
  /** false — участник без ключей: только открыть таблицу. */
  showApiActions?: boolean;
  /** false — без OAuth: просмотр, без смены режима / URL / refresh. */
  canMutate?: boolean;
};

function CalculatorTableRowInner({
  tableKey,
  label,
  mode,
  link,
  openHref,
  updated,
  rowBusy,
  onSetMode,
  onOpen,
  onEditLink,
  onRefresh,
  showApiActions = true,
  canMutate = true,
}: CalculatorTableRowProps) {
  const apiOk = showApiActions && canMutate;
  const isSummary = tableKey === "WbSummary" || tableKey === "OzSummary";
  return (
    <div className="wb-table-row wb-table-row--modes combo-clip-ok">
      <div
        className="wb-table-label"
        title={
          isSummary ? t("calculator.tables.row.summaryRowTitle", { label }) : label
        }
      >
        <span className="wb-table-label-text">
          {label}
          {isSummary ? <span className="muted">{t("calculator.tables.row.summarySuffix")}</span> : null}
        </span>
        {updated ? (
          <span className="muted wb-table-updated">
            {t("calculator.tables.row.updated", { time: updated })}
          </span>
        ) : null}
      </div>
      <div className="wb-table-mode">
        {isSummary ? (
          <span className="muted wb-table-mode-static">{t("calculator.tables.row.assembly")}</span>
        ) : (
          <AppComboBox
            className="app-combo-sm"
            value={mode}
            options={TABLE_MODE_OPTS}
            disabled={rowBusy || !canMutate}
            onChange={(v) => onSetMode(tableKey, v)}
          />
        )}
      </div>
      <div className="wb-table-field">
        <input className="input-compact" readOnly value={link} placeholder={t("calculator.tables.row.linkPlaceholder")} />
      </div>
      <div className="wb-table-actions">
        <button
          type="button"
          className="btn btn-outline btn-sm"
          disabled={!openHref}
          title={!openHref ? t("calculator.tables.row.noLinkTitle") : t("calculator.tables.row.openTableTitle")}
          onClick={() => onOpen(tableKey, link, openHref)}
        >
          {t("common.open")}
        </button>
        {apiOk && (
          <button
            type="button"
            className="btn btn-outline btn-sm"
            disabled={rowBusy || isSummary}
            onClick={() => onEditLink(tableKey, label, link)}
            title={isSummary ? t("calculator.tables.row.summaryAutoTitle") : t("calculator.tables.row.editLinkTitle")}
          >
            {t("common.edit")}
          </button>
        )}
        {apiOk && (
          <button
            type="button"
            className="btn btn-primary btn-sm btn-minw"
            disabled={rowBusy || mode === "manual"}
            title={
              mode === "manual"
                ? t("calculator.tables.row.manualDisabled")
                : isSummary
                  ? t("calculator.tables.row.rebuildSummary")
                  : t("calculator.tables.row.refreshTable")
            }
            onClick={() => onRefresh(tableKey)}
          >
            {t("common.refresh")}
          </button>
        )}
      </div>
    </div>
  );
}

export const CalculatorTableRow = memo(CalculatorTableRowInner);
