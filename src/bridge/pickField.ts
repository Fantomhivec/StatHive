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

/** Prefer camelCase wire field, fall back to snake_case (legacy dual-read). */
export function pickStr(
  row: Record<string, unknown>,
  camel: string,
  snake: string,
  fallback = "",
): string {
  const v = row[camel] ?? row[snake] ?? fallback;
  return String(v ?? "").trim();
}

export function pickNum(
  row: Record<string, unknown>,
  camel: string,
  snake: string,
  fallback = 0,
): number {
  const v = row[camel] ?? row[snake] ?? fallback;
  const n = Number(v);
  return Number.isFinite(n) ? n : fallback;
}

export function pickBool(
  row: Record<string, unknown>,
  camel: string,
  snake: string,
): boolean {
  return Boolean(row[camel] ?? row[snake]);
}
