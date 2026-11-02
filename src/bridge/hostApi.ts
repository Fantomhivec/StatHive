import { invoke } from "@tauri-apps/api/core";
import type {
  EnvKeyRow,
  EnvKeysSnapshot,
  KeyCheckResult,
  LogLine,
  ScriptGroupDto,
  TeamConfigCatalogDto,
  UpdateInfo,
} from "./hostApi.types";
import { createAuthApi } from "./hostApi.auth";
import { hostApiTemplates } from "./hostApi.templates";
import {
  clearTabsLayoutCache,
  getTabsLayout,
  saveTabsLayout,
} from "./hostApi.layout";

export type {
  AccessMatrixDto,
  AccessPresetDto,
  CalcAccessDto,
  CalcAccessFlag,
  AuthSetupStatus,
  SwitchTeamResult,
  TeamIndexEntry,
  EnvKeyRow,
  EnvKeysSnapshot,
  KeyCheckLine,
  KeyCheckResult,
  LogLine,
  ScriptCatalogItem,
  ScriptGroupDto,
  SubjectsListFiltersDto,
  TabLayoutItem,
  TabsLayoutResult,
  TeamConfigCatalogDto,
  TeamConfigListItem,
  TeamUserRow,
  TemplateDto,
  TemplateField,
  TemplateGroupDto,
  TemplatesSnapshot,
  TimeTriggerDto,
  UpdateInfo,
  UserInfo,
} from "./hostApi.types";

export {
  clearAuthStatusCache,
  ensureAuthStatus,
  setAuthStatusCache,
} from "./hostApi.auth";

export {
  clearTabsLayoutCache,
  ensureTabsLayout,
  getTabsLayout,
  onTabsLayoutChanged,
  peekTabsLayout,
  refreshTabsLayout,
  saveTabsLayout,
  setTabsLayoutCache,
} from "./hostApi.layout";

let versionCached: string | null = null;
let versionInflight: Promise<string> | null = null;

export function ensureAppVersion(): Promise<string> {
  if (versionCached != null) return Promise.resolve(versionCached);
  if (!versionInflight) {
    versionInflight = hostApi
      .version()
      .then((v) => {
        versionCached = v;
        versionInflight = null;
        return v;
      })
      .catch((e) => {
        versionInflight = null;
        throw e;
      });
  }
  return versionInflight;
}

/** Тихая проверка обновлений (verbose=false). Кнопка «Проверить» — updatesCheck(true). */
let updatesQuietCached: UpdateInfo | undefined = undefined;
let updatesQuietInflight: Promise<UpdateInfo> | null = null;
/** After failure, do not re-invoke for a while (GitHub unauth rate limit). */
let updatesQuietFailUntilMs = 0;

export function ensureUpdatesCheckQuiet(): Promise<UpdateInfo> {
  if (updatesQuietCached !== undefined) return Promise.resolve(updatesQuietCached);
  if (Date.now() < updatesQuietFailUntilMs) {
    return Promise.reject(new Error("updates check cooldown"));
  }
  if (!updatesQuietInflight) {
    updatesQuietInflight = hostApi
      .updatesCheck(false)
      .then((u) => {
        updatesQuietCached = u;
        updatesQuietInflight = null;
        updatesQuietFailUntilMs = 0;
        return u;
      })
      .catch((e) => {
        updatesQuietInflight = null;
        updatesQuietFailUntilMs = Date.now() + 15 * 60 * 1000;
        throw e;
      });
  }
  return updatesQuietInflight;
}

export { getLocale, setLocale } from "./hostApi.i18n";
import { getLocale, setLocale } from "./hostApi.i18n";

const hostApiAuth = createAuthApi({ clearTabsLayoutCache });

