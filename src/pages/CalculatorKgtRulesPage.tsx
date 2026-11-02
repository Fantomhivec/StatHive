import AppComboBox from "../components/AppComboBox";
import { useCalculatorSnapshot } from "./calculator/useCalculatorSnapshot";
import {
  coalesceOzKgt,
  coalesceWbKgt,
  DEFAULT_OZ_KGT,
  DEFAULT_WB_KGT,
  KGT_CMP_OPTIONS,
  type KgtCmp,
  type OzKgtRules,
  type WbKgtRules,
} from "./calculator/calculatorTypes";
import "./WbCalculatorLayout.css";
import "./WbCalculatorPage.css";
import { useT } from "../i18n";

function sameWb(a: WbKgtRules, b: WbKgtRules) {
  return (
    a.maxOneSideCm === b.maxOneSideCm &&
    a.oneSideCmp === b.oneSideCmp &&
    a.maxSumSidesCm === b.maxSumSidesCm &&
    a.sumSidesCmp === b.sumSidesCmp &&
    a.maxWeightKg === b.maxWeightKg &&
    a.weightCmp === b.weightCmp
  );
}

function sameOz(a: OzKgtRules, b: OzKgtRules) {
  return (
    a.maxSumSides === b.maxSumSides &&
    a.sumSidesCmp === b.sumSidesCmp &&
    a.maxWeight === b.maxWeight &&
    a.weightCmp === b.weightCmp &&
    a.maxVolume === b.maxVolume &&
    a.volumeCmp === b.volumeCmp &&
    a.bonus === b.bonus
  );
}

function cmpSymbol(cmp: KgtCmp) {
  return cmp === "gte" ? "≥" : ">";
}

function normalizeCmp(v: string): KgtCmp {
  return v === "gte" ? "gte" : "gt";
}

type RuleRowProps = {
  label: string;
  unit?: string;
  cmp: KgtCmp;
  value: number;
  disabled?: boolean;
  onCmp: (cmp: KgtCmp) => void;
  onValueLocal: (n: number) => void;
  onValueCommit: (n: number) => void;
};

function KgtRuleRow({
  label,
  unit,
  cmp,
  value,
  disabled,
  onCmp,
  onValueLocal,
  onValueCommit,
}: RuleRowProps) {
  const t = useT();
  return (
    <div className="plate kgt-rule-row">
      <div className="kgt-rule-label">
        <span className="kgt-rule-name">{label}</span>
        {unit ? <span className="muted kgt-rule-unit">{unit}</span> : null}
      </div>
      <AppComboBox
        className="app-combo-sm kgt-rule-cmp"
        value={cmp}
        options={KGT_CMP_OPTIONS}
        onChange={(v) => onCmp(normalizeCmp(v))}
        disabled={disabled}
        title={t("calculator.kgt.cmpTitle")}
      />
      <span className="kgt-rule-sym" aria-hidden="true">
        {cmpSymbol(cmp)}
      </span>
      <input
        className="input-compact kgt-rule-value"
        type="number"
        value={value}
        disabled={disabled}
        onChange={(e) => onValueLocal(Number(e.target.value))}
        onBlur={(e) => onValueCommit(Number(e.target.value))}
      />
    </div>
  );
}

