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

import { defaultExcelFileName, hostApi, hostDialogs, openUrl } from "../../bridge/hostApi";
import {
  invalidateCalculatorLookupCaches,
  markLookupWarmStarted,
  resetLookupWarmFlag,
} from "./calcLookupCache";
import type { CalculatorSnapshot } from "./calculatorTypes";
import { t } from "../../i18n";

function afterTablesMutated(): void {
  invalidateCalculatorLookupCaches();
  markLookupWarmStarted();
  void hostApi.wbCalculator.warmLookupCache().catch(() => resetLookupWarmFlag());
}

type Dialogs = {
  alert: (msg: string, title?: string) => Promise<void>;
  confirm: (msg: string, title?: string) => Promise<boolean>;
  prompt: (
    msg: string,
    title?: string,
    def?: string,
    opts?: { browseLabel?: string; onBrowse?: () => Promise<string | null> },
  ) => Promise<string | null>;
};

export async function saveEditLinkAction(
  key: string,
  url: string,
): Promise<CalculatorSnapshot> {
  return (await hostApi.wbCalculator.editTableLink(key, url)) as CalculatorSnapshot;
}

export async function openTableAction(key: string): Promise<void> {
  const res = await hostApi.wbCalculator.openTable(key);
  if (res.url) await openUrl(res.url);
}

export async function openExternalUrlAction(url: string): Promise<void> {
  await openUrl(url);
}

export async function runCreateOrUpdateAllAction(
  marketplace: "wb" | "ozon" | undefined,
  dialogs: Pick<Dialogs, "alert">,
): Promise<CalculatorSnapshot | null> {
  const res = await hostApi.wbCalculator.createOrUpdateAll(marketplace);
  if (res.ok) afterTablesMutated();
  await dialogs.alert(res.message || (res.ok ? t("common.done") : t("common.error")), res.ok ? t("nav.calculatorTables") : t("common.error"));
  return res.snapshot ? (res.snapshot as CalculatorSnapshot) : null;
}

export async function refreshOneAction(
  key: string,
  dialogs: Pick<Dialogs, "alert">,
): Promise<CalculatorSnapshot | null> {
  const res = await hostApi.wbCalculator.refreshTable(key);
  if (res.ok !== false) afterTablesMutated();
  if (res.message) await dialogs.alert(res.message, t("calculator.tables.alertUpdateTitle"));
  return res.snapshot ? (res.snapshot as CalculatorSnapshot) : null;
}

export async function clearTableLinksAction(
  dialogs: Pick<Dialogs, "confirm">,
): Promise<CalculatorSnapshot | null> {
  const ok = await dialogs.confirm(
    t("calculator.tables.confirmClearLinks"),
    t("calculator.tables.clearLinks"),
  );
  if (!ok) return null;
  const snap = (await hostApi.wbCalculator.clearTableLinks()) as CalculatorSnapshot;
  afterTablesMutated();
  return snap;
}

export async function exportExcelAction(
  dialogs: Pick<Dialogs, "alert">,
): Promise<void> {
  const path = await hostDialogs.saveExcelFile(
    t("calculator.view.exportExcelTitle"),
    defaultExcelFileName(),
  );
  if (!path?.trim()) return;
  const res = await hostApi.wbCalculator.exportExcel(path.trim());
  const warn =
    res.warnings && res.warnings.length > 0 ? `\n\n${res.warnings.join("\n")}` : "";
  await dialogs.alert(
    res.ok
      ? t("calculator.tables.exportSavedShort", {
          path: res.path ?? path,
          wbRows: res.wbRows ?? 0,
          warnings: warn,
        })
      : t("calculator.view.exportFail", { warnings: warn }),
    res.ok ? "Excel" : t("common.error"),
  );
}
