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

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  hostApi,
  TemplateDto,
  TemplateGroupDto,
  TemplatesSnapshot,
} from "../../bridge/hostApi";
import { onTabsLayoutChanged } from "../../bridge/hostApi.layout";
import { useAppDialogs } from "../../components/useAppDialogs";
import { useT } from "../../i18n";
import { enrichSpreadsheetTitles } from "./enrichSpreadsheetTitles";
import {
  advanceGroupWaveFromActive,
  beginGroupWave,
  cancelGroupWave,
  emptySnap,
  ensureScriptsMap,
  ensureTemplatesRunProgressBridge,
  getTemplatesSnapCache,
  mergeEphemeralRunFlags,
  setTemplatesSnapCache,
  snapshotGroupWaves,
  subscribeGroupWaves,
  subscribeTemplatesSnap,
  type GroupWave,
  type TemplatesMarket,
} from "./templatesPageCache";

export function useTemplatesActions() {
  const t = useT();
  const [market, setMarket] = useState<TemplatesMarket>("wb");
  const [snap, setSnap] = useState<TemplatesSnapshot>(() => emptySnap("wb"));
  const [groupWaves, setGroupWaves] = useState<Record<string, GroupWave>>(
    () => snapshotGroupWaves(),
  );
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  /** Sync guard — setState alone allows double-click before re-render. */
  const runLockRef = useRef(false);
  const [editorId, setEditorId] = useState<string | null | undefined>(undefined);
  const [groupEditor, setGroupEditor] = useState<{ groupId?: string | null } | null>(null);
  const [groupPick, setGroupPick] = useState<{ templateId: string } | null>(null);
  const [tabMenu, setTabMenu] = useState<{ name: string; x: number; y: number } | null>(null);
  const { confirm, prompt, alert, dialogs } = useAppDialogs();

  const applySnap = useCallback((s: TemplatesSnapshot) => {
    setSnap((prev) => {
      const merged = mergeEphemeralRunFlags(prev, s);
      const m = (merged.marketplace === "ozon" ? "ozon" : "wb") as TemplatesMarket;
      setTemplatesSnapCache(m, merged);
      return merged;
    });
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
    ensureTemplatesRunProgressBridge();
    const cached = getTemplatesSnapCache(market);
    if (cached) {
      setSnap(cached);
      setError(null);
    }
    void ensureScriptsMap();
    // Always re-fetch: hydrate is_running from cancel_map (cache alone loses busy after Logs).
    void reload();
  }, [market, reload]);

  useEffect(() => {
    return subscribeTemplatesSnap((s) => {
      const m = (s.marketplace === "ozon" ? "ozon" : "wb") as TemplatesMarket;
      if (m !== market) return;
      // Same tick as snap: wave may have advanced in the bridge — force React paint.
      setGroupWaves(snapshotGroupWaves());
      setSnap((prev) => {
        const merged = mergeEphemeralRunFlags(prev, s);
        setTemplatesSnapCache(m, merged);
        return merged;
      });
    });
  }, [market]);

  useEffect(() => {
    // Remount / first paint: pull module wave (progress may have advanced on Logs).
    setGroupWaves(snapshotGroupWaves());
    return subscribeGroupWaves(() => {
      setGroupWaves(snapshotGroupWaves());
    });
  }, []);

  // Lesson 121: while group wave flies, poll hydrate cursor → same tones pipeline.
  // Covers emit batching (lesson 120) and missed progress events.
  const waveFlying = Object.values(groupWaves).some((w) => w.inFlight);
  useEffect(() => {
    if (!waveFlying) return;
    let dead = false;
    const tick = async () => {
      try {
        const s = await hostApi.templatesSnapshot(market);
        if (dead) return;
        let moved = false;
        for (const g of s.groups ?? []) {
          if (advanceGroupWaveFromActive(g.id, g.active_template_id)) moved = true;
        }
        if (moved) setGroupWaves(snapshotGroupWaves());
        // Refresh busy flags from hydrate without wiping session tones (merge).
        applySnap(s);
      } catch {
        /* mid-run poll is best-effort */
      }
    };
    const timer = window.setInterval(() => void tick(), 750);
    void tick();
    return () => {
      dead = true;
      window.clearInterval(timer);
    };
  }, [waveFlying, market, applySnap]);

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

  const patchOptimisticRunning = useCallback((id: string, kind: "template" | "group") => {
    setSnap((prev) => {
      let next: TemplatesSnapshot;
      if (kind === "template") {
        const mark = (tpl: TemplateDto): TemplateDto =>
          tpl.id === id ? { ...tpl, is_running: true } : tpl;
        next = {
          ...prev,
          templates: (prev.templates ?? []).map(mark),
          standalone: (prev.standalone ?? []).map(mark),
        };
      } else {
        const firstTpl = (prev.groups ?? []).find((g) => g.id === id)?.template_ids?.[0];
        next = {
          ...prev,
          groups: (prev.groups ?? []).map((g) =>
            g.id === id
              ? {
                  ...g,
                  is_running: true,
                  active_template_id: g.active_template_id || firstTpl || null,
                }
              : g,
          ),
        };
        if (firstTpl) {
          const memberIds = new Set(
            (prev.groups ?? []).find((g) => g.id === id)?.template_ids ?? [],
          );
          const markNested = (tpl: TemplateDto): TemplateDto =>
            memberIds.has(tpl.id)
              ? { ...tpl, is_running: tpl.id === firstTpl }
              : tpl;
          next = {
            ...next,
            templates: (next.templates ?? []).map(markNested),
            standalone: (next.standalone ?? []).map(markNested),
          };
        }
      }
      const m = (next.marketplace === "ozon" ? "ozon" : "wb") as TemplatesMarket;
      setTemplatesSnapCache(m, next);
      return next;
    });
  }, []);

  const run = useCallback(
    async (id: string, dryRun = false) => {
      if (runLockRef.current) return;
      runLockRef.current = true;
      setBusyId(id);
      setMsg(null);
      setError(null);
      patchOptimisticRunning(id, "template");
      try {
        const res = await hostApi.templatesRun(id, dryRun);
        const fallback = dryRun
          ? t("templates.dryRunDone", { count: res.rows_written })
          : t("templates.rowsWritten", { count: res.rows_written });
        setMsg(res.message || fallback);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        runLockRef.current = false;
        setBusyId(null);
        // С диска: иначе кэш/effect откатывает карточку к старому last_run_*.
        await reloadSnap();
      }
    },
    [t, reloadSnap, patchOptimisticRunning],
  );

  const runGroup = useCallback(
    async (id: string, dryRun = false) => {
      if (runLockRef.current) return;
      runLockRef.current = true;
      setBusyId(id);
      setMsg(null);
      setError(null);
      const memberIds =
        snap.groups.find((g) => g.id === id)?.template_ids ?? [];
      beginGroupWave(id, memberIds);
      const waves = snapshotGroupWaves();
      setGroupWaves(waves);
      const started = waves[id];
      patchOptimisticRunning(id, "group");
      // Same tick as optimistic busy: stamp wave_tones so paint does not wait on events.
      if (started) {
        setSnap((prev) => {
          const next: TemplatesSnapshot = {
            ...prev,
            groups: (prev.groups ?? []).map((g) =>
              g.id === id
                ? {
                    ...g,
                    wave_in_flight: true,
                    wave_tones: { ...started.tones },
                  }
                : g,
            ),
          };
          const m = (next.marketplace === "ozon" ? "ozon" : "wb") as TemplatesMarket;
          setTemplatesSnapCache(m, next);
          return next;
        });
      }
      try {
        const res = await hostApi.templatesRunGroup(id, dryRun);
        const fallback = dryRun
          ? t("templates.groupDryRunDone", { count: res.rows_written })
          : t("templates.groupRowsWritten", { count: res.rows_written });
        setMsg(res.message || fallback);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        runLockRef.current = false;
        setBusyId(null);
        await reloadSnap();
      }
    },
    [t, reloadSnap, patchOptimisticRunning, snap.groups],
  );

  const duplicateTemplate = useCallback(
    async (id: string) => {
      if (runLockRef.current) return;
      setBusyId(id);
      setMsg(null);
      setError(null);
      try {
        const { template } = await hostApi.templatesGet(id);
        const srcName = (template.name ?? "").trim() || id;
        const clone: TemplateDto = {
          ...template,
          id: crypto.randomUUID(),
          name: t("templates.copyName", { name: srcName }),
          last_run_status: "",
          last_run_at: "",
          is_running: false,
          created_at: undefined,
          updated_at: undefined,
        };
        const saved = await hostApi.templatesSave(clone);
        setMsg(t("templates.copiedOk", { name: saved.template.name }));
        await reloadSnap();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusyId(null);
      }
    },
    [t, reloadSnap],
  );

  const cancelRun = useCallback(async (id: string, entityType: "template" | "group") => {
    // Lesson 118: UI unlock immediately — do not wait for long Sheets/rate-limit.
    runLockRef.current = false;
    setBusyId(null);
    if (entityType === "group") {
      cancelGroupWave(id);
      const waves = snapshotGroupWaves();
      setGroupWaves(waves);
      const cancelled = waves[id];
      setSnap((prev) => {
        const next: TemplatesSnapshot = {
          ...prev,
          groups: (prev.groups ?? []).map((g) =>
            g.id === id
              ? {
                  ...g,
                  is_running: false,
                  active_template_id: null,
                  wave_in_flight: false,
                  wave_tones: cancelled?.tones ? { ...cancelled.tones } : g.wave_tones,
                }
              : g,
          ),
          templates: (prev.templates ?? []).map((tpl) => {
            const g = prev.groups?.find((x) => x.id === id);
            if (!g?.template_ids?.includes(tpl.id)) return tpl;
            return { ...tpl, is_running: false };
          }),
        };
        const m = (next.marketplace === "ozon" ? "ozon" : "wb") as TemplatesMarket;
        setTemplatesSnapCache(m, next);
        return next;
      });
    } else {
      setSnap((prev) => {
        const mark = (tpl: TemplateDto): TemplateDto =>
          tpl.id === id ? { ...tpl, is_running: false } : tpl;
        const next = {
          ...prev,
          templates: (prev.templates ?? []).map(mark),
          standalone: (prev.standalone ?? []).map(mark),
        };
        const m = (next.marketplace === "ozon" ? "ozon" : "wb") as TemplatesMarket;
        setTemplatesSnapCache(m, next);
        return next;
      });
    }
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
    groupWaves,
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
    duplicateTemplate,
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
