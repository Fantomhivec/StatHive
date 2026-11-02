import { useT } from "../i18n";

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
