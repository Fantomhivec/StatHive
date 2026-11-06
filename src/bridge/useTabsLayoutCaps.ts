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

import { useEffect, useState } from "react";
import {
  ensureTabsLayout,
  onTabsLayoutChanged,
  peekTabsLayout,
} from "./hostApi.layout";
import type { TabsLayoutResult } from "./hostApi.types";

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
  canAllTables: false,
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
