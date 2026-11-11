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

import { useState } from "react";
import AppComboBox from "../components/AppComboBox";
import { useI18n, useT } from "../i18n";
import {
  applyTheme,
  normalizeThemeId,
  readTheme,
  THEME_PRESETS,
  type AppTheme,
} from "../theme";
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
          options={THEME_PRESETS.map((p) => ({
            value: p.id,
            label: t(p.labelKey),
          }))}
          onChange={(v) => {
            const next = normalizeThemeId(v);
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
