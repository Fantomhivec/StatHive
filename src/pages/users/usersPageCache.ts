import {
  hostApi,
  type AccessPresetDto,
  type ScriptCatalogItem,
  type TeamConfigListItem,
  type TeamUserRow,
} from "../../bridge/hostApi";

/** Лист команды сериализуется в camelCase. */
export function normalizeTeamRow(raw: TeamUserRow): TeamUserRow {
  const r = raw as TeamUserRow & {
    displayName?: string;
    accessLevel?: string;
    googleEmail?: string;
    rowIndex?: number;
    calcConfigId?: string;
  };
  const calc = String(r.calcConfigId ?? r.calc_config_id ?? "").trim();
  return {
    login: r.login,
    display_name: String(r.display_name ?? r.displayName ?? "").trim(),
    access_level: String(r.access_level ?? r.accessLevel ?? "").trim(),
    google_email: String(r.google_email ?? r.googleEmail ?? "").trim(),
    row_index: Number(r.row_index ?? r.rowIndex ?? 0),
    calc_config_id: calc,
    calcConfigId: calc,
  };
}

export function sheetsUrl(spreadsheetId: string) {
  return `https://docs.google.com/spreadsheets/d/${encodeURIComponent(spreadsheetId.trim())}`;
}

export {
  DISCORD_COMMUNITY_NAME,
  DISCORD_INVITE_URL,
} from "../../shared/community";

/** Снимок вкладки между remount (lazy route). Sheets/SA — только при первом заходе или force. */
export type UsersPageSnap = {
  login: string;
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

export let usersPageSnap: UsersPageSnap | null = null;

export function clearUsersPageSnap() {
  usersPageSnap = null;
}

export function setUsersPageSnap(snap: UsersPageSnap | null) {
  usersPageSnap = snap;
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
  force = false,
): Promise<UsersPageSnap> {
  if (
    !force &&
    usersPageSnap &&
    usersPageSnap.login === loginKey &&
    loginKey
  ) {
    return usersPageSnap;
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
