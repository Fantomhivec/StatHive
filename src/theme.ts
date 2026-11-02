export type AppTheme = "dark" | "light";

const STORAGE_KEY = "app-theme";

export function readTheme(): AppTheme {
  try {
    return localStorage.getItem(STORAGE_KEY) === "light" ? "light" : "dark";
  } catch {
    return "dark";
  }
}

/** Пишет тему на `<html>`, чтобы шапка и страницы читали одни CSS-переменные. */
export function applyTheme(theme: AppTheme) {
  document.documentElement.dataset.theme = theme;
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    /* приватный режим */
  }
}
