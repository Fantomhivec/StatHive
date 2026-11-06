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

/** Pixels from bottom that still count as «at end» (lesson 122). */
export const STICK_SCROLL_THRESHOLD_PX = 48;

/** True when the scrollport is within threshold of the bottom. */
export function isNearScrollBottom(
  el: HTMLElement,
  thresholdPx: number = STICK_SCROLL_THRESHOLD_PX,
): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight < thresholdPx;
}

/** If near bottom, jump to end so new lines stay visible. */
export function stickScrollIfNearBottom(
  el: HTMLElement | null | undefined,
  thresholdPx: number = STICK_SCROLL_THRESHOLD_PX,
): void {
  if (!el) return;
  if (isNearScrollBottom(el, thresholdPx)) el.scrollTop = el.scrollHeight;
}
