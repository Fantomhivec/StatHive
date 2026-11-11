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
  emptySnap,
  ensureScriptsMap,
  ensureTemplatesRunProgressBridge,
  finalizeGroupWaveFromHydrate,
  getTemplatesSnapCache,
  mergeEphemeralRunFlags,
  setTemplatesSnapCache,
  snapshotGroupWaves,
  subscribeGroupWaves,
  subscribeTemplatesSnap,
  type GroupWave,
  type TemplatesMarket,
} from "./templatesPageCache";
import { useTemplatesEntityActions } from "./useTemplatesEntityActions";

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
      // Cache-first paint: never show Stop from stale is_running; reload restores if live.
      setSnap({
        ...cached,
        groups: (cached.groups ?? []).map((g) => ({
          ...g,
          is_running: false,
          active_template_id: null,
        })),
        templates: (cached.templates ?? []).map((tpl) => ({
          ...tpl,
          is_running: false,
        })),
        standalone: (cached.standalone ?? []).map((tpl) => ({
          ...tpl,
          is_running: false,
        })),
      });
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

  // Groups hydrate already reported is_running — required before hydrate-end finalize
  // (avoids closing wave on the tick before Rust register enters cancel_map).
  const seenHydrateRunningRef = useRef(new Set<string>());
  /** Consecutive !is_running polls after seen running — avoid mid-group false finalize. */
  const idleHydrateTicksRef = useRef(new Map<string, number>());

  // Lesson 121: while group wave flies, poll hydrate cursor → same tones pipeline.
  // Lesson 123: hydrate !is_running + inFlight → finalize (finish events may be lost).
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
          if (g.is_running) {
            seenHydrateRunningRef.current.add(g.id);
            idleHydrateTicksRef.current.set(g.id, 0);
          }
          if (advanceGroupWaveFromActive(g.id, g.active_template_id)) moved = true;
          // Group left cancel_map — close wave even if runProgress finish never arrived.
          // Require 2 idle ticks so a brief gap between nested runs cannot flash «N of M».
          if (!g.is_running && seenHydrateRunningRef.current.has(g.id)) {
            const n = (idleHydrateTicksRef.current.get(g.id) ?? 0) + 1;
            idleHydrateTicksRef.current.set(g.id, n);
            if (
              n >= 2 &&
              busyId !== g.id &&
              finalizeGroupWaveFromHydrate(g.id, g.last_run_status)
            ) {
              seenHydrateRunningRef.current.delete(g.id);
              idleHydrateTicksRef.current.delete(g.id);
              moved = true;
              setBusyId((id) => (id === g.id ? null : id));
            }
          }
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
  }, [waveFlying, market, applySnap, busyId]);

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

  const {
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
  } = useTemplatesEntityActions({
    t,
    market,
    groups: snap.groups ?? [],
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
  });

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
