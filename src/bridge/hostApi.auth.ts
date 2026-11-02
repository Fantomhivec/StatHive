import { invoke } from "@tauri-apps/api/core";
import type {
  AccessPresetDto,
  AuthSetupStatus,
  ScriptCatalogItem,
  SwitchTeamResult,
  TeamIndexEntry,
  TeamUserRow,
  UserInfo,
} from "./hostApi.types";

/** Session auth — RequireAuth / title bar / profile share one IPC. */
let authCached: UserInfo | null | undefined = undefined;
let authInflight: Promise<UserInfo | null> | null = null;
/** Выход увеличивает счётчик: поздний refresh профиля не возвращает сессию. */
let authEpoch = 0;

export function setAuthStatusCache(user: UserInfo | null) {
  authCached = user;
  authInflight = null;
}

export function clearAuthStatusCache() {
  authEpoch += 1;
  authCached = undefined;
  authInflight = null;
}

export function ensureAuthStatus(): Promise<UserInfo | null> {
  if (authCached !== undefined) return Promise.resolve(authCached);
  if (!authInflight) {
    authInflight = invoke<UserInfo | null>("auth_status")
      .then((u) => {
        authCached = u;
        authInflight = null;
        return u;
      })
      .catch((e) => {
        authInflight = null;
        throw e;
      });
  }
  return authInflight;
}

export type AuthApiDeps = {
  clearTabsLayoutCache: () => void;
};

