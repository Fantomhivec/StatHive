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

export type CalcSnapshotView = "view" | "settings" | "tables" | "kgt" | "full";

/** Snapshot по view между remount; сбрасывать при смене команды. */
const cacheByView: Partial<Record<CalcSnapshotView, unknown>> = {};

export function clearCalculatorSnapCaches() {
  for (const k of Object.keys(cacheByView) as CalcSnapshotView[]) {
    delete cacheByView[k];
  }
}

export function getCalculatorSnapCache<T = unknown>(
  view: CalcSnapshotView,
): T | undefined {
  return cacheByView[view] as T | undefined;
}

export function setCalculatorSnapCache(
  view: CalcSnapshotView,
  snap: unknown | null,
) {
  if (snap) cacheByView[view] = snap;
  else delete cacheByView[view];
}
