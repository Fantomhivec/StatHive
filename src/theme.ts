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

export type ThemeScheme = "dark" | "light";

export type AppTheme =
  | "obsidian"
  | "polar"
  | "graphite"
  | "paper"
  | "frost"
  | "slate";

export type ThemeMeta = {
  id: AppTheme;
  scheme: ThemeScheme;
  /** i18n key under settings.* */
  labelKey: string;
};

export const THEME_PRESETS: readonly ThemeMeta[] = [
  { id: "obsidian", scheme: "dark", labelKey: "settings.themeObsidian" },
  { id: "polar", scheme: "dark", labelKey: "settings.themePolar" },
  { id: "graphite", scheme: "dark", labelKey: "settings.themeGraphite" },
  { id: "paper", scheme: "light", labelKey: "settings.themePaper" },
  { id: "frost", scheme: "light", labelKey: "settings.themeFrost" },
  { id: "slate", scheme: "light", labelKey: "settings.themeSlate" },
] as const;

const STORAGE_KEY = "app-theme";
const DEFAULT_THEME: AppTheme = "obsidian";

const PRESET_IDS = new Set<string>(THEME_PRESETS.map((p) => p.id));

function metaFor(id: AppTheme): ThemeMeta {
  return THEME_PRESETS.find((p) => p.id === id) ?? THEME_PRESETS[0];
}

/** Migrate legacy binary themes + validate preset id. */
export function normalizeThemeId(raw: string | null | undefined): AppTheme {
  const v = (raw ?? "").trim().toLowerCase();
  if (v === "dark") return "obsidian";
  if (v === "light") return "paper";
  if (PRESET_IDS.has(v)) return v as AppTheme;
  return DEFAULT_THEME;
}

export function readTheme(): AppTheme {
  try {
    return normalizeThemeId(localStorage.getItem(STORAGE_KEY));
  } catch {
    return DEFAULT_THEME;
  }
}

export function themeScheme(theme: AppTheme): ThemeScheme {
  return metaFor(theme).scheme;
}

/** Пишет тему на `<html>` (`data-theme` + `data-scheme` для light overrides). */
export function applyTheme(theme: AppTheme) {
  const id = normalizeThemeId(theme);
  const scheme = themeScheme(id);
  document.documentElement.dataset.theme = id;
  document.documentElement.dataset.scheme = scheme;
  try {
    localStorage.setItem(STORAGE_KEY, id);
  } catch {
    /* приватный режим */
  }
}
