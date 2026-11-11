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
  hostApi,
  onEvent,
  type ScriptGroupDto,
  type TemplateDto,
  type TemplateGroupDto,
  type TemplatesSnapshot,
} from "../../bridge/hostApi";
import {
  clearTemplatesSnapCache,
  getTemplatesSnapCache,
  setTemplatesSnapCache,
  type TemplatesMarket,
} from "../../bridge/snapCache";
import { t as tr } from "../../i18n";
import {
  applyGroupWaveFromProgress,
  getGroupWave,
  normalizeRunProgress,
  type TemplatesRunProgress,
} from "./templatesGroupWave";

export type { TemplatesMarket, TemplatesRunProgress };
export { clearTemplatesSnapCache, getTemplatesSnapCache, setTemplatesSnapCache };

export {
  advanceGroupWaveFromActive,
  beginGroupWave,
  cancelGroupWave,
  finalizeGroupWaveFromHydrate,
  groupWaveSuccessCount,
  snapshotGroupWaves,
  subscribeGroupWaves,
  waveFromGroup,
  waveRowClass,
  type GroupWave,
  type WaveTone,
} from "./templatesGroupWave";

type SnapListener = (snap: TemplatesSnapshot) => void;
const snapListeners = new Set<SnapListener>();
let runProgressBridgeStarted = false;
let runProgressUnlisten: (() => void) | null = null;

/**
 * Keep templates snap cache updated even when TemplatesPage is unmounted (Logs tab).
 * Call once from App / Templates mount.
 */
export function ensureTemplatesRunProgressBridge() {
  if (runProgressBridgeStarted) return;
  runProgressBridgeStarted = true;
  runProgressUnlisten = onEvent<TemplatesRunProgress>("templates.runProgress", (raw) => {
    const p = normalizeRunProgress(raw);
    applyGroupWaveFromProgress(p);
    for (const market of ["wb", "ozon"] as TemplatesMarket[]) {
      const prev = getTemplatesSnapCache(market);
      if (!prev) continue;
      const next = applyTemplatesRunProgress(prev, p);
      const withWave: TemplatesSnapshot = {
        ...next,
        groups: (next.groups ?? []).map((g) => {
          const w = getGroupWave(g.id);
          if (!w) return { ...g, wave_tones: undefined, wave_in_flight: undefined };
          return {
            ...g,
            wave_tones: { ...w.tones },
            wave_in_flight: w.inFlight,
          };
        }),
      };
      setTemplatesSnapCache(market, withWave);
      for (const l of snapListeners) l(withWave);
    }
  });
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    runProgressUnlisten?.();
    runProgressUnlisten = null;
    runProgressBridgeStarted = false;
  });
}

/** Subscribe to cache patches (run progress). Returns unsubscribe. */
export function subscribeTemplatesSnap(listener: SnapListener): () => void {
  ensureTemplatesRunProgressBridge();
  snapListeners.add(listener);
  return () => {
    snapListeners.delete(listener);
  };
}

/**
 * Preserve session wave_* on groups across hydrate/enrich.
 * Lesson 117/123: never sticky-OR is_running (group / nested / solo) — mid-flight
 * Stop is busyId; sticky after success left «выполняется…» forever.
 */
export function mergeEphemeralRunFlags(
  prev: TemplatesSnapshot | null | undefined,
  next: TemplatesSnapshot,
): TemplatesSnapshot {
  if (!prev) return next;
  const prevGroup = new Map((prev.groups ?? []).map((g) => [g.id, g]));

  const groups = (next.groups ?? []).map((g) => {
    const p = prevGroup.get(g.id);
    if (!p) return g;
    const is_running = !!g.is_running;
    const active_template_id = is_running
      ? (g.active_template_id ?? p.active_template_id ?? null)
      : null;
    const wave_in_flight =
      g.wave_in_flight != null
        ? !!g.wave_in_flight
        : is_running
          ? !!p.wave_in_flight
          : false;
    const wave_tones =
      g.wave_tones ?? (wave_in_flight || is_running ? p.wave_tones : undefined);
    if (!is_running && !active_template_id && !wave_in_flight && !wave_tones) return g;
    return {
      ...g,
      is_running,
      active_template_id,
      wave_in_flight: wave_in_flight || undefined,
      wave_tones,
    };
  });

  return {
    ...next,
    groups,
    templates: next.templates ?? [],
    standalone: next.standalone ?? [],
  };
}

