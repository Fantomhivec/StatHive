// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { memo } from "react";
import AppComboBox from "../../../components/AppComboBox";
import { t } from "../../../i18n";
import { TABLE_MODE_OPTS } from "../calculatorTypes";

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
