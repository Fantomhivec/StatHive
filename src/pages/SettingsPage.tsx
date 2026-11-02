import { useState } from "react";
import AppComboBox from "../components/AppComboBox";
import { useI18n, useT } from "../i18n";
import { applyTheme, readTheme, type AppTheme } from "../theme";
import type { AppLocale } from "../i18n";

/** Настройка интерфейса (тема). Вкладки — из шестерёнки title bar. */
export default function SettingsPage() {
  const t = useT();
  const { locale, setLocale } = useI18n();
  const [theme, setTheme] = useState<AppTheme>(readTheme);

  return (
    <div className="settings-page-stack">
      <h1 className="page-title">{t("settings.pageTitle")}</h1>
      <p className="page-sub">{t("settings.pageSub")}</p>
      <div className="field theme-field">
        <label htmlFor="app-theme">{t("settings.themeLabel")}</label>
        <AppComboBox
          value={theme}
          options={[
            { value: "dark", label: t("settings.themeDark") },
            { value: "light", label: t("settings.themeLight") },
          ]}
          onChange={(v) => {
            const next: AppTheme = v === "light" ? "light" : "dark";
            setTheme(next);
            applyTheme(next);
          }}
        />
      </div>
      <div className="field theme-field">
        <label htmlFor="app-locale">{t("settings.languageLabel")}</label>
        <AppComboBox
          value={locale}
          options={[{ value: "ru", label: t("settings.languageRu") }]}
          onChange={(v) => {
            if (v === "ru") void setLocale(v as AppLocale);
          }}
        />
      </div>
      <p className="page-sub muted">{t("settings.shortcutsHint")}</p>
    </div>
  );
}
