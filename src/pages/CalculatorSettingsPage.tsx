import { lazy, Suspense, useMemo, useState } from "react";
import { useAuth } from "../auth/AuthContext";
import { isMainAdminRole } from "../bridge/accessRole";
import { hostApi, hostDialogs } from "../bridge/hostApi";
import { useTabsLayoutCaps } from "../hooks/useTabsLayoutCaps";
import AppComboBox from "../components/AppComboBox";
import AppMessageBox from "../components/AppMessageBox";
import { useAppDialogs } from "../components/useAppDialogs";
import { MARKETPLACE_OPTS, type CalculatorSnapshot } from "./calculator/calculatorTypes";
import {
  configExportFileName,
  defaultConfigExportPath,
} from "./calculator/configFileName";
import { spaceFormulaOperators } from "./calculator/formulaTokens";
import {
  buildCabinetChips,
  chipMarketplace,
  SettingsCabinetsBlock,
} from "./calculator/SettingsCabinetsBlock";
import { SettingsImportDialog } from "./calculator/SettingsImportDialog";
import { SettingsRatesFormulaCard } from "./calculator/SettingsRatesFormulaCard";
import { useCalculatorSnapshot } from "./calculator/useCalculatorSnapshot";
import "./WbCalculatorLayout.css";
import "./WbCalculatorPage.css";
import { useT } from "../i18n";

const TeamConfigsDialog = lazy(() => import("./calculator/TeamConfigsDialog"));

