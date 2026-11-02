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

export type TemplatesMarket = "wb" | "ozon";

/** Snapshot по market между remount; сбрасывать при смене команды. */
const templatesSnapCache: Partial<Record<TemplatesMarket, TemplatesSnapshot>> = {};

export function clearTemplatesSnapCache() {
  for (const k of Object.keys(templatesSnapCache) as TemplatesMarket[]) {
    delete templatesSnapCache[k];
  }
}

export function getTemplatesSnapCache(
  market: TemplatesMarket,
): TemplatesSnapshot | undefined {
  return templatesSnapCache[market];
}

export function setTemplatesSnapCache(
  market: TemplatesMarket,
  snap: TemplatesSnapshot,
) {
  templatesSnapCache[market] = snap;
}
