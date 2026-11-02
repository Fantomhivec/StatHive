/** Tabs layout cache + IPC — thin shell imports (no wbCalculator). */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TabLayoutItem, TabsLayoutResult } from "./hostApi.types";

function normalizeTabItem(raw: Record<string, unknown>): TabLayoutItem {
  return {
    tabKey: String(raw.tabKey ?? raw.tab_key ?? ""),
    title: String(raw.title ?? ""),
    isVisible: Boolean(raw.isVisible ?? raw.is_visible ?? true),
    order: Number(raw.order ?? 0),
  };
}

function normalizeCalcAccess(raw: unknown): TabsLayoutResult["calcAccess"] {
  if (!raw || typeof raw !== "object") {
    return undefined;
  }
  const o = raw as Record<string, unknown>;
  return {
    view: Boolean(o.view),
    settings: Boolean(o.settings),
    tables: Boolean(o.tables),
    kgt: Boolean(o.kgt),
    excel: Boolean(o.excel),
  };
}

export function normalizeTabsLayout(res: Record<string, unknown>): TabsLayoutResult {
  const itemsRaw = Array.isArray(res.items) ? res.items : [];
  const canTemplatesRaw = res.canTemplates ?? res.can_templates;
  const hasEnvKeys = Boolean(res.hasEnvKeys ?? res.has_env_keys);
  return {
    login: String(res.login ?? ""),
    calculatorMode: Boolean(res.calculatorMode ?? res.calculator_mode),
    canCalculatorMode: Boolean(res.canCalculatorMode ?? res.can_calculator_mode),
    forceCalculatorMode: Boolean(
      res.forceCalculatorMode ?? res.force_calculator_mode,
    ),
    // Fail-closed: без ответа бэкенда вкладки с ключами скрыты.
    canTemplates: canTemplatesRaw === undefined ? false : Boolean(canTemplatesRaw),
    hasEnvKeys,
    canLogs: Boolean(res.canLogs ?? res.can_logs ?? hasEnvKeys),
    canTablesTab: Boolean(res.canTablesTab ?? res.can_tables_tab),
    calcAccess: normalizeCalcAccess(res.calcAccess ?? res.calc_access),
    canKeysCheck: Boolean(res.canKeysCheck ?? res.can_keys_check),
    canUsersTab: Boolean(res.canUsersTab ?? res.can_users_tab),
    hasOauth: Boolean(res.hasOauth ?? res.has_oauth),
    canCheckGoogle: Boolean(res.canCheckGoogle ?? res.can_check_google),
    templatesExcelOnly: Boolean(res.templatesExcelOnly ?? res.templates_excel_only),
    tablesCanMutate: Boolean(res.tablesCanMutate ?? res.tables_can_mutate),
    isMainAdmin: Boolean(res.isMainAdmin ?? res.is_main_admin),
    items: itemsRaw.map((x) => normalizeTabItem(x as Record<string, unknown>)),
  };
}

/** Module-level cache — Shell + TabsSettings без повторного IPC. */
let tabsLayoutCached: TabsLayoutResult | null = null;
let tabsLayoutInflight: Promise<TabsLayoutResult> | null = null;
/** Инвалидация: stale tabs_layout_get не затирает свежий кэш / событие. */
let layoutGen = 0;

export function peekLayoutGen(): number {
  return layoutGen;
}

export function setTabsLayoutCache(res: TabsLayoutResult) {
  layoutGen += 1;
  tabsLayoutCached = res;
  tabsLayoutInflight = null;
}

export function clearTabsLayoutCache() {
  layoutGen += 1;
  tabsLayoutCached = null;
  tabsLayoutInflight = null;
}

/**
 * Сброс кэша + свежий `tabs_layout_get` + локальный `ur.tabsLayout`
 * (Shell / useTabsLayoutCaps после auth.sessionUpdated).
 */
export function refreshTabsLayout(): Promise<TabsLayoutResult> {
  clearTabsLayoutCache();
  const genAtStart = layoutGen;
  return ensureTabsLayoutForGen(genAtStart).then((res) => {
    if (layoutGen !== genAtStart) {
      // Более новый clear/event — не шлём stale local notify.
      return tabsLayoutCached ?? res;
    }
    notifyTabsLayoutLocal(res);
    return res;
  });
}

/** Синхронный снимок кэша layout (после ensureTabsLayout / события). */
export function peekTabsLayout(): TabsLayoutResult | null {
  return tabsLayoutCached;
}

export function getTabsLayout(): Promise<TabsLayoutResult> {
  return invoke<Record<string, unknown>>("tabs_layout_get").then(normalizeTabsLayout);
}

function ensureTabsLayoutForGen(fetchGen: number): Promise<TabsLayoutResult> {
  if (tabsLayoutCached) {
    return Promise.resolve(tabsLayoutCached);
  }
  if (!tabsLayoutInflight) {
    tabsLayoutInflight = getTabsLayout()
      .then((res) => {
        tabsLayoutInflight = null;
        // Событие / другой refresh уже положили более новый кэш.
        if (layoutGen !== fetchGen) {
          return tabsLayoutCached ?? res;
        }
        tabsLayoutCached = res;
        return res;
      })
      .catch((e) => {
        tabsLayoutInflight = null;
        throw e;
      });
  }
  return tabsLayoutInflight;
}

export function ensureTabsLayout(): Promise<TabsLayoutResult> {
  if (tabsLayoutCached) return Promise.resolve(tabsLayoutCached);
  return ensureTabsLayoutForGen(layoutGen);
}

export function saveTabsLayout(
  items: TabLayoutItem[],
  calculatorMode?: boolean,
): Promise<TabsLayoutResult> {
  const args: {
    items: { tabKey: string; isVisible: boolean; order: number }[];
    calculatorMode?: boolean;
  } = {
    items: items.map((it) => ({
      tabKey: it.tabKey,
      isVisible: it.isVisible,
      order: it.order,
    })),
  };
  if (calculatorMode !== undefined) {
    args.calculatorMode = calculatorMode;
  }
  return invoke<Record<string, unknown>>("tabs_layout_save", { args }).then((raw) => {
    const res = normalizeTabsLayout(raw);
    setTabsLayoutCache(res);
    // Синхронно для Shell до navigate (Tauri-событие может прийти позже).
    notifyTabsLayoutLocal(res);
    return res;
  });
}

const TABS_LAYOUT_LOCAL = "ur.tabsLayout";

/** Локальный sync после saveTabsLayout — до прихода Tauri-события. */
function notifyTabsLayoutLocal(result: TabsLayoutResult) {
  window.dispatchEvent(new CustomEvent(TABS_LAYOUT_LOCAL, { detail: result }));
}

/** Shell: локальный custom event + host `tabsLayout.changed`. */
export function onTabsLayoutChanged(handler: (result: TabsLayoutResult) => void): () => void {
  const onLocal = (e: Event) => {
    handler((e as CustomEvent<TabsLayoutResult>).detail);
  };
  window.addEventListener(TABS_LAYOUT_LOCAL, onLocal);
  let un: UnlistenFn | undefined;
  let cancelled = false;
  void listen("tabsLayout.changed", (e) => {
    if (cancelled) return;
    const res = normalizeTabsLayout((e.payload ?? {}) as Record<string, unknown>);
    setTabsLayoutCache(res);
    handler(res);
  }).then((fn) => {
    if (cancelled) {
      fn();
      return;
    }
    un = fn;
  });
  return () => {
    cancelled = true;
    window.removeEventListener(TABS_LAYOUT_LOCAL, onLocal);
    un?.();
  };
}
