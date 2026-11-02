export type UserInfo = {
  login: string;
  display_name: string;
  is_authorized: boolean;
  access_level?: string;
  row_index?: number;
  session_id?: string | null;
  birth_date?: string;
  last_login?: string;
  google_email?: string;
  has_oauth?: boolean;
};

export type TeamIndexEntry = {
  id: string;
  title?: string;
  joinedByCode?: boolean;
  joined_by_code?: boolean;
  lastRole?: string;
  last_role?: string;
};

export type AuthSetupStatus = {
  oauth_configured: boolean;
  oauth_client_id_hint?: string | null;
  oauthClientIdHint?: string | null;
  google_connected: boolean;
  google_email?: string | null;
  workspace_configured: boolean;
  workspace_id?: string | null;
  workspace_title: string;
  can_register_admin: boolean;
  service_account_configured?: boolean;
  service_account_email?: string | null;
  joined_by_code?: boolean;
  teams?: TeamIndexEntry[];
  active_team_id?: string | null;
  activeTeamId?: string | null;
};

export type SwitchTeamResult = {
  status: "ok" | "needsLogin" | string;
  user?: UserInfo | null;
};

export type TeamUserRow = {
  login: string;
  display_name: string;
  access_level: string;
  google_email: string;
  row_index: number;
  calcConfigId?: string;
  calc_config_id?: string;
};

export type TeamConfigListItem = {
  id: string;
  name: string;
  revision: string;
  isDefault: boolean;
};

export type TeamConfigCatalogDto = {
  defaultId: string;
  items: TeamConfigListItem[];
};

export type AccessMatrixDto = {
  all: boolean;
  keys: string[];
};

export type CalcAccessFlag = "view" | "settings" | "tables" | "kgt" | "excel";

export type CalcAccessDto = {
  view: boolean;
  settings: boolean;
  tables: boolean;
  kgt: boolean;
  excel: boolean;
};

export type AppAccessFlag = "keysCheck" | "users";

export type AccessPresetDto = {
  id: string;
  title: string;
  calculator: boolean;
  /** Ключи CalcAccess: view|settings|tables|kgt|excel */
  calcAccess?: string[];
  /** keysCheck | users */
  appAccess?: string[];
  scripts: AccessMatrixDto;
};

export type ScriptCatalogItem = {
  key: string;
  title: string;
  memberOk?: boolean;
};

export type LogLine = {
  ts: string;
  level: string;
  message: string;
};

export type UpdateInfo = {
  version: string;
  download_url: string;
  body: string;
  is_newer: boolean;
  current_version?: string;
  latest_version?: string;
  has_update?: boolean;
  status?: string;
  releases_text?: string;
  asset_digest?: string;
};

export type EnvKeyRow = {
  id: string;
  cabinet: string;
  marketplace: string;
  expires_at: string;
  env_key: string;
  value: string;
  ozon_seller_id: string;
};

export type EnvKeysSnapshot = {
  path: string;
  rows: EnvKeyRow[];
};

export type KeyCheckLine = {
  index: number;
  text: string;
  tone: string;
  cabinet: string;
  marketplace: string;
};

export type KeyCheckResult = {
  path: string;
  count: number;
  wb_count: number;
  ozon_count: number;
  lines: KeyCheckLine[];
  warnings: string[];
  /** false если файла .env нет по path */
  exists?: boolean;
};

export type DestinationConfig = {
  spreadsheet_id: string;
  sheet_name: string;
};

export type TemplateField = {
  name: string;
  title: string;
  selected: boolean;
};

export type TimeTriggerDto = {
  id: string;
  template_id?: string;
  type?: string;
  trigger_type?: string;
  is_enabled?: boolean;
  scheduled_date_time?: string | null;
  daily_time?: string | null;
  weekly_day?: string | null;
  weekly_time?: string | null;
  interval_minutes?: number | null;
  created_at?: string;
  last_executed_at?: string | null;
  next_execution_at?: string | null;
  last_execution_status?: string | null;
  type_display_name?: string;
  settings_display_name?: string;
};

export type SyncSettingsDto = {
  enable_change_sync?: boolean;
  key_field_1?: string;
  key_field_2?: string;
  do_not_delete_old_rows?: boolean;
};

export type SubjectsListFiltersDto = {
  locale?: string;
  name?: string;
  limit?: number | null;
  offset?: number | null;
  parent_id?: number | null;
};

