import { useCallback } from "react";
import { useT } from "../../i18n";
import type { CalculatorSnapshot, ColumnBinding, TableKey } from "./calculatorTypes";
import { dictGet } from "./calculatorTypes";
import { CalculatorTableRow } from "./CalculatorTableRow";
import { CalculatorTablesManual } from "./CalculatorTablesManual";
import { CalculatorTablesPlaque } from "./CalculatorTablesPlaque";

type Metric = CalculatorSnapshot["metrics"][number];

type Props = {
  snap: CalculatorSnapshot;
  canMutate: boolean;
  busy: boolean;
  uiLocked: boolean;
  wbExportBusy: boolean;
  ozExportBusy: boolean;
  tableModes: Record<string, string>;
  wbTables: TableKey[];
  ozTables: TableKey[];
  showWbSection: boolean;
  showOzSection: boolean;
  wbOpen: boolean;
  ozOpen: boolean;
  busyWbExports: boolean;
  busyOzExports: boolean;
  manualMetricsWb: Metric[];
  manualMetricsOz: Metric[];
  localBindings: ColumnBinding[];
  manualUrlByTable: Map<string, string>;
  onPersistCollapse: (next: { wb: boolean; oz: boolean }) => void;
  onSetWbOpen: (open: boolean) => void;
  onSetOzOpen: (open: boolean) => void;
  onSetTableMode: (key: string, mode: string) => void | Promise<void>;
  onOpenRow: (key: string, link: string, openHref: string) => void;
  onEditLinkRow: (key: string, label: string, url: string) => void;
  onRefreshOne: (key: string) => void | Promise<void>;
  onRunCreateOrUpdateAll: (marketplace?: "wb" | "ozon") => void | Promise<void>;
  onOpenExternalUrl: (url: string) => void | Promise<void>;
  onUpdateBinding: (metricKey: string, partial: Partial<ColumnBinding>) => void;
  onSaveBindings: () => void;
};

export function CalculatorTablesBody({
  snap,
  canMutate,
  busy,
  uiLocked,
  wbExportBusy,
  ozExportBusy,
  tableModes,
  wbTables,
  ozTables,
  showWbSection,
  showOzSection,
  wbOpen,
  ozOpen,
  busyWbExports,
  busyOzExports,
  manualMetricsWb,
  manualMetricsOz,
  localBindings,
  manualUrlByTable,
  onPersistCollapse,
  onSetWbOpen,
  onSetOzOpen,
  onSetTableMode,
  onOpenRow,
  onEditLinkRow,
  onRefreshOne,
  onRunCreateOrUpdateAll,
  onOpenExternalUrl,
  onUpdateBinding,
  onSaveBindings,
}: Props) {
  const t = useT();

  const renderTableRow = useCallback(
    (row: TableKey) => {
      const mode = dictGet(tableModes, row.key) || "auto";
      const isOzon = (row.marketplace ?? "").toLowerCase() === "ozon";
      const link = dictGet(snap.tableLinks, row.key) || "";
      const manualLink = manualUrlByTable.get(row.key) || "";
      const openHref = link || manualLink;
      const updated = dictGet(snap.lastUpdated, row.key) || "";
      const rowBusy = busy || (isOzon ? ozExportBusy : wbExportBusy);
      return (
        <CalculatorTableRow
          key={row.key}
          tableKey={row.key}
          label={row.label}
          marketplace={row.marketplace}
          mode={mode}
          link={link}
          openHref={openHref}
          updated={updated}
          rowBusy={rowBusy}
          onSetMode={onSetTableMode}
          onOpen={onOpenRow}
          onEditLink={onEditLinkRow}
          onRefresh={onRefreshOne}
          showApiActions={snap.usesLocalKeys !== false}
          canMutate={canMutate}
        />
      );
    },
    [
      snap,
      tableModes,
      manualUrlByTable,
      busy,
      ozExportBusy,
      wbExportBusy,
      onSetTableMode,
      onOpenRow,
      onEditLinkRow,
      onRefreshOne,
      canMutate,
    ],
  );

  return (
    <>
      {!showWbSection && !showOzSection && (
        <p className="muted calc-hint">{t("calculator.tables.noTables")}</p>
      )}

      {showWbSection && (
        <CalculatorTablesPlaque
          title={t("calculator.marketplace.wb")}
          tone="wb"
          count={wbTables.length}
          open={wbOpen}
          onToggle={() => {
            const next = !wbOpen;
            onSetWbOpen(next);
            onPersistCollapse({ wb: next, oz: ozOpen });
          }}
          refreshLabel={t("calculator.tables.refreshWb")}
          refreshBusy={busyWbExports}
          refreshDisabled={busy || wbExportBusy || !canMutate}
          refreshTitle={t("calculator.tables.refreshWbTitle")}
          showRefresh={canMutate && snap.usesLocalKeys !== false}
          onRefresh={() => void onRunCreateOrUpdateAll("wb")}
          hint={t("calculator.tables.plaqueHintWb")}
        >
          <div className="wb-tables">{wbTables.map((row) => renderTableRow(row))}</div>
        </CalculatorTablesPlaque>
      )}

      {showOzSection && (
        <CalculatorTablesPlaque
          title={t("calculator.marketplace.ozon")}
          tone="ozon"
          count={ozTables.length}
          open={ozOpen}
          onToggle={() => {
            const next = !ozOpen;
            onSetOzOpen(next);
            onPersistCollapse({ wb: wbOpen, oz: next });
          }}
          refreshLabel={t("calculator.tables.refreshOzon")}
          refreshBusy={busyOzExports}
          refreshDisabled={busy || ozExportBusy || !canMutate}
          refreshTitle={t("calculator.tables.refreshOzonTitle")}
          showRefresh={canMutate && snap.usesLocalKeys !== false}
          onRefresh={() => void onRunCreateOrUpdateAll("ozon")}
          hint={t("calculator.tables.plaqueHintOzon")}
        >
          <div className="wb-tables">{ozTables.map((row) => renderTableRow(row))}</div>
        </CalculatorTablesPlaque>
      )}

      {canMutate && manualMetricsWb.length > 0 && (
        <section className="wb-card">
          <h3>{t("calculator.tables.manualWbTitle")}</h3>
          <p className="muted calc-hint">{t("calculator.tables.manualWbHint")}</p>
          <CalculatorTablesManual
            bindings={localBindings}
            metrics={manualMetricsWb}
            articlePlaceholder={t("calculator.tables.articleColumnWb")}
            disabled={uiLocked}
            onOpenUrl={(u) => void onOpenExternalUrl(u)}
            onUpdateBinding={onUpdateBinding}
            onSaveBindings={onSaveBindings}
          />
        </section>
      )}

      {canMutate && manualMetricsOz.length > 0 && (
        <section className="wb-card">
          <h3>{t("calculator.tables.manualOzonTitle")}</h3>
          <p className="muted calc-hint">{t("calculator.tables.manualOzonHint")}</p>
          <CalculatorTablesManual
            bindings={localBindings}
            metrics={manualMetricsOz}
            articlePlaceholder="SKU"
            disabled={uiLocked}
            onOpenUrl={(u) => void onOpenExternalUrl(u)}
            onUpdateBinding={onUpdateBinding}
            onSaveBindings={onSaveBindings}
          />
        </section>
      )}
    </>
  );
}