export default function CalculatorKgtRulesPage() {
  const t = useT();
  const { snap, setSnap, error, setError, busy, patch } = useCalculatorSnapshot("kgt");

  if (!snap) {
    return (
      <div className="page wb-calc">
        <p className="muted">{error || t("common.loading")}</p>
      </div>
    );
  }

  const wb = coalesceWbKgt(snap.wbKgt);
  const oz = coalesceOzKgt(snap.ozKgt);
  const isDefault = sameWb(wb, DEFAULT_WB_KGT) && sameOz(oz, DEFAULT_OZ_KGT);

  async function persist(partial: { wbKgt?: WbKgtRules; ozKgt?: OzKgtRules }) {
    setError(null);
    try {
      await patch(partial);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  function patchWb(next: WbKgtRules) {
    setSnap({ ...snap!, wbKgt: next });
    return persist({ wbKgt: next });
  }

  function patchOz(next: OzKgtRules) {
    setSnap({ ...snap!, ozKgt: next });
    return persist({ ozKgt: next });
  }

  async function resetToDefaults() {
    const wbKgt = { ...DEFAULT_WB_KGT };
    const ozKgt = { ...DEFAULT_OZ_KGT };
    setSnap({ ...snap!, wbKgt, ozKgt });
    await persist({ wbKgt, ozKgt });
  }

  return (
    <div className="page wb-calc">
      <div className="wb-calc-header">
        <h2>{t("nav.calculatorKgtRules")}</h2>
        <div className="wb-calc-header-actions">
          <button
            type="button"
            className="btn btn-outline btn-sm"
            disabled={busy || isDefault}
            title="WB: ≥120 / ≥200 / ≥25; Ozon: >200 / >35 / >500 + 280 ₽"
            onClick={() => void resetToDefaults()}
          >
            {t("calculator.kgt.resetDefaults")}
          </button>
        </div>
      </div>
      {error && <p className="error">{error}</p>}
      <p className="muted calc-hint">{t("calculator.kgt.intro")}</p>

      <section className="wb-card">
        <h3>{t("calculator.kgt.wbTitle")}</h3>
        <p className="muted calc-hint">
          {t("calculator.kgt.wbHint")}
        </p>
        <div className="kgt-rules-list">
          <KgtRuleRow
            label={t("calculator.kgt.oneSide")}
            unit={t("calculator.card.unitCm")}
            cmp={wb.oneSideCmp}
            value={wb.maxOneSideCm}
            disabled={busy}
            onCmp={(oneSideCmp) => void patchWb({ ...wb, oneSideCmp })}
            onValueLocal={(maxOneSideCm) => setSnap({ ...snap, wbKgt: { ...wb, maxOneSideCm } })}
            onValueCommit={(maxOneSideCm) => void patchWb({ ...wb, maxOneSideCm })}
          />
          <KgtRuleRow
            label={t("calculator.kgt.sumSides")}
            unit={t("calculator.card.unitCm")}
            cmp={wb.sumSidesCmp}
            value={wb.maxSumSidesCm}
            disabled={busy}
            onCmp={(sumSidesCmp) => void patchWb({ ...wb, sumSidesCmp })}
            onValueLocal={(maxSumSidesCm) => setSnap({ ...snap, wbKgt: { ...wb, maxSumSidesCm } })}
            onValueCommit={(maxSumSidesCm) => void patchWb({ ...wb, maxSumSidesCm })}
          />
          <KgtRuleRow
            label={t("calculator.card.weight")}
            unit={t("calculator.card.unitKg")}
            cmp={wb.weightCmp}
            value={wb.maxWeightKg}
            disabled={busy}
            onCmp={(weightCmp) => void patchWb({ ...wb, weightCmp })}
            onValueLocal={(maxWeightKg) => setSnap({ ...snap, wbKgt: { ...wb, maxWeightKg } })}
            onValueCommit={(maxWeightKg) => void patchWb({ ...wb, maxWeightKg })}
          />
        </div>
      </section>

      <section className="wb-card">
        <h3>{t("calculator.kgt.ozonTitle")}</h3>
        <p className="muted calc-hint">{t("calculator.kgt.ozonHint")}</p>
        <div className="kgt-rules-list">
          <KgtRuleRow
            label={t("calculator.kgt.sumSides")}
            unit={t("calculator.card.unitCm")}
            cmp={oz.sumSidesCmp}
            value={oz.maxSumSides}
            disabled={busy}
            onCmp={(sumSidesCmp) => void patchOz({ ...oz, sumSidesCmp })}
            onValueLocal={(maxSumSides) => setSnap({ ...snap, ozKgt: { ...oz, maxSumSides } })}
            onValueCommit={(maxSumSides) => void patchOz({ ...oz, maxSumSides })}
          />
          <KgtRuleRow
            label={t("calculator.card.weight")}
            unit={t("calculator.card.unitKg")}
            cmp={oz.weightCmp}
            value={oz.maxWeight}
            disabled={busy}
            onCmp={(weightCmp) => void patchOz({ ...oz, weightCmp })}
            onValueLocal={(maxWeight) => setSnap({ ...snap, ozKgt: { ...oz, maxWeight } })}
            onValueCommit={(maxWeight) => void patchOz({ ...oz, maxWeight })}
          />
          <KgtRuleRow
            label={t("calculator.card.volume")}
            unit={t("calculator.card.unitL")}
            cmp={oz.volumeCmp}
            value={oz.maxVolume}
            disabled={busy}
            onCmp={(volumeCmp) => void patchOz({ ...oz, volumeCmp })}
            onValueLocal={(maxVolume) => setSnap({ ...snap, ozKgt: { ...oz, maxVolume } })}
            onValueCommit={(maxVolume) => void patchOz({ ...oz, maxVolume })}
          />
          <div className="plate kgt-rule-row kgt-rule-row--bonus">
            <div className="kgt-rule-label">
              <span className="kgt-rule-name">{t("calculator.kgt.kgtBonus")}</span>
              <span className="muted kgt-rule-unit">₽</span>
            </div>
            <span className="kgt-rule-bonus-spacer" aria-hidden="true" />
            <input
              className="input-compact kgt-rule-value"
              type="number"
              value={oz.bonus}
              disabled={busy}
              onChange={(e) =>
                setSnap({ ...snap, ozKgt: { ...oz, bonus: Number(e.target.value) } })
              }
              onBlur={(e) => void patchOz({ ...oz, bonus: Number(e.target.value) })}
            />
          </div>
        </div>
      </section>
    </div>
  );
}
