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

import { t } from "../../../i18n";
import type { ColumnBinding } from "../calculatorTypes";

type Metric = { key: string; label: string; marketplace: string; tableKey?: string };

type Props = {
  bindings: ColumnBinding[];
  metrics: Metric[];
  articlePlaceholder: string;
  disabled: boolean;
  onOpenUrl: (url: string) => void;
  onUpdateBinding: (metricKey: string, partial: Partial<ColumnBinding>) => void;
  onSaveBindings: () => void;
};

export function CalculatorTablesManual({
  bindings,
  metrics,
  articlePlaceholder,
  disabled,
  onOpenUrl,
  onUpdateBinding,
  onSaveBindings,
}: Props) {
  const byKey = new Map(bindings.map((b) => [b.metricKey, b]));
  return (
    <>
      <div className="plate-head--accent wb-manual-head">
        <span>{t("calculator.tables.manualGrid.metric")}</span>
        <span />
        <span>{t("calculator.tables.manualGrid.table")}</span>
        <span>{t("common.sheet")}</span>
        <span>{t("calculator.tables.manualGrid.column")}</span>
        <span>{t("calculator.card.article")}</span>
      </div>
      <div className="wb-tables">
        {metrics.map((m) => {
          const b =
            byKey.get(m.key) ??
            ({
              metricKey: m.key,
              spreadsheetUrl: "",
              sheetName: "",
              columnHeader: "",
              articleColumnHeader: articlePlaceholder,
            } as ColumnBinding);
          const openHref = b.spreadsheetUrl?.trim() || "";
          return (
            <div key={`${m.marketplace}:${m.key}`} className="wb-manual-row">
              <div className="wb-table-label" title={m.tableKey}>
                {m.label}
              </div>
              <button
                type="button"
                className="btn btn-outline btn-sm btn-pad-x-tight"
                disabled={disabled || !openHref}
                title={!openHref ? t("calculator.tables.row.noLinkTitle") : t("users.openSheet")}
                onClick={() => onOpenUrl(openHref)}
              >
                {t("common.open")}
              </button>
              <input
                className="input-compact wb-manual-url"
                placeholder={t("calculator.tables.manualGrid.urlPlaceholder")}
                value={b.spreadsheetUrl}
                disabled={disabled}
                onChange={(e) => onUpdateBinding(m.key, { spreadsheetUrl: e.target.value })}
                onBlur={onSaveBindings}
              />
              <input
                className="input-compact wb-manual-sheet"
                placeholder={t("common.sheet")}
                value={b.sheetName}
                disabled={disabled}
                onChange={(e) => onUpdateBinding(m.key, { sheetName: e.target.value })}
                onBlur={onSaveBindings}
              />
              <input
                className="input-compact wb-manual-col"
                placeholder={t("calculator.tables.manualGrid.columnPlaceholder")}
                value={b.columnHeader}
                disabled={disabled}
                onChange={(e) => onUpdateBinding(m.key, { columnHeader: e.target.value })}
                onBlur={onSaveBindings}
              />
              <input
                className="input-compact wb-manual-article"
                placeholder={`${articlePlaceholder} / A2:A`}
                value={b.articleColumnHeader}
                disabled={disabled}
                onChange={(e) => onUpdateBinding(m.key, { articleColumnHeader: e.target.value })}
                onBlur={onSaveBindings}
              />
            </div>
          );
        })}
      </div>
    </>
  );
}
