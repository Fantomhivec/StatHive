import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { hostApi } from "../../bridge/hostApi";
import { useAppDialogs } from "../../components/useAppDialogs";
import { useTabsLayoutCaps } from "../../hooks/useTabsLayoutCaps";
import { useT } from "../../i18n";
import { dictGet, type ColumnBinding, type TriggerRow } from "./calculatorTypes";
import {
  clearTableLinksAction,
  exportExcelAction,
  openExternalUrlAction,
  openTableAction,
  refreshOneAction,
  runCreateOrUpdateAllAction,
  saveEditLinkAction,
} from "./calculatorTablesActions";
import { useCalculatorSnapshot } from "./useCalculatorSnapshot";

export function useCalculatorTablesController() {
  const t = useT();
  const { snap, setSnap, error, setError, busy, setBusy, patch, reload } =
    useCalculatorSnapshot("tables");
  const { alert, confirm, dialogs } = useAppDialogs();
  const [editLink, setEditLink] = useState<{ key: string; label: string; url: string } | null>(
    null,
  );
  const [logsOpen, setLogsOpen] = useState(false);
  const [triggersOpen, setTriggersOpen] = useState(false);
  const [draftTriggers, setDraftTriggers] = useState<TriggerRow[]>([]);
  /* По умолчанию свёрнуты; разворот — в sessionStorage до перезапуска приложения. */
  const [wbOpen, setWbOpen] = useState(false);
  const [ozOpen, setOzOpen] = useState(false);
  const [busyAllExports, setBusyAllExports] = useState(false);
  const [busyWbExports, setBusyWbExports] = useState(false);
  const [busyOzExports, setBusyOzExports] = useState(false);
  const [busyExcelExport, setBusyExcelExport] = useState(false);
  const layoutCaps = useTabsLayoutCaps();
  const canMutate = layoutCaps.tablesCanMutate === true;
  const canExcel = layoutCaps.calcAccess?.excel === true;

  const uiLocked = busy || busyAllExports || busyExcelExport || !canMutate;
  const wbExportBusy = busyAllExports || busyWbExports || busyExcelExport;
  const ozExportBusy = busyAllExports || busyOzExports || busyExcelExport;

  const tableKeys = snap?.tableKeys ?? [];
  const tableModes = snap?.tableModes ?? {};
  const wbTables = useMemo(
    () => tableKeys.filter((row) => (row.marketplace ?? "wb").toLowerCase() !== "ozon"),
    [tableKeys],
  );
  const ozTables = useMemo(
    () => tableKeys.filter((row) => (row.marketplace ?? "").toLowerCase() === "ozon"),
    [tableKeys],
  );
  const showWbSection = wbTables.length > 0;
  const showOzSection = ozTables.length > 0;

  useEffect(() => {
    if (!snap?.bundleId) return;
    try {
      const raw = sessionStorage.getItem(`calc-tables-collapse:${snap.bundleId}`);
      if (!raw) return;
      const parsed = JSON.parse(raw) as { wb?: boolean; oz?: boolean };
      if (typeof parsed.wb === "boolean") setWbOpen(parsed.wb);
      if (typeof parsed.oz === "boolean") setOzOpen(parsed.oz);
    } catch {
      /* ignore */
    }
  }, [snap?.bundleId]);

  function persistCollapse(next: { wb: boolean; oz: boolean }) {
    if (!snap?.bundleId) return;
    try {
      sessionStorage.setItem(`calc-tables-collapse:${snap.bundleId}`, JSON.stringify(next));
    } catch {
      /* ignore */
    }
  }

  const manualMetrics = useMemo(() => {
    if (!snap) return [];
    const manualKeys = new Set(
      tableKeys
        .filter((row) => (dictGet(tableModes, row.key) || "auto") === "manual")
        .map((row) => row.key),
    );
    return (snap.metrics ?? []).filter((m) => m.tableKey && manualKeys.has(m.tableKey));
  }, [snap, tableModes, tableKeys]);

  const manualMetricsWb = useMemo(
    () => manualMetrics.filter((m) => (m.marketplace || "wb").toLowerCase() !== "ozon"),
    [manualMetrics],
  );
  const manualMetricsOz = useMemo(
    () => manualMetrics.filter((m) => (m.marketplace || "").toLowerCase() === "ozon"),
    [manualMetrics],
  );

  const setTableMode = useCallback(
    async (key: string, mode: string) => {
      if (!snap) return;
      const next: Record<string, string> = {};
      for (const row of tableKeys) {
        next[row.key] = dictGet(snap.tableModes, row.key) || (row.key === key ? mode : "auto");
      }
      next[key] = mode;
      setSnap({ ...snap, tableModes: next });
      await patch({ tableModes: next });
    },
    [snap, tableKeys, setSnap, patch],
  );

  const bindingsRef = useRef<ColumnBinding[]>(snap?.columnBindings ?? []);
  const [localBindings, setLocalBindings] = useState<ColumnBinding[]>(
    () => snap?.columnBindings ?? [],
  );
  useEffect(() => {
    const list = snap?.columnBindings ?? [];
    bindingsRef.current = list;
    setLocalBindings(list);
  }, [snap?.columnBindings]);

  const metricByKey = useMemo(() => {
    const map = new Map<string, { key: string; marketplace: string; tableKey?: string }>();
    for (const m of snap?.metrics ?? []) map.set(m.key, m);
    return map;
  }, [snap?.metrics]);

  const manualUrlByTable = useMemo(() => {
    const map = new Map<string, string>();
    for (const b of localBindings) {
      const url = b.spreadsheetUrl?.trim();
      if (!url) continue;
      const tableKey = metricByKey.get(b.metricKey)?.tableKey;
      if (tableKey && !map.has(tableKey)) map.set(tableKey, url);
    }
    return map;
  }, [localBindings, metricByKey]);

  function updateBinding(metricKey: string, partial: Partial<ColumnBinding>) {
    if (!snap) return;
    const list = [...(bindingsRef.current.length ? bindingsRef.current : snap.columnBindings ?? [])];
    const idx = list.findIndex((b) => b.metricKey === metricKey);
    const metricMp = (metricByKey.get(metricKey)?.marketplace || "wb").toLowerCase();
    const base: ColumnBinding =
      idx >= 0
        ? list[idx]
        : {
            metricKey,
            spreadsheetUrl: "",
            sheetName: "",
            columnHeader: "",
            articleColumnHeader: metricMp === "ozon" ? "SKU" : t("calculator.tables.articleColumnWb"),
          };
    const nextRow = { ...base, ...partial, metricKey };
    if (idx >= 0) list[idx] = nextRow;
    else list.push(nextRow);
    bindingsRef.current = list;
    setLocalBindings(list);
  }

  function saveBindings() {
    if (!snap) return;
    void patch({ columnBindings: bindingsRef.current });
  }

  async function saveEditLink() {
    if (!editLink) return;
    setBusy(true);
    try {
      setSnap(await saveEditLinkAction(editLink.key, editLink.url));
      setEditLink(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  const openTable = useCallback(async (key: string) => {
    try {
      await openTableAction(key);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [setError]);

  const openExternalUrl = useCallback(async (url: string) => {
    try {
      await openExternalUrlAction(url);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [setError]);

  const onOpenRow = useCallback(
    (key: string, link: string, openHref: string) => {
      if (link) void openTable(key);
      else void openExternalUrl(openHref);
    },
    [openTable, openExternalUrl],
  );

  const onEditLinkRow = useCallback((key: string, label: string, url: string) => {
    setEditLink({ key, label, url });
  }, []);

  async function runCreateOrUpdateAll(marketplace?: "wb" | "ozon") {
    if (!snap) return;
    const setMpBusy =
      marketplace === "wb"
        ? setBusyWbExports
        : marketplace === "ozon"
          ? setBusyOzExports
          : setBusyAllExports;
    setMpBusy(true);
    setError(null);
    try {
      const next = await runCreateOrUpdateAllAction(marketplace, { alert });
      if (next) setSnap(next);
      else await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setMpBusy(false);
    }
  }

  const refreshOne = useCallback(
    async (key: string) => {
      setBusy(true);
      setError(null);
      try {
        const next = await refreshOneAction(key, { alert });
        if (next) setSnap(next);
        else await reload();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusy(false);
      }
    },
    [alert, reload, setBusy, setError, setSnap],
  );

  async function clearTableLinks() {
    setBusy(true);
    try {
      const next = await clearTableLinksAction({ confirm });
      if (next) setSnap(next);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function exportExcel() {
    setBusyExcelExport(true);
    try {
      await exportExcelAction({ alert });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusyExcelExport(false);
    }
  }

  function openTriggers() {
    setDraftTriggers(
      (snap?.triggers ?? []).map((row) => ({
        tableKey: row.tableKey,
        displayName: row.displayName,
        mode: row.mode || "None",
        time: row.time || "09:00",
        intervalHours: row.intervalHours ?? 1,
        intervalMinutes: row.intervalMinutes ?? 0,
        nextExecution: row.nextExecution ?? "",
        scheduledDateTime: row.scheduledDateTime ?? "",
        weeklyDay: row.weeklyDay || "Monday",
        weeklyTime: row.weeklyTime || row.time || "09:00",
      })),
    );
    setTriggersOpen(true);
  }

  async function saveTriggers() {
    setBusy(true);
    try {
      const next = (await hostApi.wbCalculator.setTriggers(draftTriggers)) as typeof snap;
      if (next) setSnap(next);
      setTriggersOpen(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return {
    t,
    snap,
    setSnap,
    error,
    setError,
    busy,
    patch,
    dialogs,
    editLink,
    setEditLink,
    logsOpen,
    setLogsOpen,
    triggersOpen,
    setTriggersOpen,
    draftTriggers,
    setDraftTriggers,
    openTriggers,
    saveTriggers,
    wbOpen,
    setWbOpen,
    ozOpen,
    setOzOpen,
    busyAllExports,
    busyWbExports,
    busyOzExports,
    busyExcelExport,
    canMutate,
    canExcel,
    uiLocked,
    wbExportBusy,
    ozExportBusy,
    tableModes,
    wbTables,
    ozTables,
    showWbSection,
    showOzSection,
    persistCollapse,
    manualMetricsWb,
    manualMetricsOz,
    setTableMode,
    localBindings,
    manualUrlByTable,
    updateBinding,
    saveBindings,
    saveEditLink,
    openExternalUrl,
    onOpenRow,
    onEditLinkRow,
    runCreateOrUpdateAll,
    refreshOne,
    clearTableLinks,
    exportExcel,
  };
}