/** Auth / team / access-preset invoke helpers for hostApi barrel. */
export function createAuthApi(deps: AuthApiDeps) {
  const { clearTabsLayoutCache } = deps;
  return {
    authStatus: () => invoke<UserInfo | null>("auth_status"),
    authSetupStatus: () => invoke<AuthSetupStatus>("auth_setup_status"),
    saveOauthClient: (json: string) =>
      invoke<void>("auth_save_oauth_client", { args: { json } }),
    saveServiceAccount: (json: string) =>
      invoke<string>("auth_save_service_account", { args: { json } }),
    createWorkspaceServiceAccount: () =>
      invoke<string>("auth_create_workspace_service_account"),
    replaceWorkspaceServiceAccount: () =>
      invoke<string>("auth_replace_workspace_service_account"),
    connectGoogle: () => invoke<string>("auth_connect_google"),
    switchGoogleAccount: () => invoke<string>("auth_switch_google_account"),
    registerAdmin: (args: {
      login: string;
      password: string;
      display_name: string;
      workspace_title?: string;
    }) =>
      invoke<UserInfo>("auth_register_admin", {
        args: {
          login: args.login,
          password: args.password,
          display_name: args.display_name,
          workspace_title: args.workspace_title ?? "",
        },
      }).then((u) => {
        clearTabsLayoutCache();
        setAuthStatusCache(u);
        return u;
      }),
    joinWorkspace: (spreadsheetId: string, title = "") =>
      invoke<void>("auth_join_workspace", {
        args: { spreadsheet_id: spreadsheetId, title },
      }),
    renameWorkspace: (title: string) =>
      invoke<string>("auth_rename_workspace", { args: { title } }),
    exportTeamInvite: (passphrase: string) =>
      invoke<string>("auth_export_team_invite", { args: { passphrase } }),
    applyTeamInvite: (code: string, passphrase: string) =>
      invoke<void>("auth_apply_team_invite", { args: { code, passphrase } }).then(() => {
        clearTabsLayoutCache();
      }),
    teamOauthGet: () =>
      invoke<{
        hasClient: boolean;
        clientIdHint?: string | null;
        grants: {
          login: string;
          displayName: string;
          allowed: boolean;
          lockedOn: boolean;
        }[];
      }>("auth_team_oauth_get"),
    teamOauthPublish: () => invoke<void>("auth_team_oauth_publish"),
    teamOauthSetGrants: (allowedLogins: string[]) =>
      invoke<void>("auth_team_oauth_set_grants", {
        args: { allowedLogins },
      }),
    listTeams: () => invoke<TeamIndexEntry[]>("auth_list_teams"),
    beginNewTeam: () => invoke<string>("auth_begin_new_team"),
    setActiveTeam: (id: string) =>
      invoke<void>("auth_set_active_team", { args: { id } }).then(() => {
        clearAuthStatusCache();
        clearTabsLayoutCache();
      }),
    switchTeam: (id: string) =>
      invoke<SwitchTeamResult>("auth_switch_team", { args: { id } }).then((r) => {
        clearTabsLayoutCache();
        if (r.user) setAuthStatusCache(r.user);
        else clearAuthStatusCache();
        return r;
      }),
    inviteUser: (args: {
      login: string;
      password: string;
      display_name: string;
      google_email: string;
      preset_id: string;
      calc_config_id?: string;
    }) =>
      invoke<TeamUserRow>("auth_invite_user", {
        args: {
          login: args.login,
          password: args.password,
          displayName: args.display_name,
          googleEmail: args.google_email,
          presetId: args.preset_id,
          calcConfigId: args.calc_config_id ?? "",
        },
      }),
    setUserCalcConfig: (rowIndex: number, calcConfigId: string) =>
      invoke<void>("auth_set_user_calc_config", {
        args: { rowIndex, calcConfigId },
      }),
    listTeam: () => invoke<TeamUserRow[]>("auth_list_team"),
    /** force=true — сброс кэша и чтение Sheets (кнопка «Обновить»). */
    usersDashboard: (force = false) =>
      invoke<{
        team: TeamUserRow[];
        presets: AccessPresetDto[];
        workspaceId?: string | null;
        workspace_id?: string | null;
        workspaceTitle?: string;
        workspace_title?: string;
        serviceAccountConfigured?: boolean;
        service_account_configured?: boolean;
        serviceAccountEmail?: string | null;
        service_account_email?: string | null;
      }>("auth_users_dashboard", { args: { force } }),
    invalidateUsersCache: () => invoke<void>("access_invalidate_users_cache"),
    accessListPresets: () => invoke<AccessPresetDto[]>("access_list_presets"),
    accessSavePreset: (preset: AccessPresetDto) =>
      invoke<AccessPresetDto>("access_save_preset", {
        args: {
          id: preset.id,
          title: preset.title,
          calculator: preset.calculator,
          calcAccess: preset.calcAccess ?? [],
          appAccess: preset.appAccess ?? [],
          scripts: preset.scripts,
        },
      }),
    accessDeletePreset: (presetId: string) =>
      invoke<void>("access_delete_preset", { args: { presetId } }),
    accessSetUserPreset: (rowIndex: number, presetId: string) =>
      invoke<void>("access_set_user_preset", {
        args: { rowIndex, presetId },
      }),
    accessScriptCatalog: () =>
      invoke<ScriptCatalogItem[]>("access_script_catalog"),
    deleteUser: (rowIndex: number) =>
      invoke<void>("auth_delete_user", { args: { rowIndex } }),
    connectMemberOauth: () => invoke<UserInfo>("auth_connect_member_oauth"),
    /** Перечитать профиль из листа Пользователи. */
    refreshProfile: async () => {
      const epoch = authEpoch;
      const u = await invoke<UserInfo>("auth_refresh_profile");
      if (epoch !== authEpoch) return u;
      setAuthStatusCache(u);
      return u;
    },
    login: async (login: string, password: string, googleEmail?: string) => {
      const u = await invoke<UserInfo>("auth_login", {
        login,
        password,
        googleEmail: googleEmail ?? "",
      });
      clearTabsLayoutCache();
      setAuthStatusCache(u);
      return u;
    },
    logout: async () => {
      await invoke<void>("auth_logout");
      clearAuthStatusCache();
      clearTabsLayoutCache();
    },
    changePassword: (
      currentPassword: string,
      newPassword: string,
      confirmPassword: string,
    ) =>
      invoke<{ success: boolean; error?: string }>("auth_change_password", {
        args: {
          current_password: currentPassword,
          new_password: newPassword,
          confirm_password: confirmPassword,
        },
      }),
  };
}

export type HostApiAuth = ReturnType<typeof createAuthApi>;
