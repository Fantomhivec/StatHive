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

export type { TemplatesMarket };
export { clearTemplatesSnapCache, getTemplatesSnapCache, setTemplatesSnapCache };

export type TemplatesRunProgress = {
  entityType: string;
  id: string;
  lastRunStatus?: string;
  isRunning?: boolean;
  lastRunAt?: string | null;
  activeTemplateId?: string | null;
  groupId?: string | null;
};

type SnapListener = (snap: TemplatesSnapshot) => void;
const snapListeners = new Set<SnapListener>();
let runProgressBridgeStarted = false;

/**
 * Keep templates snap cache updated even when TemplatesPage is unmounted (Logs tab).
 * Call once from App / Templates mount.
 */
export function ensureTemplatesRunProgressBridge() {
  if (runProgressBridgeStarted) return;
  runProgressBridgeStarted = true;
  onEvent<TemplatesRunProgress>("templates.runProgress", (p) => {
    for (const market of ["wb", "ozon"] as TemplatesMarket[]) {
      const prev = getTemplatesSnapCache(market);
      if (!prev) continue;
      const next = applyTemplatesRunProgress(prev, p);
      setTemplatesSnapCache(market, next);
      for (const l of snapListeners) l(next);
    }
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
 * Keep ephemeral run flags when disk snapshot / enrich would wipe them.
 * Group members: exactly one active (active_template_id) — never sticky-OR
 * finished nested templates (that caused two «выполняется» at once).
 */
export function mergeEphemeralRunFlags(
  prev: TemplatesSnapshot | null | undefined,
  next: TemplatesSnapshot,
): TemplatesSnapshot {
  if (!prev) return next;
  const prevTpl = new Map(
    [...(prev.templates ?? []), ...(prev.standalone ?? [])].map((t) => [t.id, t]),
  );
  const prevGroup = new Map((prev.groups ?? []).map((g) => [g.id, g]));

  const groups = (next.groups ?? []).map((g) => {
    const p = prevGroup.get(g.id);
    if (!p) return g;
    const is_running = !!(g.is_running || p.is_running);
    // Prefer next active (progress); keep prev only while group still running.
    const active_template_id = is_running
      ? (g.active_template_id ?? p.active_template_id ?? null)
      : null;
    if (!is_running && !active_template_id) return g;
    return {
      ...g,
      is_running,
      active_template_id,
    };
  });

  /** Nested tpl id → active id of its running group (null = group busy, none selected yet). */
  const nestedActive = new Map<string, string | null>();
  for (const g of groups) {
    if (!g.is_running) continue;
    const active = g.active_template_id ?? null;
    for (const tid of g.template_ids ?? []) {
      nestedActive.set(tid, active);
    }
  }

  const mergeT = (t: TemplateDto): TemplateDto => {
    if (nestedActive.has(t.id)) {
      const active = nestedActive.get(t.id);
      return {
        ...t,
        is_running: !!active && active === t.id,
      };
    }
    // Solo run: preserve prev if disk/enrich wiped the flag mid-flight.
    if (t.is_running) return t;
    const p = prevTpl.get(t.id);
    if (p?.is_running) return { ...t, is_running: true };
    return t;
  };

  return {
    ...next,
    groups,
    templates: (next.templates ?? []).map(mergeT),
    standalone: (next.standalone ?? []).map(mergeT),
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
    // Nested progress with groupId: exactly one active among siblings.
    if (p.groupId && p.isRunning) {
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
    } else if (!p.groupId && p.isRunning === false) {
      // Nested runs emit template stop without groupId — drop sticky flag here;
      // mergeEphemeral will re-apply only if group.active still points at this id.
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
