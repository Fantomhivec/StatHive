import { useCallback, useEffect, useMemo, useState } from "react";
import {
  hostApi,
  onEvent,
  TemplateDto,
  TemplateGroupDto,
  TemplatesSnapshot,
} from "../../bridge/hostApi";
import { onTabsLayoutChanged } from "../../bridge/hostApi.layout";
import { useAppDialogs } from "../../components/useAppDialogs";
import { useT } from "../../i18n";
import { enrichSpreadsheetTitles } from "./enrichSpreadsheetTitles";
import {
  applyTemplatesRunProgress,
  emptySnap,
  ensureScriptsMap,
  getTemplatesSnapCache,
  setTemplatesSnapCache,
  type TemplatesMarket,
  type TemplatesRunProgress,
} from "./templatesPageCache";

export function useTemplatesActions() {
  const t = useT();
  const [market, setMarket] = useState<TemplatesMarket>("wb");
  const [snap, setSnap] = useState<TemplatesSnapshot>(() => emptySnap("wb"));
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [editorId, setEditorId] = useState<string | null | undefined>(undefined);
  const [groupEditor, setGroupEditor] = useState<{ groupId?: string | null } | null>(null);
  const [groupPick, setGroupPick] = useState<{ templateId: string } | null>(null);
  const [tabMenu, setTabMenu] = useState<{ name: string; x: number; y: number } | null>(null);
  const { confirm, prompt, alert, dialogs } = useAppDialogs();

  const applySnap = useCallback((s: TemplatesSnapshot) => {
    const m = (s.marketplace === "ozon" ? "ozon" : "wb") as TemplatesMarket;
    setTemplatesSnapCache(m, s);
    setSnap(s);
  }, []);

  /** Карточки сразу; названия Google — фоном (кэш сессии + один batch save). */
  const enrichTitlesInBackground = useCallback(
    (s: TemplatesSnapshot) => {
      void enrichSpreadsheetTitles(s)
        .then((enriched) => {
          if (enriched !== s) applySnap(enriched);
        })
        .catch(() => {
          /* названия подтянутся при следующем обновлении */
        });
    },
    [applySnap],
  );

  const reloadSnap = useCallback(async () => {
    setError(null);
    try {
      const s = await hostApi.templatesSnapshot(market);
      applySnap(s);
      enrichTitlesInBackground(s);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [market, applySnap, enrichTitlesInBackground]);

  const reload = useCallback(async () => {
    setError(null);
    try {
      const [s] = await Promise.all([
        hostApi.templatesSnapshot(market),
        ensureScriptsMap(),
      ]);
      applySnap(s);
      enrichTitlesInBackground(s);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [market, applySnap, enrichTitlesInBackground]);

  useEffect(() => {
    const cached = getTemplatesSnapCache(market);
    if (cached) {
      setSnap(cached);
      setError(null);
      void ensureScriptsMap();
      return;
    }
    void reload();
  }, [market, reload]);

  useEffect(() => {
    return onEvent<TemplatesRunProgress>("templates.runProgress", (p) => {
      setSnap((prev) => (prev ? applyTemplatesRunProgress(prev, p) : prev));
    });
  }, []);

  // Caps/пресеты догрузились после restore — убрать ложный «нет доступа» без ухода со вкладки.
  useEffect(() => {
    return onTabsLayoutChanged(() => {
      setError((prev) => {
        if (prev) void reload();
        return prev;
      });
    });
  }, [reload]);

  useEffect(() => {
    if (!tabMenu) return;
    const close = () => setTabMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [tabMenu]);

  const withReload = useCallback(
    async (fn: () => Promise<TemplatesSnapshot | unknown>) => {
      setError(null);
      try {
        const result = await fn();
        if (result && typeof result === "object" && "tabs" in (result as object)) {
          const s = result as TemplatesSnapshot;
          applySnap(s);
          enrichTitlesInBackground(s);
        } else {
          await reloadSnap();
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [applySnap, enrichTitlesInBackground, reloadSnap],
  );

  const run = useCallback(
    async (id: string, dryRun = false) => {
      setBusyId(id);
      setMsg(null);
      setError(null);
      try {
        const res = await hostApi.templatesRun(id, dryRun);
        const fallback = dryRun
          ? t("templates.dryRunDone", { count: res.rows_written })
          : t("templates.rowsWritten", { count: res.rows_written });
        setMsg(res.message || fallback);
        // Статус/is_running уже с `templates.runProgress` — полный snapshot не нужен.
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusyId(null);
      }
    },
    [t],
  );

  const runGroup = useCallback(
    async (id: string, dryRun = false) => {
      setBusyId(id);
      setMsg(null);
      setError(null);
      try {
        const res = await hostApi.templatesRunGroup(id, dryRun);
        const fallback = dryRun
          ? t("templates.groupDryRunDone", { count: res.rows_written })
          : t("templates.groupRowsWritten", { count: res.rows_written });
        setMsg(res.message || fallback);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusyId(null);
      }
    },
    [t],
  );

  const cancelRun = useCallback(async (id: string, entityType: "template" | "group") => {
    try {
      await hostApi.templatesCancel(id, entityType);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  const remove = useCallback(
    async (id: string) => {
      if (!(await confirm(t("templates.deleteTemplateConfirm"), t("dialogs.confirm")))) return;
      setBusyId(id);
      try {
        await hostApi.templatesDelete(id);
        await reloadSnap();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusyId(null);
      }
    },
    [confirm, reloadSnap, t],
  );

  const removeGroup = useCallback(
    async (id: string) => {
      if (!(await confirm(t("templates.deleteGroupConfirm"), t("dialogs.confirm")))) return;
      setBusyId(id);
      try {
        await hostApi.templatesDeleteGroup(id);
        await reloadSnap();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusyId(null);
      }
    },
    [confirm, reloadSnap, t],
  );

  const onExpandGroup = useCallback(
    (id: string, expanded: boolean) => {
      void withReload(async () => {
        await hostApi.templatesSetGroupExpanded(id, expanded);
        return hostApi.templatesSnapshot(market);
      });
    },
    [market, withReload],
  );

  const onMoveGroup = useCallback(
    (id: string, dir: "up" | "down") => {
      void withReload(async () => {
        await hostApi.templatesMoveGroup(id, dir);
        return hostApi.templatesSnapshot(market);
      });
    },
    [market, withReload],
  );

  const onMoveTemplateInGroup = useCallback(
    (groupId: string, id: string, dir: "up" | "down") => {
      void withReload(async () => {
        await hostApi.templatesMove(id, dir, groupId);
        return hostApi.templatesSnapshot(market);
      });
    },
    [market, withReload],
  );

  const onRemoveFromGroup = useCallback(
    (id: string) => {
      void withReload(async () => {
        await hostApi.templatesRemoveFromGroup(id);
        return hostApi.templatesSnapshot(market);
      });
    },
    [market, withReload],
  );

  const onMoveStandalone = useCallback(
    (id: string, dir: "up" | "down") => {
      void withReload(async () => {
        await hostApi.templatesMove(id, dir);
        return hostApi.templatesSnapshot(market);
      });
    },
    [market, withReload],
  );

  const onEditTemplate = useCallback((id: string) => setEditorId(id), []);
  const onEditGroup = useCallback((id: string) => setGroupEditor({ groupId: id }), []);
  const onAddToGroup = useCallback((id: string) => setGroupPick({ templateId: id }), []);
  const onCancelTemplate = useCallback(
    (id: string) => void cancelRun(id, "template"),
    [cancelRun],
  );
  const onCancelGroup = useCallback(
    (id: string) => void cancelRun(id, "group"),
    [cancelRun],
  );

  const byId = useMemo(() => {
    const m = new Map<string, TemplateDto>();
    for (const tpl of snap.templates) m.set(tpl.id, tpl);
    return m;
  }, [snap.templates]);

  const editingGroup: TemplateGroupDto | null = groupEditor?.groupId
    ? (snap.groups.find((g) => g.id === groupEditor.groupId) ?? null)
    : null;

  return {
    t,
    market,
    setMarket,
    snap,
    error,
    msg,
    busyId,
    editorId,
    setEditorId,
    groupEditor,
    setGroupEditor,
    groupPick,
    setGroupPick,
    tabMenu,
    setTabMenu,
    dialogs,
    confirm,
    prompt,
    alert,
    withReload,
    reloadSnap,
    run,
    runGroup,
    remove,
    removeGroup,
    onExpandGroup,
    onMoveGroup,
    onMoveTemplateInGroup,
    onRemoveFromGroup,
    onMoveStandalone,
    onEditTemplate,
    onEditGroup,
    onAddToGroup,
    onCancelTemplate,
    onCancelGroup,
    byId,
    editingGroup,
    editorOpen: editorId !== undefined,
  };
}
