/** Тонкий i18n-мост — без импорта всего hostApi (shell / i18n не тянут calc invoke). */
import { invoke } from "@tauri-apps/api/core";

export function getLocale(): Promise<string> {
  return invoke<string>("i18n_get_locale");
}

export function setLocale(locale: string): Promise<string> {
  return invoke<string>("i18n_set_locale", { locale });
}
