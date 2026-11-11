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
  type AccessPresetDto,
  type ScriptCatalogItem,
  type TeamConfigListItem,
  type TeamUserRow,
} from "../../bridge/hostApi";
import { pickNum, pickStr } from "../../bridge/pickField";
import { sheetsUrl } from "../../bridge/sheetsUrl";
import {
  clearUsersPageSnap as clearUsersPageSnapStore,
  getUsersPageSnap,
  setUsersPageSnap as setUsersPageSnapStore,
} from "../../bridge/snapCache";
import { clearTeamOauthSnap } from "./teamOauthTabCache";

export { sheetsUrl };

/** Лист команды сериализуется в camelCase. */
export function normalizeTeamRow(raw: TeamUserRow): TeamUserRow {
  const r = raw as unknown as Record<string, unknown>;
  const calc = pickStr(r, "calcConfigId", "calc_config_id");
  return {
    login: String(r.login ?? ""),
    display_name: pickStr(r, "displayName", "display_name"),
    access_level: pickStr(r, "accessLevel", "access_level"),
    google_email: pickStr(r, "googleEmail", "google_email"),
    row_index: pickNum(r, "rowIndex", "row_index"),
    calc_config_id: calc,
    calcConfigId: calc,
  };
}

export {
  DISCORD_COMMUNITY_NAME,
  DISCORD_INVITE_URL,
} from "../login/community";

/** Снимок вкладки между remount (lazy route). Sheets/SA — только при первом заходе или force. */
export type UsersPageSnap = {
  login: string;
  /** Слот команды — иначе SA/список «переезжают» между командами. */
  activeTeamId: string;
  team: TeamUserRow[];
  presets: AccessPresetDto[];
  catalog: ScriptCatalogItem[];
  calcConfigs: TeamConfigListItem[];
  workspaceId: string | null;
  workspaceTitle: string;
  saConfigured: boolean;
  saEmail: string | null;
  invitePreset: string;
  inviteCalcConfig: string;
  teamCatDefaultId: string;
};

export function peekUsersPageSnap(): UsersPageSnap | null {
  return getUsersPageSnap<UsersPageSnap>();
}

export function clearUsersPageSnap() {
  clearUsersPageSnapStore();
  // TeamOAuth tab cache (Sheets) — сброс при logout / force reload Users.
  clearTeamOauthSnap();
}

export function setUsersPageSnap(snap: UsersPageSnap | null) {
  setUsersPageSnapStore(snap);
}

const emptyMatrix = { all: false, keys: [] as string[] };

export const emptyPreset = (): AccessPresetDto => ({
  id: "",
  title: "",
  calculator: true,
  calcAccess: ["view", "settings", "tables", "kgt", "excel"],
  appAccess: [],
  scripts: emptyMatrix,
});

/** Загрузка dashboard + catalog + calc configs → snap (и кэш сессии). */
export async function fetchUsersPageSnap(
  loginKey: string,
  activeTeamId: string,
  force = false,
): Promise<UsersPageSnap> {
  const teamKey = activeTeamId.trim();
  const cached = peekUsersPageSnap();
  if (
    !force &&
    cached &&
    cached.login === loginKey &&
    cached.activeTeamId === teamKey &&
    loginKey
  ) {
    return cached;
  }
  if (force) {
    clearUsersPageSnap();
  }
  const [dash, c, teamCat] = await Promise.all([
    hostApi.usersDashboard(force),
    hostApi.accessScriptCatalog(),
    hostApi.wbCalculator.listTeamConfigs().catch(
      (): { defaultId: string; items: TeamConfigListItem[] } => ({
        defaultId: "",
        items: [],
      }),
    ),
  ]);
  const team = (dash.team ?? []).map(normalizeTeamRow);
  const presets = dash.presets ?? [];
  const cfgItems: TeamConfigListItem[] = teamCat.items ?? [];
  const workspace = (dash.workspaceId ?? dash.workspace_id)?.trim() || null;
  const workspaceTitle = String(
    dash.workspaceTitle ?? dash.workspace_title ?? "",
  ).trim();
  const saOk = !!(dash.serviceAccountConfigured ?? dash.service_account_configured);
  const saEm =
    (dash.serviceAccountEmail ?? dash.service_account_email)?.trim() || null;
  const snap: UsersPageSnap = {
    login: loginKey,
    activeTeamId: teamKey,
    team,
    presets,
    catalog: c,
    calcConfigs: cfgItems,
    workspaceId: workspace,
    workspaceTitle,
    saConfigured: saOk,
    saEmail: saEm,
    invitePreset: presets[0]?.id ?? "",
    inviteCalcConfig:
      cfgItems.find((x) => x.isDefault)?.id ??
      teamCat.defaultId ??
      cfgItems[0]?.id ??
      "",
    teamCatDefaultId: teamCat.defaultId ?? "",
  };
  setUsersPageSnap(snap);
  return snap;
}
