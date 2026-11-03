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
};

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
    // Groups hold template_ids only — no clone of every group on template progress.
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
          }
        : g,
    );
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
