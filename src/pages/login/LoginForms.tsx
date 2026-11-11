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

import type { SubmitEvent } from "react";
import { useT } from "../../i18n";

type RegisterProps = {
  busy: boolean;
  googleEmail: string;
  displayName: string;
  login: string;
  password: string;
  workspaceTitle: string;
  onDisplayName: (v: string) => void;
  onLogin: (v: string) => void;
  onPassword: (v: string) => void;
  onWorkspaceTitle: (v: string) => void;
  onSwitchGoogle: () => void;
  onSubmit: (e: SubmitEvent<HTMLFormElement>) => void;
};

export function LoginRegisterForm({
  busy,
  googleEmail,
  displayName,
  login,
  password,
  workspaceTitle,
  onDisplayName,
  onLogin,
  onPassword,
  onWorkspaceTitle,
  onSwitchGoogle,
  onSubmit,
}: RegisterProps) {
  const t = useT();
  return (
    <form className="login-stack" onSubmit={onSubmit} autoComplete="off">
      <div className="login-google-row">
        <p className="muted login-google-ok">
          {t("login.googleConnectedCreateWs", {
            email: googleEmail || t("login.googleEmailFallback"),
          })}
        </p>
        <button
          type="button"
          className="btn btn-outline btn-sm"
          disabled={busy}
          onClick={onSwitchGoogle}
        >
          {t("profile.switchGoogle")}
        </button>
      </div>
      <div className="field">
        <label htmlFor="reg-name">{t("common.name")}</label>
        <input
          id="reg-name"
          name="stathive-reg-name"
          value={displayName}
          onChange={(e) => onDisplayName(e.target.value)}
          disabled={busy}
          autoComplete="off"
        />
      </div>
      <div className="field">
        <label htmlFor="reg-login">{t("common.login")}</label>
        <input
          id="reg-login"
          name="stathive-reg-login"
          value={login}
          onChange={(e) => onLogin(e.target.value)}
          disabled={busy}
          autoComplete="off"
        />
      </div>
      <div className="field">
        <label htmlFor="reg-password">{t("common.password")}</label>
        <input
          id="reg-password"
          name="stathive-reg-password"
          type="password"
          value={password}
          onChange={(e) => onPassword(e.target.value)}
          disabled={busy}
          autoComplete="off"
        />
      </div>
      <div className="field">
        <label htmlFor="ws-title">{t("login.wsTitle")}</label>
        <input
          id="ws-title"
          value={workspaceTitle}
          onChange={(e) => onWorkspaceTitle(e.target.value)}
          disabled={busy}
        />
      </div>
      <button className="btn btn-primary" type="submit" disabled={busy}>
        {busy ? t("common.creating") : t("login.createAdminWorkspace")}
      </button>
    </form>
  );
}

type MemberCodeProps = {
  busy: boolean;
  teamCode: string;
  teamCodePassphrase: string;
  onTeamCode: (v: string) => void;
  onPassphrase: (v: string) => void;
  onSubmit: (e: SubmitEvent<HTMLFormElement>) => void;
  onBack: () => void;
};

export function LoginMemberCodeForm({
  busy,
  teamCode,
  teamCodePassphrase,
  onTeamCode,
  onPassphrase,
  onSubmit,
  onBack,
}: MemberCodeProps) {
  const t = useT();
  return (
    <form className="login-stack" onSubmit={onSubmit}>
      <p className="muted">{t("login.teamCodeHint")}</p>
      <div className="field">
        <label htmlFor="team-code">{t("login.teamCode")}</label>
        <textarea
          id="team-code"
          className="input input-mono login-oauth-json"
          value={teamCode}
          onChange={(e) => onTeamCode(e.target.value)}
          disabled={busy}
          rows={4}
          placeholder="UR3.…"
        />
      </div>
      <div className="field">
        <label htmlFor="team-code-pass">{t("login.teamCodePassphrase")}</label>
        <input
          id="team-code-pass"
          type="password"
          autoComplete="off"
          value={teamCodePassphrase}
          onChange={(e) => onPassphrase(e.target.value)}
          disabled={busy}
        />
      </div>
      <button
        className="btn btn-primary"
        type="submit"
        disabled={busy || !teamCode.trim() || teamCodePassphrase.length < 8}
      >
        {t("login.applyTeamCode")}
      </button>
      <button type="button" className="btn btn-ghost" disabled={busy} onClick={onBack}>
        {t("common.back")}
      </button>
    </form>
  );
}

type SignInProps = {
  busy: boolean;
  welcomeTeam: string;
  memberEmail: string;
  login: string;
  password: string;
  showBackToHub: boolean;
  showOtherTeamCode: boolean;
  onMemberEmail: (v: string) => void;
  onLogin: (v: string) => void;
  onPassword: (v: string) => void;
  onSubmit: (e: SubmitEvent<HTMLFormElement>) => void;
  onBackToHub: () => void;
  onOtherTeamCode: () => void;
};

export function LoginSignInForm({
  busy,
  welcomeTeam,
  memberEmail,
  login,
  password,
  showBackToHub,
  showOtherTeamCode,
  onMemberEmail,
  onLogin,
  onPassword,
  onSubmit,
  onBackToHub,
  onOtherTeamCode,
}: SignInProps) {
  const t = useT();
  return (
    <form className="login-stack" onSubmit={onSubmit} autoComplete="off">
      {welcomeTeam ? (
        <p className="login-welcome">{t("login.welcomeTeam", { name: welcomeTeam })}</p>
      ) : null}
      <div className="field">
        <label htmlFor="member-email">{t("users.googleEmail")}</label>
        <input
          id="member-email"
          name="stathive-member-email"
          type="text"
          inputMode="email"
          value={memberEmail}
          onChange={(e) => onMemberEmail(e.target.value)}
          disabled={busy}
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          required
        />
      </div>
      <div className="field">
        <label htmlFor="login">{t("common.login")}</label>
        <input
          id="login"
          name="stathive-login"
          value={login}
          onChange={(e) => onLogin(e.target.value)}
          disabled={busy}
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          autoFocus
        />
      </div>
      <div className="field">
        <label htmlFor="password">{t("common.password")}</label>
        <input
          id="password"
          name="stathive-password"
          type="password"
          value={password}
          onChange={(e) => onPassword(e.target.value)}
          disabled={busy}
          autoComplete="off"
        />
      </div>
      <button className="btn btn-primary" type="submit" disabled={busy}>
        {busy ? t("common.signingIn") : t("login.signIn")}
      </button>
      {showBackToHub ? (
        <button type="button" className="btn btn-ghost" disabled={busy} onClick={onBackToHub}>
          {t("login.backToHub")}
        </button>
      ) : showOtherTeamCode ? (
        <button type="button" className="btn btn-ghost" disabled={busy} onClick={onOtherTeamCode}>
          {t("login.otherTeamCode")}
        </button>
      ) : null}
    </form>
  );
}
