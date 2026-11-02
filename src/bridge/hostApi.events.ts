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

/** Thin event listen — shell/auth without wbCalculator. */
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Подписка на host-события (updates.progress, wbCalculator.runLog и т.п.). */
export function onEvent<T = unknown>(
  event: string,
  handler: (payload: T) => void,
): () => void {
  let un: UnlistenFn | undefined;
  let cancelled = false;
  void listen<T>(event, (e) => {
    if (!cancelled) handler(e.payload);
  }).then((fn) => {
    if (cancelled) {
      fn();
      return;
    }
    un = fn;
  });
  return () => {
    cancelled = true;
    un?.();
  };
}
