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

import type { TemplatesSnapshot } from "./hostApi.types";

/** Keyed snap store между remount; сбрасывать при смене команды. */
function createKeyedSnapCache<K extends string>() {
  const store: Partial<Record<K, unknown>> = {};
  return {
    clear() {
      for (const k of Object.keys(store) as K[]) {
        delete store[k];
      }
    },
    get<T = unknown>(key: K): T | undefined {
      return store[key] as T | undefined;
    },
    set(key: K, snap: unknown | null) {
      if (snap) store[key] = snap;
      else delete store[key];
    },
  };
}

/** Single-slot snap (Users tab). */
function createSlotSnapCache() {
  let slot: unknown | null = null;
  return {
    clear() {
      slot = null;
    },
    get<T = unknown>(): T | null {
      return slot as T | null;
    },
    set(snap: unknown | null) {
      slot = snap;
    },
  };
}

export type CalcSnapshotView = "view" | "settings" | "tables" | "kgt" | "full";
export type TemplatesMarket = "wb" | "ozon";

const calcCache = createKeyedSnapCache<CalcSnapshotView>();
const templatesCache = createKeyedSnapCache<TemplatesMarket>();
const usersCache = createSlotSnapCache();

export function clearCalculatorSnapCaches() {
  calcCache.clear();
}

export function getCalculatorSnapCache<T = unknown>(
  view: CalcSnapshotView,
): T | undefined {
  return calcCache.get<T>(view);
}

export function setCalculatorSnapCache(
  view: CalcSnapshotView,
  snap: unknown | null,
) {
  calcCache.set(view, snap);
}

export function clearTemplatesSnapCache() {
  templatesCache.clear();
}

export function getTemplatesSnapCache(
  market: TemplatesMarket,
): TemplatesSnapshot | undefined {
  return templatesCache.get<TemplatesSnapshot>(market);
}

export function setTemplatesSnapCache(
  market: TemplatesMarket,
  snap: TemplatesSnapshot,
) {
  templatesCache.set(market, snap);
}

export function clearUsersPageSnap() {
  usersCache.clear();
}

export function getUsersPageSnap<T = unknown>(): T | null {
  return usersCache.get<T>();
}

export function setUsersPageSnap(snap: unknown | null) {
  usersCache.set(snap);
}
