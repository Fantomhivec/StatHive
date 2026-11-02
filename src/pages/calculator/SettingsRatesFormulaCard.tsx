import { lazy, Suspense, type ReactNode } from "react";
import { hostApi } from "../../bridge/hostApi";
import { useT } from "../../i18n";
import type { CalculatorSnapshot } from "./calculatorTypes";

const FormulaEditor = lazy(() =>
  import("./FormulaEditor").then((m) => ({ default: m.FormulaEditor })),
);
const FormulaTokens = lazy(() =>
  import("./FormulaEditor").then((m) => ({ default: m.FormulaTokens })),
);

type Props = {
  mp: "wb" | "ozon";
  snap: CalculatorSnapshot;
  busy: boolean;
  cabinets: ReactNode;
  setSnap: (next: CalculatorSnapshot) => void;
  patch: (partial: Record<string, unknown>) => void | Promise<void>;
  onAppendToken: (token: string) => void;
};

type RateField = {
  labelKey:
    | "calculator.settings.taxPercent"
    | "calculator.settings.acquiringPercent"
    | "calculator.settings.advPercent"
    | "calculator.settings.extraExpenses";
  get: (s: CalculatorSnapshot) => number;
  setKey: string;
};

const WB_RATES: RateField[] = [
  {
    labelKey: "calculator.settings.taxPercent",
    get: (s) => s.taxPercent,
    setKey: "taxPercent",
  },
  {
    labelKey: "calculator.settings.acquiringPercent",
    get: (s) => s.acquiringPercent,
    setKey: "acquiringPercent",
  },
  {
    labelKey: "calculator.settings.advPercent",
    get: (s) => s.advPercent,
    setKey: "advPercent",
  },
  {
    labelKey: "calculator.settings.extraExpenses",
    get: (s) => s.extraExpensesPercent,
    setKey: "extraExpensesPercent",
  },
];

const OZON_RATES: RateField[] = [
  {
    labelKey: "calculator.settings.taxPercent",
    get: (s) => s.ozonTaxPercent ?? 0,
    setKey: "ozonTaxPercent",
  },
  {
    labelKey: "calculator.settings.acquiringPercent",
    get: (s) => s.ozonAcquiringPercent ?? 0,
    setKey: "ozonAcquiringPercent",
  },
  {
    labelKey: "calculator.settings.advPercent",
    get: (s) => s.ozonAdvPercent ?? 0,
    setKey: "ozonAdvPercent",
  },
  {
    labelKey: "calculator.settings.extraExpenses",
    get: (s) => s.ozonExtraExpensesPercent ?? 0,
    setKey: "ozonExtraExpensesPercent",
  },
];

export function SettingsRatesFormulaCard({
  mp,
  snap,
  busy,
  cabinets,
  setSnap,
  patch,
  onAppendToken,
}: Props) {
  const t = useT();
  const rates = mp === "ozon" ? OZON_RATES : WB_RATES;
  const formulaKey = mp === "ozon" ? "ozonProfitFormula" : "profitFormula";
  const formulaValue = mp === "ozon" ? (snap.ozonProfitFormula ?? "") : snap.profitFormula;

  return (
    <section className="wb-card">
      <h3>
        {mp === "ozon"
          ? t("calculator.settings.ozonSection")
          : t("calculator.settings.wbSection")}
      </h3>
      {mp === "ozon" && (
        <p className="muted calc-hint">{t("calculator.settings.ozonSectionHint")}</p>
      )}
      {cabinets}
      <div className="wb-settings-grid">
        {rates.map((field) => (
          <label key={field.setKey} className="wb-settings-field">
            <span className="wb-settings-label">{t(field.labelKey)}</span>
            <input
              className="input-compact"
              type="number"
              value={field.get(snap)}
              onChange={(e) =>
                setSnap({ ...snap, [field.setKey]: Number(e.target.value) })
              }
              onBlur={() => void patch({ [field.setKey]: field.get(snap) })}
            />
          </label>
        ))}
      </div>
      <h4 className="calc-subhead">
        {mp === "ozon"
          ? t("calculator.settings.formulaOzonTitle")
          : t("calculator.settings.formulaWbTitle")}
      </h4>
      {mp === "wb" && (
        <p className="muted calc-hint">{t("calculator.settings.formulaHint")}</p>
      )}
      <Suspense fallback={null}>
        <FormulaEditor
          label={t("calculator.settings.formulaLabel")}
          value={formulaValue}
          disabled={busy}
          onChange={(v) => setSnap({ ...snap, [formulaKey]: v })}
          onBlur={(v) => void patch({ [formulaKey]: v })}
          onClear={() =>
            void hostApi.wbCalculator
              .clearProfitFormula(mp)
              .then((s) => setSnap(s as CalculatorSnapshot))
          }
        />
        <FormulaTokens busy={busy} onToken={onAppendToken} />
      </Suspense>
    </section>
  );
}
