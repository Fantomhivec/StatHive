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
