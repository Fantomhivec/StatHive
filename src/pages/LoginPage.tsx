import { FormEvent, useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { takeAccessRevokedMessage, useAuth } from "../auth/AuthContext";
import { hostApi, openUrl, type AuthSetupStatus, type TeamIndexEntry } from "../bridge/hostApi";
import { DISCORD_COMMUNITY_NAME, DISCORD_INVITE_URL } from "../shared/community";
import { shortTeamName } from "../shared/workspaceTitle";
import { LoginOauthGuidePanel } from "./LoginOauthGuidePanel";
import "./LoginPage.css";
import { useT } from "../i18n";

type Gate =
  | "hub"
  | "choose"
  | "pick_team"
  | "admin_oauth"
  | "admin_register"
  | "member_code"
  | "login";

function teamsOf(s: AuthSetupStatus): TeamIndexEntry[] {
  return s.teams ?? [];
}

function pickGate(s: AuthSetupStatus): Gate {
  if (teamsOf(s).length >= 1) return "hub";
  if (s.oauth_configured && s.workspace_configured) return "login";
  if (s.oauth_configured && s.google_connected && s.can_register_admin) return "admin_register";
  if (s.oauth_configured && !s.google_connected) return "admin_oauth";
  if (s.oauth_configured && s.can_register_admin) return "admin_register";
  return "choose";
}

function teamLabel(t: TeamIndexEntry): string {
  const title = (t.title ?? "").trim();
  if (title) return shortTeamName(title) || title;
  return t.id;
}

export default function LoginPage() {
  const t = useT();
  const navigate = useNavigate();
  const location = useLocation();
  const { setUser } = useAuth();
  const [setup, setSetup] = useState<AuthSetupStatus | null>(null);
  const [gate, setGate] = useState<Gate>("login");
  const [oauthJson, setOauthJson] = useState("");
  const [oauthReplacing, setOauthReplacing] = useState(false);
  const [teamCode, setTeamCode] = useState("");
  const [teamCodePassphrase, setTeamCodePassphrase] = useState("");
  const [login, setLogin] = useState("");
  const [password, setPassword] = useState("");
  const [memberEmail, setMemberEmail] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [workspaceTitle, setWorkspaceTitle] = useState("StatHive");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function refreshSetup(prefer?: Gate) {
    const s = await hostApi.authSetupStatus();
    setSetup(s);
    setGate(prefer ?? pickGate(s));
    if (!s.oauth_configured) setOauthReplacing(false);
  }

  useEffect(() => {
    const asked = (location.state as { gate?: Gate } | null)?.gate;
    void refreshSetup(asked).catch((e: Error) => setError(e.message));
  }, [location.state]);

  useEffect(() => {
    const revoked = takeAccessRevokedMessage();
    if (!revoked) return;
    setError(revoked === "1" ? t("login.noProjectAccess") : revoked);
  }, [t]);

  async function onSaveOauth() {
    setError(null);
    setBusy(true);
    try {
      await hostApi.saveOauthClient(oauthJson);
      setOauthJson("");
      setOauthReplacing(false);
      await hostApi.connectGoogle();
      await refreshSetup();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      await refreshSetup("admin_oauth").catch(() => undefined);
    } finally {
      setBusy(false);
    }
  }

  async function onConnectGoogle() {
    setError(null);
    setBusy(true);
    try {
      await hostApi.connectGoogle();
      await refreshSetup();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  function onSwitchGoogle() {
    setError(null);
    setMemberEmail("");
    setGate(setup && teamsOf(setup).length >= 1 ? "hub" : "choose");
  }

  function openAdminOauth(replacing = false) {
    setError(null);
    setOauthReplacing(replacing);
    setOauthJson("");
    setGate("admin_oauth");
  }

  async function onHubNewOauth() {
    setError(null);
    setBusy(true);
    try {
      await hostApi.beginNewTeam();
      openAdminOauth(false);
      await refreshSetup("admin_oauth");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function onPickTeam(id: string) {
    setError(null);
    setBusy(true);
    try {
      await hostApi.setActiveTeam(id);
      await refreshSetup("login");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function onRegister(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const u = await hostApi.registerAdmin({
        login: login.trim(),
        password,
        display_name: displayName.trim() || login.trim(),
        workspace_title: workspaceTitle.trim(),
      });
      setUser(u);
      navigate("/", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function onApplyCode(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await hostApi.applyTeamInvite(teamCode.trim(), teamCodePassphrase.trim());
      setTeamCode("");
      setTeamCodePassphrase("");
      await refreshSetup("login");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  function backFromMemberCode() {
    setError(null);
    setTeamCode("");
    if (setup && teamsOf(setup).length >= 1) setGate("hub");
    else setGate(setup?.workspace_configured ? "login" : "choose");
  }

  function backToHubOrChoose() {
    setError(null);
    setGate(setup && teamsOf(setup).length >= 1 ? "hub" : "choose");
  }

  async function onLogin(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const u = await hostApi.login(login.trim(), password, memberEmail.trim());
      setUser(u);
      navigate("/", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

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
          <div className="login-stack">
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy}
              onClick={() => {
                setError(null);
                setGate("pick_team");
              }}
            >
              {t("login.hubExisting")}
            </button>
            <button
              type="button"
              className="btn btn-outline"
              disabled={busy}
              onClick={() => {
                setError(null);
                setGate("member_code");
              }}
            >
              {t("login.hubTeamCode")}
            </button>
            <button
              type="button"
              className="btn btn-outline"
              disabled={busy}
              onClick={() => void onHubNewOauth()}
            >
              {t("login.hubNewOauth")}
            </button>
            <button
              type="button"
              className="btn btn-outline"
              disabled={busy}
              title={t("users.discordTitle", { name: DISCORD_COMMUNITY_NAME })}
              onClick={() => void openUrl(DISCORD_INVITE_URL)}
            >
              {t("users.discordBtn", { name: DISCORD_COMMUNITY_NAME })}
            </button>
          </div>
        )}

        {gate === "pick_team" && (
          <div className="login-stack">
            <p className="muted">{t("login.pickTeamHint")}</p>
            {savedTeams.length === 0 ? (
              <p className="muted">{t("login.pickTeamEmpty")}</p>
            ) : (
              <ul className="login-team-list">
                {savedTeams.map((team) => (
                  <li key={team.id} className="login-team-row">
                    <button
                      type="button"
                      className="btn btn-outline login-team-pick"
                      disabled={busy}
                      onClick={() => void onPickTeam(team.id)}
                    >
                      <span className="login-team-name">{teamLabel(team)}</span>
                      {team.id === activeId ? (
                        <span className="muted login-team-badge">{t("login.activeTeamMark")}</span>
                      ) : null}
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <button type="button" className="btn btn-ghost" disabled={busy} onClick={backToHubOrChoose}>
              {t("login.backToHub")}
            </button>
          </div>
        )}

        {gate === "choose" && (
          <div className="login-stack">
            {setup.workspace_configured ? (
              <>
                <p className="muted">{t("login.workspaceReadyHint")}</p>
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={busy}
                  onClick={() => {
                    setError(null);
                    setGate("login");
                  }}
                >
                  {t("login.goToSignIn")}
                </button>
                <button
                  type="button"
                  className="btn btn-outline"
                  disabled={busy}
                  onClick={() => openAdminOauth(false)}
                >
                  {t("login.iAmAdmin")}
                </button>
              </>
            ) : (
              <>
                <p className="muted">{t("login.whoAreYou")}</p>
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={busy}
                  onClick={() => openAdminOauth(false)}
                >
                  {t("login.iAmAdmin")}
                </button>
                <button
                  type="button"
                  className="btn btn-outline"
                  disabled={busy}
                  onClick={() => setGate("member_code")}
                >
                  {t("login.iAmMember")}
                </button>
              </>
            )}
            <button
              type="button"
              className="btn btn-outline"
              disabled={busy}
              title={t("users.discordTitle", { name: DISCORD_COMMUNITY_NAME })}
              onClick={() => void openUrl(DISCORD_INVITE_URL)}
            >
              {t("users.discordBtn", { name: DISCORD_COMMUNITY_NAME })}
            </button>
          </div>
        )}

        {gate === "admin_oauth" && (
          <div className="login-stack login-stack--oauth">
            <LoginOauthGuidePanel />

            {showOauthForm ? (
              <>
                <p className="muted">{t("login.oauthJsonHint")}</p>
                <div className="field">
                  <label htmlFor="oauth-json">OAuth client JSON</label>
                  <textarea
                    id="oauth-json"
                    className="input input-mono login-oauth-json"
                    value={oauthJson}
                    onChange={(e) => setOauthJson(e.target.value)}
                    disabled={busy}
                    rows={4}
                    placeholder='{"installed":{"client_id":"...","client_secret":"..."}}'
                  />
                </div>
                <div className="form-actions">
                  <button
                    className="btn btn-primary"
                    type="button"
                    disabled={busy || !oauthJson.trim()}
                    onClick={() => void onSaveOauth()}
                  >
                    {t("login.saveAndConnectGoogle")}
                  </button>
                  {oauthReplacing ? (
                    <button
                      type="button"
                      className="btn btn-ghost"
                      disabled={busy}
                      onClick={() => {
                        setOauthReplacing(false);
                        setOauthJson("");
                      }}
                    >
                      {t("common.cancel")}
                    </button>
                  ) : null}
                </div>
              </>
            ) : (
              <div className="login-oauth-current">
                <p className="muted u-m-0">{t("login.oauthSavedHint")}</p>
                {oauthClientHint ? (
                  <p className="mono u-m-0">
                    {t("login.oauthCurrentClient", { id: oauthClientHint })}
                  </p>
                ) : null}
                <p className="muted u-m-0">
                  {setup.google_connected && (setup.google_email ?? "").trim()
                    ? t("login.oauthCurrentGoogle", {
                        email: (setup.google_email ?? "").trim(),
                      })
                    : t("login.oauthCurrentGoogleNone")}
                </p>
                {!setup.google_connected ? (
                  <button
                    className="btn btn-primary"
                    type="button"
                    disabled={busy}
                    onClick={() => void onConnectGoogle()}
                  >
                    {t("login.connectGoogle")}
                  </button>
                ) : null}
              </div>
            )}

            <div className="login-oauth-footer form-actions">
              {setup.oauth_configured && !oauthReplacing ? (
                <button
                  type="button"
                  className="btn btn-outline"
                  disabled={busy}
                  onClick={() => {
                    setOauthReplacing(true);
                    setOauthJson("");
                    setError(null);
                  }}
                >
                  {t("login.newOauth")}
                </button>
              ) : null}
              <button
                type="button"
                className="btn btn-ghost"
                disabled={busy}
                onClick={() => {
                  setOauthReplacing(false);
                  setOauthJson("");
                  backToHubOrChoose();
                }}
              >
                {t("common.back")}
              </button>
            </div>
          </div>
        )}

        {gate === "admin_register" && (
          <form className="login-stack" onSubmit={onRegister} autoComplete="off">
            <div className="login-google-row">
              <p className="muted login-google-ok">
                {t("login.googleConnectedCreateWs", {
                  email: setup.google_email || t("login.googleEmailFallback"),
                })}
              </p>
              <button
                type="button"
                className="btn btn-outline btn-sm"
                disabled={busy}
                onClick={() => void onSwitchGoogle()}
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
                onChange={(e) => setDisplayName(e.target.value)}
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
                onChange={(e) => setLogin(e.target.value)}
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
                onChange={(e) => setPassword(e.target.value)}
                disabled={busy}
                autoComplete="off"
              />
            </div>
            <div className="field">
              <label htmlFor="ws-title">{t("login.wsTitle")}</label>
              <input
                id="ws-title"
                value={workspaceTitle}
                onChange={(e) => setWorkspaceTitle(e.target.value)}
                disabled={busy}
              />
            </div>
            <button className="btn btn-primary" type="submit" disabled={busy}>
              {busy ? t("common.creating") : t("login.createAdminWorkspace")}
            </button>
          </form>
        )}

        {gate === "member_code" && (
          <form className="login-stack" onSubmit={onApplyCode}>
            <p className="muted">{t("login.teamCodeHint")}</p>
            <div className="field">
              <label htmlFor="team-code">{t("login.teamCode")}</label>
              <textarea
                id="team-code"
                className="input input-mono login-oauth-json"
                value={teamCode}
                onChange={(e) => setTeamCode(e.target.value)}
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
                onChange={(e) => setTeamCodePassphrase(e.target.value)}
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
            <button type="button" className="btn btn-ghost" disabled={busy} onClick={backFromMemberCode}>
              {t("common.back")}
            </button>
          </form>
        )}

        {gate === "login" && (
          <form className="login-stack" onSubmit={onLogin} autoComplete="off">
            {welcomeTeam ? (
              <p className="login-welcome">
                {t("login.welcomeTeam", { name: welcomeTeam })}
              </p>
            ) : null}
            <div className="field">
              <label htmlFor="member-email">{t("users.googleEmail")}</label>
              <input
                id="member-email"
                name="stathive-member-email"
                type="text"
                inputMode="email"
                value={memberEmail}
                onChange={(e) => setMemberEmail(e.target.value)}
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
                onChange={(e) => setLogin(e.target.value)}
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
                onChange={(e) => setPassword(e.target.value)}
                disabled={busy}
                autoComplete="off"
              />
            </div>
            <button className="btn btn-primary" type="submit" disabled={busy}>
              {busy ? t("common.signingIn") : t("login.signIn")}
            </button>
            {savedTeams.length >= 1 ? (
              <button
                type="button"
                className="btn btn-ghost"
                disabled={busy}
                onClick={backToHubOrChoose}
              >
                {t("login.backToHub")}
              </button>
            ) : !setup.workspace_configured ? (
              <button
                type="button"
                className="btn btn-ghost"
                disabled={busy}
                onClick={() => setGate("member_code")}
              >
                {t("login.otherTeamCode")}
              </button>
            ) : null}
          </form>
        )}

        {error && <div className="error">{error}</div>}
      </div>
    </div>
  );
}