export type TemplateDto = {
  id: string;
  name: string;
  script_key: string;
  spreadsheet_id: string;
  sheet_name: string;
  /** ExportToExcel: писать в .xlsx вместо Google Sheets. */
  export_to_excel?: boolean;
  /** Кэш названия документа Google Sheets. */
  spreadsheet_title?: string;
  /** Второй источник (Массив / отчёт Ozon / …). */
  second_spreadsheet_id?: string;
  second_sheet_name?: string;
  second_spreadsheet_title?: string;
  /** Ozon: `sku` | `offer_id` | `product_id`. */
  id_mode?: string;
  /** Устаревший alias → sync с offer/product/sku range. */
  id_source_range?: string;
  /** Унифицированные диапазоны источников. */
  cabinets_source_range?: string;
  nm_id_source_range?: string;
  /** chrtId (`wb_stocks`). */
  stocks_skus_source_range?: string;
  /** ID РК (`wb_adv_campaigns_stats_new`). */
  id_adv_campaign_source_range?: string;
  offer_id_source_range?: string;
  product_id_source_range?: string;
  sku_source_range?: string;
  content_media_links_source_range?: string;
  subject_id_source_range?: string;
  subject_name_source_range?: string;
  use_custom_subject_ids?: boolean;
  cards_update_dimensions_mode?: boolean;
  cards_update_vendor_code_mode?: boolean;
  cards_update_vat_mode?: boolean;
  cards_update_brand_mode?: boolean;
  cards_update_length_range?: string;
  cards_update_width_range?: string;
  cards_update_height_range?: string;
  cards_update_vendor_code_range?: string;
  cards_update_vat_range?: string;
  cards_update_brand_range?: string;
  prices_price_range?: string;
  prices_discount_range?: string;
  prices_exclude_range?: string;
  prices_raise_percent?: string;
  price_set_price_range?: string;
  price_set_discount_range?: string;
  prices_history_task_upload_id_range?: string;
  paid_storage_status_task_id_range?: string;
  paid_storage_status_output_range?: string;
  paid_storage_download_task_id_range?: string;
  symbols_check_range?: string;
  cells_count_count_filled?: boolean;
  /** `promotions`: allPromo — все акции. */
  promotions_all_promo?: boolean;
  /** `daily` | `weekly` for finance reports. */
  report_realization_v5_period?: string;
  subjects_filters?: SubjectsListFiltersDto;
  date_from?: string;
  date_to?: string;
  /** Пресет: `1week` / `2weeks` / `4week` / … */
  date_template_type?: string;
  cabinets: string[];
  /** Полный список — только `templates_get` / save; в snapshot карточек может отсутствовать. */
  selected_fields?: TemplateField[];
  sync_settings?: SyncSettingsDto;
  /** Сортировка перед записью (имя поля скрипта). */
  write_sort_field_name?: string;
  /** `asc` | `desc`. */
  write_sort_direction?: string;
  /**
   * true = очищать только колонки записываемых данных
   * (справа пользовательские формулы не трогаем). По умолчанию false (весь лист).
   */
  clear_only_written_range?: boolean;
  marketplace?: string;
  tab_name?: string;
  last_run_status?: string;
  last_run_at?: string;
  /** Эфемерный флаг с события `templates.runProgress`. */
  is_running?: boolean;
  time_triggers?: TimeTriggerDto[];
  created_at?: string;
  updated_at?: string;
};

export type TemplateGroupDto = {
  id: string;
  name: string;
  marketplace: string;
  tab_name: string;
  template_ids: string[];
  is_expanded: boolean;
  last_run_status?: string;
  last_run_at?: string;
  is_running?: boolean;
  time_triggers?: TimeTriggerDto[];
};

export type ScriptGroupDto = {
  name: string;
  scripts: { key: string; title: string; api_method: string }[];
};

export type TemplatesSnapshot = {
  marketplace: string;
  tabs: string[];
  selected_tab: string;
  default_tab: string;
  templates: TemplateDto[];
  groups: TemplateGroupDto[];
  standalone: TemplateDto[];
};

export type TabLayoutItem = {
  tabKey: string;
  title: string;
  isVisible: boolean;
  order: number;
};

export type TabsLayoutResult = {
  login: string;
  calculatorMode?: boolean;
  canCalculatorMode?: boolean;
  /** Нет скриптов + есть калькулятор → режим включён и его нельзя выключить. */
  forceCalculatorMode?: boolean;
  /** false — скрыть вкладку «Шаблоны» (пресет / нет действующего ключа .env). */
  canTemplates?: boolean;
  /** false — нет действующего ключа в .env (Шаблоны / Логи / Таблицы). */
  hasEnvKeys?: boolean;
  /** Вкладка «Логи» — только при hasEnvKeys. */
  canLogs?: boolean;
  /** Вкладка «Таблицы» в режиме калькулятора — tables + hasEnvKeys. */
  canTablesTab?: boolean;
  calcAccess?: CalcAccessDto;
  canKeysCheck?: boolean;
  canUsersTab?: boolean;
  hasOauth?: boolean;
  /** Свой Desktop client или грант TeamOAuth — кнопка «Проверить подключение». */
  canCheckGoogle?: boolean;
  templatesExcelOnly?: boolean;
  tablesCanMutate?: boolean;
  isMainAdmin?: boolean;
  items: TabLayoutItem[];
};

export type ExcelSheetPreview = {
  path: string;
  sheet_name: string;
  sheet_names: string[];
  headers: string[];
  rows: string[][];
  row_count: number;
};