export const hostApi = {
  version: () => invoke<string>("app_version"),
  getLocale,
  setLocale,
  logsGet: (knownRevision = 0) =>
    invoke<{ revision: number; unchanged: boolean; lines: LogLine[] }>("logs_get", {
      args: { knownRevision },
    }),
  logsClear: () => invoke<void>("logs_clear"),
  ...hostApiAuth,
  scriptsList: () => invoke<[string, string][]>("scripts_list"),
  scriptsListGrouped: (marketplace: "wb" | "ozon" = "wb") =>
    invoke<{ groups: ScriptGroupDto[] }>("scripts_list_grouped", {
      args: { marketplace },
    }),
  /** `verbose: true` — кнопка «Проверить» (логи процесса). Авто/профиль — без verbose (только ошибки). */
  updatesCheck: (verbose = false) =>
    invoke<UpdateInfo>("updates_check", { args: { verbose } }),
  updatesDownloadInstall: () =>
    invoke<{ success: boolean; message: string }>("updates_download_install"),
  /** Имена кабинетов без секретов (шаблоны при scripts+OAuth без keysCheck). */
  envListCabinetNames: () => invoke<EnvKeysSnapshot>("env_list_cabinet_names"),
  envSaveRows: (rows: EnvKeyRow[]) =>
    invoke<EnvKeysSnapshot>("env_save_rows", { args: { rows } }),
  envWbTokenInfo: (cabinet: string, marketplace: string, value: string) =>
    invoke<{ ok: boolean; title: string; message: string }>("env_wb_token_info", {
      args: { cabinet, marketplace, value },
    }),
  envCheckKeys: () => invoke<KeyCheckResult>("env_check_keys"),
  envCreateDefault: () => invoke<EnvKeysSnapshot>("env_create_default"),
  envSetPath: (path: string) =>
    invoke<{ path: string }>("env_set_path", { args: { path } }),
  envExportTo: (path: string) =>
    invoke<{ ok: boolean; path: string }>("env_export_to", { args: { path } }),
  envImportFrom: (path: string) =>
    invoke<EnvKeysSnapshot>("env_import_from", { args: { path } }),
  googleCheckConnection: (spreadsheetId?: string, auto?: boolean) =>
    invoke<{
      success: boolean;
      status: string;
      message: string;
      time: string;
      tone: string;
    }>("google_check_connection", {
      args: { spreadsheet_id: spreadsheetId ?? "", auto: !!auto },
    }),
  googleListSheets: (spreadsheetId: string) =>
    invoke<{ spreadsheet_id: string; title?: string; sheets: string[] }>("google_list_sheets", {
      args: { spreadsheet_id: spreadsheetId },
    }),
  spreadsheetNormalize: (value: string) =>
    invoke<{ spreadsheet_id: string; changed: boolean }>("spreadsheet_normalize", {
      args: { value },
    }),
  ...hostApiTemplates,
  getTabsLayout,
  saveTabsLayout,
  wbCalculator: {
    getSnapshot: (view?: "view" | "settings" | "tables" | "kgt" | "full") =>
      view && view !== "full"
        ? invoke<Record<string, unknown>>("wb_calculator_get_snapshot_for", {
            args: { view },
          })
        : invoke<Record<string, unknown>>("wb_calculator_get_snapshot"),
    saveSettings: (settings: Record<string, unknown>) =>
      invoke<Record<string, unknown>>("wb_calculator_save_settings", { settings }),
    setTriggers: (triggers: unknown[]) =>
      invoke<Record<string, unknown>>("wb_calculator_set_triggers", {
        args: { triggers },
      }),
    editTableLink: (key: string, url: string) =>
      invoke<Record<string, unknown>>("wb_calculator_edit_table_link", {
        args: { key, url },
      }),
    clearTableLinks: () => invoke<Record<string, unknown>>("wb_calculator_clear_table_links"),
    lookupCard: (article: string, marketplace?: string, force?: boolean) =>
      invoke<Record<string, unknown>>("wb_calculator_lookup_card", {
        args: { article, marketplace, force: !!force },
      }),
    /** Фоновый прогрев сводов/индекса в RAM (идемпотентно). */
    warmLookupCache: () =>
      invoke<{ ok: boolean; cached?: boolean; keys?: number; ms?: number }>(
        "wb_calculator_warm_lookup_cache",
      ),
    exportExcel: (path: string) =>
      invoke<{
        ok: boolean;
        cancelled?: boolean;
        path?: string;
        warnings?: string[];
        wbRows?: number;
        ozRows?: number;
      }>("wb_calculator_export_excel", { args: { path } }),
    openTable: (key: string) =>
      invoke<{ ok: boolean; url?: string }>("wb_calculator_open_table", {
        args: { key },
      }),
    appendProfitToken: (token: string, formulaField?: string) =>
      invoke<Record<string, unknown>>("wb_calculator_append_profit_token", {
        args: { token, formulaField },
      }),
    clearProfitFormula: (formulaField?: string) =>
      invoke<Record<string, unknown>>("wb_calculator_clear_profit_formula", {
        args: { formulaField },
      }),
    exportConfigFile: (path: string) =>
      invoke<{ ok: boolean; cancelled?: boolean; path?: string; json?: string }>(
        "wb_calculator_export_config_file",
        { args: { path } },
      ),
    importConfig: (json: string) =>
      invoke<Record<string, unknown>>("wb_calculator_import_config", { args: { json } }),
    importConfigFile: (path: string) =>
      invoke<{
        ok: boolean;
        cancelled?: boolean;
        path?: string;
        snapshot?: Record<string, unknown>;
      }>("wb_calculator_import_config_file", { args: { path } }),
    createOrUpdateAll: (marketplace?: string) =>
      invoke<{
        ok: boolean;
        message?: string;
        snapshot?: Record<string, unknown>;
      }>("wb_calculator_create_or_update_all", {
        args: { marketplace: marketplace ?? null },
      }),
    refreshTable: (key: string) =>
      invoke<{
        ok: boolean;
        message?: string;
        snapshot?: Record<string, unknown>;
      }>("wb_calculator_refresh_table", { args: { key } }),
    getRunLogs: () =>
      invoke<{ lines: string[]; text: string }>("wb_calculator_get_run_logs"),
    clearRunLogs: () =>
      invoke<{ ok: boolean }>("wb_calculator_clear_run_logs"),
    listTeamConfigs: async () => {
      const raw = await invoke<{
        defaultId?: string;
        default_id?: string;
        items?: Array<Record<string, unknown>>;
      }>("wb_calculator_list_team_configs");
      const itemsRaw = Array.isArray(raw.items) ? raw.items : [];
      return {
        defaultId: String(raw.defaultId ?? raw.default_id ?? ""),
        items: itemsRaw.map((x) => ({
          id: String(x.id ?? ""),
          name: String(x.name ?? ""),
          revision: String(x.revision ?? ""),
          isDefault: Boolean(x.isDefault ?? x.is_default),
        })),
      } satisfies TeamConfigCatalogDto;
    },
    publishTeamConfig: (name: string, makeDefault?: boolean) =>
      invoke<{
        ok: boolean;
        id?: string;
        name?: string;
        revision?: string;
      }>("wb_calculator_publish_team_config", {
        args: { name, makeDefault: !!makeDefault },
      }),
    setDefaultTeamConfig: (id: string) =>
      invoke<{ ok: boolean }>("wb_calculator_set_default_team_config", {
        args: { id },
      }),
    deleteTeamConfig: (id: string) =>
      invoke<{ ok: boolean }>("wb_calculator_delete_team_config", {
        args: { id },
      }),
    pullTeamConfig: (id?: string) =>
      invoke<{
        ok: boolean;
        pulled?: boolean;
        id?: string;
        name?: string;
        revision?: string;
      }>("wb_calculator_pull_team_config", {
        args: { id: id ?? "" },
      }),
  },
};

export {
  defaultExcelFileName,
  hostDialogs,
  hostWindow,
  openUrl,
} from "./hostApiChrome";

export { onEvent } from "./hostApi.events";
