import { lazy, Suspense } from "react";
import SpreadsheetIdField from "../components/templates/SpreadsheetIdField";
import { CalculatorTablesBody } from "./calculator/CalculatorTablesBody";
import { useCalculatorTablesController } from "./calculator/useCalculatorTablesController";
import "./WbCalculatorLayout.css";
import "./WbCalculatorPage.css";

const CalculatorRunLogsDialog = lazy(() =>
  import("./calculator/CalculatorRunLogsDialog").then((m) => ({
    default: m.CalculatorRunLogsDialog,
  })),
);
const CalculatorTriggersDialog = lazy(() =>
  import("./calculator/CalculatorTriggersDialog").then((m) => ({
    default: m.CalculatorTriggersDialog,
  })),
);

export default function CalculatorTablesPage() {
  const c = useCalculatorTablesController();
  const { t, snap, error, setError, busy, dialogs } = c;

  if (!snap) {
    return (
      <div className="calc-tables page wb-calc">
        <p className="muted">{error || t("common.loading")}</p>
        {dialogs}
      </div>
    );
  }

  return (
    <div className="calc-tables page wb-calc">
      <div className="sticky-page-header wb-calc-header wb-calc-header--sticky">
        <div className="wb-calc-header-title">
          <h1 className="page-title calc-page-title">{t("nav.calculatorTables")}</h1>
          {c.canMutate && snap.usesLocalKeys !== false && (
            <button
              type="button"
              className="btn btn-danger btn-sm"
              disabled={
                busy || c.busyAllExports || c.busyExcelExport || c.busyWbExports || c.busyOzExports
              }
              title={t("calculator.tables.clearLinksTitle")}
              onClick={() => void c.clearTableLinks()}
            >
              {t("calculator.tables.clearLinks")}
            </button>
          )}
        </div>
        <div className="wb-calc-header-actions">
          {c.canMutate && snap.usesLocalKeys !== false && (
            <label
              className="toggle-switch calc-triggers-toggle"
              title={t("calculator.tables.triggersToggleTitle")}
            >
              <input
                type="checkbox"
                checked={!!snap.triggersEnabled}
                disabled={c.uiLocked}
                onChange={(e) => void c.patch({ triggersEnabled: e.target.checked })}
              />
              <span className="toggle-track" />
              <span>{t("calculator.tables.triggersToggle")}</span>
            </label>
          )}
          {c.canMutate && snap.usesLocalKeys !== false && (
            <button
              type="button"
              className="btn btn-outline btn-sm"
              disabled={c.uiLocked}
              onClick={() => c.openTriggers()}
            >
              {t("calculator.tables.triggersBtn")}
            </button>
          )}
          {c.canMutate && snap.usesLocalKeys !== false && (
            <button
              type="button"
              className="btn btn-primary btn-sm btn-minw"
              disabled={
                busy || c.busyAllExports || c.busyExcelExport || c.busyWbExports || c.busyOzExports
              }
              title={
                snap.createButtonLabel === t("calculator.tables.createTablesBtn")
                  ? t("calculator.tables.createHint")
                  : t("calculator.tables.updateAllHint")
              }
              onClick={() => void c.runCreateOrUpdateAll()}
            >
              {c.busyAllExports
                ? t("common.exporting")
                : snap.createButtonLabel === t("calculator.tables.createTablesBtn")
                  ? t("calculator.tables.createTablesBtn")
                  : t("calculator.tables.updateAllTablesBtn")}
            </button>
          )}
          {c.canExcel && (
            <button
              type="button"
              className="btn btn-outline btn-sm"
              disabled={
                busy || c.busyAllExports || c.busyExcelExport || c.busyWbExports || c.busyOzExports
              }
              title={t("calculator.tables.exportExcelTitle")}
              onClick={() => void c.exportExcel()}
            >
              {c.busyExcelExport ? t("common.exportingExcel") : t("calculator.tables.exportExcel")}
            </button>
          )}
          <button
            type="button"
            className="btn btn-ghost btn-sm"
            title={t("calculator.tables.logsTitle")}
            onClick={() => c.setLogsOpen(true)}
          >
            {t("logs.title")}
          </button>
        </div>
      </div>

      {error && <p className="error">{error}</p>}
      {!c.canMutate ? (
        <p className="muted calc-hint">{t("calculator.tables.viewOnlyHint")}</p>
      ) : null}

      <CalculatorTablesBody
        snap={snap}
        canMutate={c.canMutate}
        busy={busy}
        uiLocked={c.uiLocked}
        wbExportBusy={c.wbExportBusy}
        ozExportBusy={c.ozExportBusy}
        tableModes={c.tableModes}
        wbTables={c.wbTables}
        ozTables={c.ozTables}
        showWbSection={c.showWbSection}
        showOzSection={c.showOzSection}
        wbOpen={c.wbOpen}
        ozOpen={c.ozOpen}
        busyWbExports={c.busyWbExports}
        busyOzExports={c.busyOzExports}
        manualMetricsWb={c.manualMetricsWb}
        manualMetricsOz={c.manualMetricsOz}
        localBindings={c.localBindings}
        manualUrlByTable={c.manualUrlByTable}
        onPersistCollapse={c.persistCollapse}
        onSetWbOpen={c.setWbOpen}
        onSetOzOpen={c.setOzOpen}
        onSetTableMode={c.setTableMode}
        onOpenRow={c.onOpenRow}
        onEditLinkRow={c.onEditLinkRow}
        onRefreshOne={c.refreshOne}
        onRunCreateOrUpdateAll={c.runCreateOrUpdateAll}
        onOpenExternalUrl={c.openExternalUrl}
        onUpdateBinding={c.updateBinding}
        onSaveBindings={c.saveBindings}
      />

      {c.canMutate && c.editLink && (
        <div className="wb-card">
          <h3>{t("calculator.tables.linkEditTitle", { label: c.editLink.label })}</h3>
          <SpreadsheetIdField
            idPrefix={`calc-link-${c.editLink.key}`}
            spreadsheetId={c.editLink.url}
            sheetName=""
            sheetOptions={[]}
            hideSheet
            disabled={busy}
            onSpreadsheetIdChange={(id) => c.setEditLink({ ...c.editLink!, url: id })}
            onSheetNameChange={() => {}}
            onSheetOptionsChange={() => {}}
            onMessage={(m) => {
              if (m) setError(m);
            }}
          />
          <div className="wb-calc-header-actions u-mt-8">
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy}
              onClick={() => void c.saveEditLink()}
            >
              {t("common.save")}
            </button>
            <button type="button" className="btn btn-ghost" onClick={() => c.setEditLink(null)}>
              {t("common.cancel")}
            </button>
          </div>
        </div>
      )}
      {c.logsOpen ? (
        <Suspense fallback={null}>
          <CalculatorRunLogsDialog
            open={c.logsOpen}
            onClose={() => c.setLogsOpen(false)}
            onError={(m) => setError(m)}
          />
        </Suspense>
      ) : null}
      {c.triggersOpen ? (
        <Suspense fallback={null}>
          <CalculatorTriggersDialog
            open={c.triggersOpen}
            busy={busy}
            draft={c.draftTriggers}
            onChange={c.setDraftTriggers}
            onSave={() => void c.saveTriggers()}
            onClose={() => c.setTriggersOpen(false)}
          />
        </Suspense>
      ) : null}
      {dialogs}
    </div>
  );
}