/** Патч snap по событию templates.runProgress. */
export function applyTemplatesRunProgress(
  prev: TemplatesSnapshot,
  p: TemplatesRunProgress,
): TemplatesSnapshot {
  const next = { ...prev };
  const patchT = (tpl: TemplateDto): TemplateDto =>
    tpl.id === p.id
      ? {
          ...tpl,
          last_run_status: p.lastRunStatus ?? tpl.last_run_status,
          is_running: p.isRunning ?? tpl.is_running,
          last_run_at:
            p.lastRunAt !== undefined && p.lastRunAt !== null
              ? p.lastRunAt
              : tpl.last_run_at,
        }
      : tpl;
  if (p.entityType === "template") {
    next.templates = (next.templates ?? []).map(patchT);
    next.standalone = (next.standalone ?? []).map(patchT);
    if (p.groupId && p.isRunning === true) {
      // Nested start: exactly one active among siblings.
      next.groups = (next.groups ?? []).map((g: TemplateGroupDto) =>
        g.id === p.groupId
          ? { ...g, is_running: true, active_template_id: p.id }
          : g,
      );
      const clearSiblings = (tpl: TemplateDto): TemplateDto => {
        const g = next.groups?.find((x) => x.id === p.groupId);
        if (!g?.template_ids?.includes(tpl.id)) return tpl;
        return { ...tpl, is_running: tpl.id === p.id };
      };
      next.templates = (next.templates ?? []).map(clearSiblings);
      next.standalone = (next.standalone ?? []).map(clearSiblings);
    } else if (p.isRunning === false) {
      // Nested/solo stop: clear running on this template only.
      // Do NOT null group.active_template_id here — mergeEphemeral `??` would
      // sticky-restore the previous id and keep warning on the first nested
      // (lesson 119). Next template start / group active overwrites the cursor.
      const stop = (tpl: TemplateDto): TemplateDto =>
        tpl.id === p.id ? { ...tpl, is_running: false } : tpl;
      next.templates = (next.templates ?? []).map(stop);
      next.standalone = (next.standalone ?? []).map(stop);
    }
  } else if (p.entityType === "group") {
    next.groups = (next.groups ?? []).map((g: TemplateGroupDto) =>
      g.id === p.id
        ? {
            ...g,
            last_run_status: p.lastRunStatus ?? g.last_run_status,
            is_running: p.isRunning ?? g.is_running,
            last_run_at:
              p.lastRunAt !== undefined && p.lastRunAt !== null
                ? p.lastRunAt
                : g.last_run_at,
            active_template_id:
              p.isRunning === false
                ? null
                : (p.activeTemplateId ?? g.active_template_id ?? null),
          }
        : g,
    );
    // Sync nested is_running from activeTemplateId.
    const g = next.groups?.find((x) => x.id === p.id);
    const activeId = g?.active_template_id;
    if (g?.is_running && activeId) {
      const mark = (tpl: TemplateDto): TemplateDto =>
        tpl.id === activeId
          ? { ...tpl, is_running: true }
          : g.template_ids?.includes(tpl.id)
            ? { ...tpl, is_running: false }
            : tpl;
      next.templates = (next.templates ?? []).map(mark);
      next.standalone = (next.standalone ?? []).map(mark);
    } else if (p.isRunning === false && g) {
      const clear = (tpl: TemplateDto): TemplateDto =>
        g.template_ids?.includes(tpl.id) ? { ...tpl, is_running: false } : tpl;
      next.templates = (next.templates ?? []).map(clear);
      next.standalone = (next.standalone ?? []).map(clear);
    }
  }
  return next;
}

/** scriptsList не зависит от market — один раз на сессию страницы. */
let scriptsListCache: Record<string, string> | null = null;
/** scriptsListGrouped per marketplace — reuse when opening editor. */
const scriptsGroupedCache = new Map<
  TemplatesMarket,
  { groups: ScriptGroupDto[] }
>();

export function emptySnap(market: TemplatesMarket): TemplatesSnapshot {
  return {
    marketplace: market,
    tabs: [tr("templates.defaultTab"), tr("templates.calculatorTab")],
    selected_tab: tr("templates.defaultTab"),
    default_tab: tr("templates.defaultTab"),
    templates: [],
    groups: [],
    standalone: [],
  };
}

export async function ensureScriptsMap(): Promise<Record<string, string>> {
  if (scriptsListCache) return scriptsListCache;
  const scriptList = await hostApi.scriptsList();
  const map: Record<string, string> = {};
  for (const [k, title] of scriptList) map[k] = title;
  scriptsListCache = map;
  return map;
}

export async function ensureScriptsGrouped(
  marketplace: TemplatesMarket,
): Promise<{ groups: ScriptGroupDto[] }> {
  const hit = scriptsGroupedCache.get(marketplace);
  if (hit) return hit;
  const grouped = await hostApi.scriptsListGrouped(marketplace);
  const value = { groups: grouped.groups ?? [] };
  scriptsGroupedCache.set(marketplace, value);
  return value;
}

export function clearScriptsGroupedCache() {
  scriptsGroupedCache.clear();
}
