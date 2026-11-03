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

const TITLE_SUFFIX = " - Команда";
/** Старый суффикс Drive-файла (до переименования Workspace → Команда). */
const LEGACY_TITLE_SUFFIX = " - Workspace";

/** Событие после rename команды (titlebar / кэш). */
export const WORKSPACE_TITLE_CHANGED = "stathive-workspace-title-changed";

function stripKnownSuffix(title: string): string {
  const lower = title.toLowerCase();
  for (const suf of [TITLE_SUFFIX, LEGACY_TITLE_SUFFIX]) {
    if (lower.endsWith(suf.toLowerCase())) {
      const s = title.slice(0, title.length - suf.length).trim();
      return s || title;
    }
  }
  return title;
}

/** Короткое имя команды для UI (без суффикса ` - Команда` / legacy ` - Workspace`). */
export function shortTeamName(title: string | null | undefined): string {
  const t = (title ?? "").trim();
  if (!t) return "";
  return stripKnownSuffix(t);
}
