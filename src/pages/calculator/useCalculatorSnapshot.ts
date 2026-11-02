import { useCallback, useEffect, useRef, useState } from "react";
import { hostApi, onEvent } from "../../bridge/hostApi";
import {
  invalidateCalculatorLookupCaches,
  isLookupWarmStarted,
  markLookupWarmStarted,
  resetLookupWarmFlag,
} from "./calcLookupCache";
import {
  DEFAULT_OZ_KGT,
  DEFAULT_WB_KGT,
  type CalculatorSnapshot,
} from "./calculatorTypes";

export type CalcSnapshotView = "view" | "settings" | "tables" | "kgt" | "full";

/** Кэш по view — не смешивать slice между вкладками. */
const cacheByView: Partial<Record<CalcSnapshotView, CalculatorSnapshot>> = {};

function setCached(view: CalcSnapshotView, next: CalculatorSnapshot | null) {
  if (next) cacheByView[view] = next;
  else delete cacheByView[view];
}

function clearAllSnapCaches() {
  for (const k of Object.keys(cacheByView) as CalcSnapshotView[]) {
    delete cacheByView[k];
  }
}

/** Дополнить отсутствующие поля после view-slice (мутации могут вернуть full). */
function coerceSnap(raw: Record<string, unknown>): CalculatorSnapshot {
  return {
    bundleId: String(raw.bundleId ?? ""),
    calculatorName: String(raw.calculatorName ?? ""),
    marketplace: String(raw.marketplace ?? "wb"),
    tablesMarketplace: raw.tablesMarketplace as string | undefined,
    triggersEnabled: Boolean(raw.triggersEnabled),
    selectedCabinets: Array.isArray(raw.selectedCabinets)
      ? (raw.selectedCabinets as string[])
      : [],
    availableCabinets: Array.isArray(raw.availableCabinets)
      ? (raw.availableCabinets as string[])
      : [],
    missingKeyCabinets: Array.isArray(raw.missingKeyCabinets)
      ? (raw.missingKeyCabinets as string[])
      : undefined,
    usesLocalKeys: raw.usesLocalKeys as boolean | undefined,
    cabinetMarketplaces: (raw.cabinetMarketplaces as Record<string, string>) ?? {},
    cabinetChips: raw.cabinetChips as CalculatorSnapshot["cabinetChips"],
    tableLinks: (raw.tableLinks as Record<string, string>) ?? {},
    tableModes: (raw.tableModes as Record<string, string>) ?? {},
    lastUpdated: (raw.lastUpdated as Record<string, string>) ?? {},
    taxPercent: Number(raw.taxPercent ?? 0),
    acquiringPercent: Number(raw.acquiringPercent ?? 0),
    advPercent: Number(raw.advPercent ?? 0),
    extraExpensesPercent: Number(raw.extraExpensesPercent ?? 0),
    ozonTaxPercent: Number(raw.ozonTaxPercent ?? 0),
    ozonAcquiringPercent: Number(raw.ozonAcquiringPercent ?? 0),
    ozonAdvPercent: Number(raw.ozonAdvPercent ?? 0),
    ozonExtraExpensesPercent: Number(raw.ozonExtraExpensesPercent ?? 0),
    profitFormula: String(raw.profitFormula ?? ""),
    ozonProfitFormula: String(raw.ozonProfitFormula ?? ""),
    wbKgt: (raw.wbKgt as CalculatorSnapshot["wbKgt"]) ?? DEFAULT_WB_KGT,
    ozKgt: (raw.ozKgt as CalculatorSnapshot["ozKgt"]) ?? DEFAULT_OZ_KGT,
    columnBindings: Array.isArray(raw.columnBindings)
      ? (raw.columnBindings as CalculatorSnapshot["columnBindings"])
      : [],
    metrics: Array.isArray(raw.metrics)
      ? (raw.metrics as CalculatorSnapshot["metrics"])
      : [],
    triggers: Array.isArray(raw.triggers)
      ? (raw.triggers as CalculatorSnapshot["triggers"])
      : [],
    tableKeys: Array.isArray(raw.tableKeys)
      ? (raw.tableKeys as CalculatorSnapshot["tableKeys"])
      : [],
    templateIdsByTableKey: (raw.templateIdsByTableKey as Record<string, string>) ?? {},
    createButtonLabel: String(raw.createButtonLabel ?? ""),
  };
}

function ensureLookupWarm() {
  if (isLookupWarmStarted()) return;
  markLookupWarmStarted();
  void hostApi.wbCalculator.warmLookupCache().catch(() => {
    resetLookupWarmFlag();
  });
}

export function useCalculatorSnapshot(view: CalcSnapshotView = "full") {
  const [snap, setSnapState] = useState<CalculatorSnapshot | null>(
    () => cacheByView[view] ?? null,
  );
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const snapRef = useRef<CalculatorSnapshot | null>(snap);
  snapRef.current = snap;
  const viewRef = useRef(view);
  viewRef.current = view;

  const setSnap = useCallback(
    (
      next:
        | CalculatorSnapshot
        | null
        | ((prev: CalculatorSnapshot | null) => CalculatorSnapshot | null),
    ) => {
      setSnapState((prev) => {
        const resolved = typeof next === "function" ? next(prev) : next;
        setCached(viewRef.current, resolved);
        return resolved;
      });
    },
    [],
  );

  const reload = useCallback(async () => {
    const v = viewRef.current;
    const raw = (await hostApi.wbCalculator.getSnapshot(v)) as Record<string, unknown>;
    const s = coerceSnap(raw);
    setCached(v, s);
    setSnapState(s);
  }, []);

  useEffect(() => {
    // Sheets lookup needed only on the card view — not Settings/Tables/KGT.
    if (view === "view") ensureLookupWarm();
    const cached = cacheByView[view];
    if (cached) {
      setSnapState(cached);
      return;
    }
    void reload().catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [reload, view]);

  useEffect(() => {
    return onEvent<{ name?: string }>("wbCalculator.teamConfigApplied", () => {
      invalidateCalculatorLookupCaches();
      clearAllSnapCaches();
      void reload().catch((e) => setError(e instanceof Error ? e.message : String(e)));
    });
  }, [reload]);

  /**
   * Только partial → saveSettings (бэкенд мержит поля).
   * Не слать весь snap: иначе параллельный/устаревший patch затирает
   * columnBindings / tableModes чужими значениями (часто ручные поля Ozon).
   */
  async function patch(partial: Record<string, unknown>) {
    if (!snapRef.current) return;
    setBusy(true);
    setError(null);
    try {
      const next = await hostApi.wbCalculator.saveSettings(partial);
      setSnap(coerceSnap(next as Record<string, unknown>));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return { snap, setSnap, error, setError, busy, setBusy, reload, patch };
}
