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

import { t } from "../../i18n";
import { BY_KEY } from "./scriptVisibilityByKey";
import { DEFAULT, type ScriptVisibility } from "./scriptVisibilityDefaults";

export type { ScriptVisibility } from "./scriptVisibilityDefaults";


export function visibilityForScript(scriptKey: string): ScriptVisibility {
  const key = scriptKey.trim().toLowerCase();
  return BY_KEY[key] ?? DEFAULT;
}

/** Visibility с учётом тумблера ТН ВЭД (вторая таблица + subject ranges). */
export function visibilityForTemplate(
  scriptKey: string,
  useCustomSubjectIds?: boolean,
): ScriptVisibility {
  const base = visibilityForScript(scriptKey);
  const key = scriptKey.trim().toLowerCase();
  if ((key !== "wb_tnved_codes" && key !== "wb_object_charcs") || !useCustomSubjectIds) {
    return base;
  }
  return {
    ...base,
    showSecondSpreadsheet: true,
    showSubjectIdSourceRange: true,
    showSubjectNameSourceRange: true,
    secondHint:
      key === "wb_object_charcs"
        ? t("templates.scriptHints.objectCharcs")
        : t("templates.scriptHints.tnvedCustom"),
  };
}

export function hasUnifiedSources(v: ScriptVisibility): boolean {
  return (
    v.showSecondSpreadsheet ||
    v.showCabinetsSourceRange ||
    v.showNmIdSourceRange ||
    v.showStocksSkusSourceRange ||
    v.showIdAdvCampaignSourceRange ||
    v.showOfferIdSourceRange ||
    v.showProductIdSourceRange ||
    v.showSkuSourceRange ||
    v.showOzonIdRangeByMode ||
    v.showContentMediaLinksSourceRange ||
    v.showSubjectIdSourceRange ||
    v.showSubjectNameSourceRange ||
    v.showCardsUpdateRanges ||
    v.showPricesUploadRanges ||
    v.showPriceSetRanges ||
    v.showPricesHistoryUploadIdRange ||
    v.showPaidStorageStatusRanges ||
    v.showPaidStorageDownloadTaskIdRange
  );
}
