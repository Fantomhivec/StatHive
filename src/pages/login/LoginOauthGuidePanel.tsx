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

import { useT } from "../../i18n";

/** Компактная инструкция OAuth Desktop — всегда на экране Admin. */
export function LoginOauthGuidePanel() {
  const t = useT();
  const steps = [
    t("login.oauthGuide.step1"),
    t("login.oauthGuide.step2"),
    t("login.oauthGuide.step3"),
    t("login.oauthGuide.step4"),
    t("login.oauthGuide.step5"),
    t("login.oauthGuide.step6"),
    t("login.oauthGuide.step7"),
    t("login.oauthGuide.step8"),
  ];

  return (
    <aside className="login-oauth-guide-panel" aria-label={t("login.oauthGuide.title")}>
      <header className="login-oauth-guide-head">
        <h2 className="login-oauth-guide-title">{t("login.oauthGuide.title")}</h2>
        <p className="muted login-oauth-guide-lead">{t("login.oauthGuide.lead")}</p>
      </header>
      <ol className="login-oauth-guide-list">
        {steps.map((text, i) => (
          <li key={i} className="login-oauth-guide-step">
            <span className="login-oauth-guide-num" aria-hidden>
              {i + 1}
            </span>
            <span className="login-oauth-guide-text">{text}</span>
          </li>
        ))}
      </ol>
      <p className="muted login-oauth-guide-scopes">{t("login.oauthGuide.scopes")}</p>
    </aside>
  );
}
