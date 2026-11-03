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

import { shortTeamName } from "../../bridge/workspaceTitle";
import "../LoginPage.css";
import { LoginChoosePanel, LoginHubPanel } from "./LoginHubPanel";
import { LoginMemberCodeForm, LoginRegisterForm, LoginSignInForm } from "./LoginForms";
import { LoginOauthSection } from "./LoginOauthSection";
import { LoginTeamListPanel } from "./LoginTeamListPanel";
import { teamsOf } from "./loginGate";
import { useLoginPageModel } from "./useLoginPageModel";

export default function LoginPage() {
  const m = useLoginPageModel();
  const {
    t,
    dialogs,
    setup,
    gate,
    setGate,
    setError,
    oauthJson,
    setOauthJson,
    oauthReplacing,
    setOauthReplacing,
    teamCode,
    setTeamCode,
    teamCodePassphrase,
    setTeamCodePassphrase,
    login,
    setLogin,
    password,
    setPassword,
    memberEmail,
    setMemberEmail,
    displayName,
    setDisplayName,
    workspaceTitle,
    setWorkspaceTitle,
    error,
    busy,
    onSaveOauth,
    onConnectGoogle,
    onSwitchGoogle,
    openAdminOauth,
    onHubNewOauth,
    onPickTeam,
    onDeleteTeam,
    onRegister,
    onApplyCode,
    backFromMemberCode,
    backToHubOrChoose,
    onLogin,
  } = m;

  if (!setup) {
    return (
      <div className="login-page">
        <div className="card card--stack card--xl login-card">{t("common.loading")}</div>
      </div>
    );
  }

  const showOauthForm = !setup.oauth_configured || oauthReplacing;
  const oauthClientHint =
    (setup.oauth_client_id_hint ?? setup.oauthClientIdHint ?? "").trim();
  const welcomeTeam = shortTeamName(setup.workspace_title);
  const savedTeams = teamsOf(setup);
  const activeId = (setup.active_team_id ?? setup.activeTeamId ?? "").trim();
  const oauthCardClass =
    "card card--stack card--xl login-card" +
    (gate === "admin_oauth" ? " login-card--oauth" : " login-card--wide");

  return (
    <div className="login-page">
      <div className={oauthCardClass}>
        <h1>StatHive</h1>
        <p className="sub">
          {gate === "login"
            ? t("login.subtitlePassword")
            : gate === "hub" || gate === "pick_team"
              ? t("login.hubHint")
              : t("login.subtitleSetup")}
        </p>

        {gate === "hub" && (
          <LoginHubPanel
            busy={busy}
            onExisting={() => {
              setError(null);
              setGate("pick_team");
            }}
            onTeamCode={() => {
              setError(null);
              setGate("member_code");
            }}
            onNewOauth={() => void onHubNewOauth()}
          />
        )}

        {gate === "pick_team" && (
          <LoginTeamListPanel
            busy={busy}
            teams={savedTeams}
            activeId={activeId}
            onPick={(id) => void onPickTeam(id)}
            onDelete={(team) => void onDeleteTeam(team)}
            onBack={backToHubOrChoose}
          />
        )}

        {gate === "choose" && (
          <LoginChoosePanel
            busy={busy}
            workspaceConfigured={setup.workspace_configured}
            onGoSignIn={() => {
              setError(null);
              setGate("login");
            }}
            onIAmAdmin={() => openAdminOauth(false)}
            onIAmMember={() => setGate("member_code")}
          />
        )}

        {gate === "admin_oauth" && (
          <LoginOauthSection
            busy={busy}
            showOauthForm={showOauthForm}
            oauthConfigured={setup.oauth_configured}
            oauthReplacing={oauthReplacing}
            oauthJson={oauthJson}
            oauthClientHint={oauthClientHint}
            googleConnected={setup.google_connected}
            googleEmail={setup.google_email ?? ""}
            onOauthJsonChange={setOauthJson}
            onSaveOauth={() => void onSaveOauth()}
            onConnectGoogle={() => void onConnectGoogle()}
            onStartReplace={() => {
              setOauthReplacing(true);
              setOauthJson("");
              setError(null);
            }}
            onCancelReplace={() => {
              setOauthReplacing(false);
              setOauthJson("");
            }}
            onBack={() => {
              setOauthReplacing(false);
              setOauthJson("");
              backToHubOrChoose();
            }}
          />
        )}

        {gate === "admin_register" && (
          <LoginRegisterForm
            busy={busy}
            googleEmail={setup.google_email || ""}
            displayName={displayName}
            login={login}
            password={password}
            workspaceTitle={workspaceTitle}
            onDisplayName={setDisplayName}
            onLogin={setLogin}
            onPassword={setPassword}
            onWorkspaceTitle={setWorkspaceTitle}
            onSwitchGoogle={() => void onSwitchGoogle()}
            onSubmit={(e) => void onRegister(e)}
          />
        )}

        {gate === "member_code" && (
          <LoginMemberCodeForm
            busy={busy}
            teamCode={teamCode}
            teamCodePassphrase={teamCodePassphrase}
            onTeamCode={setTeamCode}
            onPassphrase={setTeamCodePassphrase}
            onSubmit={(e) => void onApplyCode(e)}
            onBack={backFromMemberCode}
          />
        )}

        {gate === "login" && (
          <LoginSignInForm
            busy={busy}
            welcomeTeam={welcomeTeam}
            memberEmail={memberEmail}
            login={login}
            password={password}
            showBackToHub={savedTeams.length >= 1}
            showOtherTeamCode={!setup.workspace_configured}
            onMemberEmail={setMemberEmail}
            onLogin={setLogin}
            onPassword={setPassword}
            onSubmit={(e) => void onLogin(e)}
            onBackToHub={backToHubOrChoose}
            onOtherTeamCode={() => setGate("member_code")}
          />
        )}

        {error && <div className="error">{error}</div>}
      </div>
      {dialogs}
    </div>
  );
}
