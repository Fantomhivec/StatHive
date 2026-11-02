import { useEffect, useState } from "react";
import {
  ensureTabsLayout,
  onTabsLayoutChanged,
  peekTabsLayout,
} from "../bridge/hostApi.layout";
import type { TabsLayoutResult } from "../bridge/hostApi.types";

/** Fail-closed caps пока layout не пришёл. */
export const FAIL_CLOSED_TABS_LAYOUT: TabsLayoutResult = {
  login: "",
  items: [],
  calculatorMode: false,
  canTemplates: false,
  hasEnvKeys: false,
  canLogs: false,
  canTablesTab: false,
  canKeysCheck: false,
  canUsersTab: false,
  hasOauth: false,
  canCheckGoogle: false,
  templatesExcelOnly: true,
  tablesCanMutate: false,
  isMainAdmin: false,
  calcAccess: {
    view: false,
    settings: false,
    tables: false,
    kgt: false,
    excel: false,
  },
};

/**
 * Caps из tabs layout: синхронный peek + подписка.
 * Ensure только если Shell ещё не прогрел кэш (peek === null).
 */
export function useTabsLayoutCaps(): TabsLayoutResult {
  const [caps, setCaps] = useState<TabsLayoutResult>(
    () => peekTabsLayout() ?? FAIL_CLOSED_TABS_LAYOUT,
  );

  useEffect(() => {
    const apply = (res: TabsLayoutResult) => setCaps(res);
    if (peekTabsLayout() == null) {
      void ensureTabsLayout()
        .then(apply)
        .catch(() => setCaps(FAIL_CLOSED_TABS_LAYOUT));
    }
    return onTabsLayoutChanged(apply);
  }, []);

  return caps;
}
