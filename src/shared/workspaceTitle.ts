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

/** Полный title таблицы/команды в Drive: `{name} - Команда`. */
export function normalizeTeamTitle(input: string | null | undefined): string {
  const base = stripKnownSuffix((input ?? "").trim()) || "StatHive";
  return `${base}${TITLE_SUFFIX}`;
}