export default function CalculatorSettingsPage() {
  const t = useT();
  const { user } = useAuth();
  const isAdmin = isMainAdminRole(user?.access_level);
  const { snap, setSnap, error, setError, busy, setBusy, patch, reload } =
    useCalculatorSnapshot("settings");
  const { dialogs } = useAppDialogs();
  const [msg, setMsg] = useState<{
    title: string;
    message: string;
    tone?: "success" | "warning" | "error";
    width?: number;
  } | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [importJson, setImportJson] = useState("");
  const [teamOpen, setTeamOpen] = useState(false);
  /** Импорт пакета переписывает tableLinks — только tables+OAuth / Admin. */
  const layoutCaps = useTabsLayoutCaps();
  const canImportConfig = layoutCaps.tablesCanMutate === true;

  function toggleCabinet(name: string) {
    if (!snap) return;
    const set = new Set(snap.selectedCabinets);
    if (set.has(name)) set.delete(name);
    else set.add(name);
    void patch({ selectedCabinets: [...set] });
  }

  const cabinetChips = useMemo(() => (snap ? buildCabinetChips(snap) : []), [snap]);
  const wbCabinets = useMemo(
    () => (snap ? cabinetChips.filter((c) => chipMarketplace(c, snap) === "wb") : []),
    [cabinetChips, snap],
  );
  const ozonCabinets = useMemo(
    () => (snap ? cabinetChips.filter((c) => chipMarketplace(c, snap) === "ozon") : []),
    [cabinetChips, snap],
  );

  async function changeMarketplace(marketplace: string) {
    if (!snap) return;
    setSnap({ ...snap, marketplace });
    void patch({ marketplace });
  }

  async function copyConfig() {
    setBusy(true);
    try {
      const path = await hostDialogs.saveJsonFile(
        t("bridge.saveJsonConfig"),
        defaultConfigExportPath(snap?.calculatorName ?? ""),
      );
      if (!path?.trim()) return;
      const res = await hostApi.wbCalculator.exportConfigFile(path.trim());
      if (!res.ok || res.cancelled) return;
      if (res.json) {
        try {
          await navigator.clipboard.writeText(res.json);
        } catch {
          /* clipboard optional */
        }
      }
      setMsg({
        title: t("calculator.settings.exportSavedTitle"),
        message: t("calculator.settings.exportSavedBody", {
          pathSuffix: res.path
            ? t("calculator.settings.exportPathSuffix", { path: res.path })
            : t("calculator.settings.exportPathDot"),
        }),
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function pasteConfig() {
    setBusy(true);
    try {
      const next = await hostApi.wbCalculator.importConfig(importJson);
      setSnap(next as CalculatorSnapshot);
      setImportOpen(false);
      setImportJson("");
      setMsg({
        title: t("dialogs.attention"),
        tone: "warning",
        width: 560,
        message: t("calculator.settings.importWarnBody"),
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function importConfigFromFile() {
    setBusy(true);
    try {
      const path = await hostDialogs.pickJsonFile(t("bridge.pickJsonImport"));
      if (!path?.trim()) return;
      const res = await hostApi.wbCalculator.importConfigFile(path.trim());
      if (!res.ok || res.cancelled) return;
      if (res.snapshot) setSnap(res.snapshot as CalculatorSnapshot);
      setImportOpen(false);
      setImportJson("");
      setMsg({
        title: t("dialogs.attention"),
        tone: "warning",
        width: 560,
        message: t("calculator.settings.importDoneBody", {
          pathSuffix: res.path
            ? t("calculator.settings.importPathSuffix", { path: res.path })
            : "",
        }),
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  function appendToken(token: string, field: "wb" | "ozon" = "wb") {
    void hostApi.wbCalculator
      .appendProfitToken(token, field)
      .then((s) => {
        const next = s as CalculatorSnapshot;
        const key = field === "ozon" ? "ozonProfitFormula" : "profitFormula";
        const spaced = spaceFormulaOperators(next[key] ?? "");
        if (spaced !== (next[key] ?? "")) {
          setSnap({ ...next, [key]: spaced });
          void patch({ [key]: spaced });
        } else {
          setSnap(next);
        }
      })
      .catch((e) => setError(String(e)));
  }

  if (!snap) {
    return (
      <div className="page wb-calc">
        <p className="muted">{error || t("common.loading")}</p>
        {dialogs}
      </div>
    );
  }

  const showWbFormula = snap.marketplace === "wb" || snap.marketplace === "both";
  const showOzonFormula = snap.marketplace === "ozon" || snap.marketplace === "both";

  return (
    <div className="page wb-calc">
      <div className="wb-calc-header wb-calc-header--settings">
        <div className="wb-calc-name-wrap">
          <input
            className="input input-sm wb-calc-name"
            value={snap.calculatorName}
            disabled={busy}
            placeholder={t("calculator.settings.namePlaceholder")}
            title={t("calculator.settings.nameTitle")}
            aria-label={t("calculator.settings.nameAria")}
            onChange={(e) => setSnap({ ...snap, calculatorName: e.target.value })}
            onBlur={() => void patch({ calculatorName: snap.calculatorName })}
          />
          <span className="muted wb-calc-name-file">
            {t("common.fileLabel", { name: configExportFileName(snap.calculatorName) })}
          </span>
        </div>
        <div className="wb-calc-header-actions">
          <button
            type="button"
            className="btn btn-outline btn-sm"
            disabled={busy}
            title={t("calculator.settings.copyConfigTitle", {
              fileName: configExportFileName(snap.calculatorName),
            })}
            onClick={() => void copyConfig()}
          >
            {t("calculator.settings.copyConfig")}
          </button>
          {canImportConfig && (
            <button
              type="button"
              className="btn btn-outline btn-sm"
              disabled={busy}
              title={t("calculator.settings.pasteConfigTitle")}
              onClick={() => setImportOpen(true)}
            >
              {t("calculator.settings.importDialogTitle")}
            </button>
          )}
          {isAdmin && (
            <button
              type="button"
              className="btn btn-outline btn-sm"
              disabled={busy}
              title={t("calculator.settings.teamConfigsTitle")}
              onClick={() => setTeamOpen(true)}
            >
              {t("calculator.settings.teamConfigs")}
            </button>
          )}
        </div>
      </div>

      {error && <p className="error">{error}</p>}

      <section className="wb-card">
        <h3>{t("common.marketplace")}</h3>
        <div className="calc-field-row">
          <span className="calc-field-label">{t("common.marketplace")}</span>
          <div className="calc-field-control">
            <AppComboBox
              value={snap.marketplace || "wb"}
              options={MARKETPLACE_OPTS}
              onChange={(marketplace) => void changeMarketplace(marketplace)}
            />
          </div>
        </div>
        <p className="muted calc-hint">{t("calculator.settings.marketplaceHint")}</p>
      </section>

      {showWbFormula && (
        <SettingsRatesFormulaCard
          mp="wb"
          snap={snap}
          busy={busy}
          setSnap={setSnap}
          patch={patch}
          onAppendToken={(tok) => appendToken(tok, "wb")}
          cabinets={
            <SettingsCabinetsBlock
              snap={snap}
              busy={busy}
              chips={wbCabinets}
              mp="wb"
              emptyHint={t("calculator.settings.noWbCabinets")}
              onToggle={toggleCabinet}
            />
          }
        />
      )}

      {showOzonFormula && (
        <SettingsRatesFormulaCard
          mp="ozon"
          snap={snap}
          busy={busy}
          setSnap={setSnap}
          patch={patch}
          onAppendToken={(tok) => appendToken(tok, "ozon")}
          cabinets={
            <SettingsCabinetsBlock
              snap={snap}
              busy={busy}
              chips={ozonCabinets}
              mp="ozon"
              emptyHint={t("calculator.settings.noOzonCabinets")}
              onToggle={toggleCabinet}
            />
          }
        />
      )}

      <SettingsImportDialog
        open={importOpen}
        busy={busy}
        importJson={importJson}
        onImportJsonChange={setImportJson}
        onPaste={() => void pasteConfig()}
        onImportFile={() => void importConfigFromFile()}
        onClose={() => setImportOpen(false)}
      />

      {teamOpen ? (
        <Suspense fallback={null}>
          <TeamConfigsDialog
            open={teamOpen}
            isAdmin={isAdmin}
            busy={busy}
            onClose={() => setTeamOpen(false)}
            onBusy={setBusy}
            onError={setError}
            onMessage={(title, message, tone) => setMsg({ title, message, tone })}
            onLoaded={() => void reload()}
          />
        </Suspense>
      ) : null}

      {msg && (
        <AppMessageBox
          title={msg.title}
          message={msg.message}
          tone={msg.tone}
          width={msg.width}
          onClose={() => setMsg(null)}
        />
      )}
      {dialogs}
    </div>
  );
}
