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

import {
  useCallback,
  type Dispatch,
  type RefObject,
  type SetStateAction,
} from "react";
import {
  hostApi,
  type TemplateDto,
  type TemplateGroupDto,
  type TemplatesSnapshot,
} from "../../bridge/hostApi";
import {
  beginGroupWave,
  cancelGroupWave,
  finalizeGroupWaveFromHydrate,
  setTemplatesSnapCache,
  snapshotGroupWaves,
  type GroupWave,
  type TemplatesMarket,
} from "./templatesPageCache";

type ConfirmFn = (message: string, title?: string) => Promise<boolean>;

type Deps = {
  t: (key: string, vars?: Record<string, string | number>) => string;
  market: TemplatesMarket;
  groups: TemplateGroupDto[];
  confirm: ConfirmFn;
  withReload: (fn: () => Promise<TemplatesSnapshot>) => Promise<void>;
  reloadSnap: () => Promise<void>;
  setSnap: Dispatch<SetStateAction<TemplatesSnapshot>>;
  setBusyId: Dispatch<SetStateAction<string | null>>;
  setMsg: Dispatch<SetStateAction<string | null>>;
  setError: Dispatch<SetStateAction<string | null>>;
  setGroupWaves: Dispatch<SetStateAction<Record<string, GroupWave>>>;
  setEditorId: Dispatch<SetStateAction<string | null | undefined>>;
  setGroupEditor: Dispatch<SetStateAction<{ groupId?: string | null } | null>>;
  setGroupPick: Dispatch<SetStateAction<{ templateId: string } | null>>;
  runLockRef: RefObject<boolean>;
  seenHydrateRunningRef: RefObject<Set<string>>;
};

/** Run / cancel / CRUD / reorder for templates page — split from useTemplatesActions. */
export function useTemplatesEntityActions(d: Deps) {
  const {
    t,
    market,
    groups,
    confirm,
    withReload,
    reloadSnap,
    setSnap,
    setBusyId,
    setMsg,
    setError,
    setGroupWaves,
    setEditorId,
    setGroupEditor,
    setGroupPick,
    runLockRef,
    seenHydrateRunningRef,
  } = d;

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
  }, [setSnap]);

  const run = useCallback(
    async (id: string, dryRun = false) => {
      if (runLockRef.current) return;
      runLockRef.current = true;
      setBusyId(id);
      setMsg(null);
      setError(null);
      patchOptimisticRunning(id, "template");
      try {
        await hostApi.templatesRun(id, dryRun);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        runLockRef.current = false;
        setBusyId(null);
        setSnap((prev) => {
          const clear = (tpl: TemplateDto): TemplateDto =>
            tpl.id === id ? { ...tpl, is_running: false } : tpl;
          const next: TemplatesSnapshot = {
            ...prev,
            templates: (prev.templates ?? []).map(clear),
            standalone: (prev.standalone ?? []).map(clear),
          };
          const m = (next.marketplace === "ozon" ? "ozon" : "wb") as TemplatesMarket;
          setTemplatesSnapCache(m, next);
          return next;
        });
        await reloadSnap();
      }
    },
    [reloadSnap, patchOptimisticRunning, runLockRef, setBusyId, setMsg, setError, setSnap],
  );

  const runGroup = useCallback(
    async (id: string, dryRun = false) => {
      if (runLockRef.current) return;
      runLockRef.current = true;
      setBusyId(id);
      setMsg(null);
      setError(null);
      const memberIds = groups.find((g) => g.id === id)?.template_ids ?? [];
      beginGroupWave(id, memberIds);
      seenHydrateRunningRef.current.delete(id);
      const waves = snapshotGroupWaves();
      setGroupWaves(waves);
      const started = waves[id];
      patchOptimisticRunning(id, "group");
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
        await hostApi.templatesRunGroup(id, dryRun);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        runLockRef.current = false;
        setBusyId(null);
        finalizeGroupWaveFromHydrate(id, "ok");
        setGroupWaves(snapshotGroupWaves());
        await reloadSnap();
      }
    },
    [
      reloadSnap,
      patchOptimisticRunning,
      groups,
      runLockRef,
      seenHydrateRunningRef,
      setBusyId,
      setMsg,
      setError,
      setGroupWaves,
      setSnap,
    ],
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
    [t, reloadSnap, runLockRef, setBusyId, setMsg, setError],
  );

  const cancelRun = useCallback(
    async (id: string, entityType: "template" | "group") => {
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
    },
    [runLockRef, setBusyId, setGroupWaves, setSnap, setError],
  );

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
    [confirm, reloadSnap, t, setBusyId, setError],
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
    [confirm, reloadSnap, t, setBusyId, setError],
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

  const onEditTemplate = useCallback((id: string) => setEditorId(id), [setEditorId]);
  const onEditGroup = useCallback(
    (id: string) => setGroupEditor({ groupId: id }),
    [setGroupEditor],
  );
  const onAddToGroup = useCallback(
    (id: string) => setGroupPick({ templateId: id }),
    [setGroupPick],
  );
  const onCancelTemplate = useCallback(
    (id: string) => void cancelRun(id, "template"),
    [cancelRun],
  );
  const onCancelGroup = useCallback(
    (id: string) => void cancelRun(id, "group"),
    [cancelRun],
  );

  return {
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
  };
}
