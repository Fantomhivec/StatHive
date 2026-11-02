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

/** Сброс кэшей карточки/прогрева после обновления таблиц. */

const SESSION_PREFIX = "calc:lastLookup:";
export const CALC_LOOKUP_INVALIDATE_EVENT = "calc-lookup-invalidate";

type WarmFlag = { started: boolean };

/** Общий флаг с useCalculatorSnapshot (один модульный прогрев на сессию). */
const warmFlag: WarmFlag = { started: false };

export function isLookupWarmStarted(): boolean {
  return warmFlag.started;
}

export function markLookupWarmStarted(): void {
  warmFlag.started = true;
}

export function resetLookupWarmFlag(): void {
  warmFlag.started = false;
}

export function clearCalcLookupSessions(): void {
  try {
    const keys: string[] = [];
    for (let i = 0; i < sessionStorage.length; i++) {
      const k = sessionStorage.key(i);
      if (k?.startsWith(SESSION_PREFIX)) keys.push(k);
    }
    for (const k of keys) sessionStorage.removeItem(k);
  } catch {
    // private mode / quota
  }
}

/** После create/refresh таблиц: сессия карточки + повторный прогрев RAM. */
export function invalidateCalculatorLookupCaches(): void {
  clearCalcLookupSessions();
  resetLookupWarmFlag();
  try {
    window.dispatchEvent(new CustomEvent(CALC_LOOKUP_INVALIDATE_EVENT));
  } catch {
    // ignore
  }
}
