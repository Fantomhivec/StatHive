import { useT } from "../../i18n";
import type { CalculatorSnapshot } from "./calculatorTypes";

export type CabinetChip = { name: string; marketplace: string; hasKey: boolean };

export function buildCabinetChips(snap: CalculatorSnapshot): CabinetChip[] {
  if (snap.cabinetChips && snap.cabinetChips.length > 0) return snap.cabinetChips;
  const available = new Set(snap.availableCabinets.map((x) => x.toLowerCase()));
  const names = [...new Set([...snap.availableCabinets, ...snap.selectedCabinets])];
  return names
    .filter(Boolean)
    .sort((a, b) => a.localeCompare(b, "ru"))
    .map((name) => ({
      name,
      marketplace: snap.cabinetMarketplaces?.[name] ?? "",
      hasKey: available.has(name.toLowerCase()),
    }));
}

export function chipMarketplace(c: CabinetChip, snap: CalculatorSnapshot): string {
  const fromChip = (c.marketplace || "").trim().toLowerCase();
  if (fromChip) return fromChip === "oz" ? "ozon" : fromChip;
  const fromMap = (snap.cabinetMarketplaces?.[c.name] || "").trim().toLowerCase();
  if (fromMap === "oz" || fromMap === "ozon") return "ozon";
  if (fromMap === "wb" || fromMap === "wildberries") return "wb";
  return "wb";
}

type Props = {
  snap: CalculatorSnapshot;
  busy: boolean;
  chips: CabinetChip[];
  mp: "wb" | "ozon";
  emptyHint: string;
  onToggle: (name: string) => void;
};

export function SettingsCabinetsBlock({
  snap,
  busy,
  chips,
  mp,
  emptyHint,
  onToggle,
}: Props) {
  const t = useT();
  const chipSet = new Set(chips.map((c) => c.name.toLowerCase()));
  const missing = (snap.missingKeyCabinets ?? []).filter((n) => chipSet.has(n.toLowerCase()));

  return (
    <>
      <h4 className="calc-subhead">
        {mp === "ozon" ? t("calculator.settings.cabinetsOzon") : t("calculator.settings.cabinetsWb")}
      </h4>
      {snap.usesLocalKeys !== false && missing.length > 0 && (
        <p className="wb-cabinets-missing">
          {t("calculator.settings.missingKeys", { list: missing.join(", ") })}
        </p>
      )}
      {snap.usesLocalKeys === false && (
        <p className="muted calc-hint">{t("calculator.settings.cabinetsNoKeys")}</p>
      )}
      <div className="wb-cabinets">
        {chips.length === 0 && <p className="muted">{emptyHint}</p>}
        {chips.map((c) =>
          snap.usesLocalKeys === false ? (
            <span key={c.name} className="wb-cabinet-chip">
              {c.name}
            </span>
          ) : (
            <label
              key={c.name}
              className={`wb-cabinet-chip${c.hasKey ? "" : " wb-cabinet-chip--missing"}`}
            >
              <input
                type="checkbox"
                checked={snap.selectedCabinets.includes(c.name)}
                onChange={() => onToggle(c.name)}
                disabled={busy}
              />
              <span>{c.name}</span>
            </label>
          ),
        )}
      </div>
    </>
  );
}
