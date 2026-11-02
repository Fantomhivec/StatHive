import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { isMainAdminRole } from "../bridge/accessRole";
import {
  ensureAppVersion,
  ensureUpdatesCheckQuiet,
  hostApi,
  onEvent,
  type AuthSetupStatus,
  type TeamIndexEntry,
  type UpdateInfo,
  type UserInfo,
} from "../bridge/hostApi";
import { useAuth } from "../auth/AuthContext";
import { IconEye } from "../components/icons";
import { useTabsLayoutCaps } from "../hooks/useTabsLayoutCaps";
import { shortTeamName } from "../shared/workspaceTitle";
import "./ProfilePage.css";
import { useT } from "../i18n";

function teamLabel(entry: TeamIndexEntry): string {
  const title = (entry.title ?? "").trim();
  if (title) return shortTeamName(title) || title;
  return entry.id;
}

function PasswordField({
  label,
  value,
  onChange,
  disabled,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}) {
  const t = useT();
  const [show, setShow] = useState(false);
  return (
    <div className="profile-field">
      <label className="profile-label">{label}</label>
      <div className="profile-password-row">
        <input
          className="input-sm"
          type={show ? "text" : "password"}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
        />
        <button
          type="button"
          className="btn btn-ghost btn-icon"
          onClick={() => setShow((s) => !s)}
          disabled={disabled}
          aria-label={show ? t("profile.hidePassword") : t("profile.showPassword")}
          title={show ? t("profile.hidePassword") : t("profile.showPassword")}
        >
          <IconEye hidden={show} />
        </button>
      </div>
    </div>
  );
}

export default function ProfilePage() {
  const t = useT();
  const navigate = useNavigate();
  const { user: authUser, setUser } = useAuth();
  const [profile, setProfile] = useState<UserInfo | null>(null);
  const [version, setVersion] = useState("");
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [pwdError, setPwdError] = useState<string | null>(null);
  const [pwdOk, setPwdOk] = useState<string | null>(null);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [setup, setSetup] = useState<AuthSetupStatus | null>(null);
  const layoutCaps = useTabsLayoutCaps();

  const isAdmin = isMainAdminRole(profile?.access_level);
  const canSeeWorkspaceId = isAdmin || !!layoutCaps.canUsersTab || !!layoutCaps.isMainAdmin;

  useEffect(() => {
    if (authUser === undefined) return;
    setProfile(authUser);
    if (!authUser?.is_authorized) return;
    let cancelled = false;
    void (async () => {
      try {
        const [fresh, s] = await Promise.all([
          hostApi.refreshProfile().catch(() => null),
          hostApi.authSetupStatus().catch(() => null),
        ]);
        if (cancelled) return;
        if (fresh) {
          setProfile(fresh);
          setUser(fresh);
        }
        if (s) setSetup(s);
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [authUser, setUser]);

  useEffect(() => {
    void ensureAppVersion().then(setVersion).catch(() => undefined);
    void ensureUpdatesCheckQuiet()
      .then(setUpdate)
      .catch(() => undefined);

    return onEvent<{ percent?: number }>("updates.progress", (payload) => {
      const p = payload?.percent;
      if (typeof p === "number") setProgress(Math.max(0, Math.min(100, p)));
    });
  }, []);

  async function onChangePassword() {
    setPwdError(null);
    setPwdOk(null);
    setBusy(true);
    try {
      const res = await hostApi.changePassword(currentPassword, newPassword, confirmPassword);
      if (res.success) {
        setPwdOk(t("profile.pwdChangedOk"));
        setCurrentPassword("");
        setNewPassword("");
        setConfirmPassword("");
      } else {
        setPwdError(res.error || t("profile.pwdChangeFail"));
      }
    } catch (e) {
      setPwdError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onCheckUpdates() {
    setBusy(true);
    setError(null);
    try {
      setUpdate(await hostApi.updatesCheck(true));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onInstallUpdate() {
    setBusy(true);
    setError(null);
    setProgress(0);
    try {
      const res = await hostApi.updatesDownloadInstall();
      if (!res.success) {
        setError(res.message || t("profile.updateInstallFail"));
        setProgress(null);
      }
    } catch (e) {
      // Tauri обычно кладёт текст Err(String) в message — показываем как есть (детали также в Логах).
      const raw = e instanceof Error ? e.message : String(e);
      setError(raw || t("profile.updateErrorLogs"));
      setProgress(null);
    } finally {
      setBusy(false);
    }
  }

  async function onSwitchGoogle() {
    setBusy(true);
    setError(null);
    try {
      await hostApi.logout();
      setUser(null);
      navigate("/login", { replace: true, state: { gate: "choose" } });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setBusy(false);
    }
  }

  async function onLogout() {
    setBusy(true);
    try {
      await hostApi.logout();
      setUser(null);
      navigate("/login", { replace: true, state: { gate: "hub" } });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setBusy(false);
    }
  }

  async function onSwitchTeam(id: string) {
    setBusy(true);
    setError(null);
    try {
      const res = await hostApi.switchTeam(id);
      if (res.status === "ok" && res.user) {
        setUser(res.user);
        setProfile(res.user);
        const s = await hostApi.authSetupStatus().catch(() => null);
        if (s) setSetup(s);
        navigate("/", { replace: true });
        return;
      }
      setUser(null);
      navigate("/login", { replace: true, state: { gate: "login" } });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!profile && !error) {
    return <div className="profile-page muted">{t("common.loadingProfile")}</div>;
  }

  const hasUpdate = !!(update?.has_update ?? update?.is_newer);
  const currentVer = update?.current_version || version || "—";
  const latestVer = update?.latest_version || update?.version || "—";

  return (
    <div className="profile-page">
      <div className="profile-header">
        <h1 className="page-heading">{t("profile.title")}</h1>
      </div>
      {error && <p className="error">{error}</p>}

      {profile && (
        <fieldset className="card card--flush card--stack profile-group">
          <legend>{t("profile.personalInfo")}</legend>
          <div className="profile-grid">
            <span className="profile-label">{t("profile.fullName")}</span>
            <span>{profile.display_name || "—"}</span>
            <span className="profile-label">{t("profile.login")}</span>
            <span>{profile.login || "—"}</span>
            <span className="profile-label">{t("profile.accessLevel")}</span>
            <span>{profile.access_level || "—"}</span>
            <span className="profile-label">Google:</span>
            <span className="profile-google-cell">
              <span>{profile.google_email || setup?.google_email || "—"}</span>
              {isAdmin && (
                <button
                  type="button"
                  className="btn btn-outline btn-sm"
                  disabled={busy}
                  onClick={() => void onSwitchGoogle()}
                >
                  {t("profile.switchGoogle")}
                </button>
              )}
            </span>
            <span className="profile-label">{t("profile.workspaceTitle")}</span>
            <span>
              {shortTeamName(setup?.workspace_title) ||
                setup?.workspace_title?.trim() ||
                "—"}
            </span>
            {canSeeWorkspaceId ? (
              <>
                <span className="profile-label">{t("profile.workspaceId")}</span>
                <span className="mono">{setup?.workspace_id || "—"}</span>
              </>
            ) : null}
            <span className="profile-label">{t("profile.lastLogin")}</span>
            <span>{profile.last_login || t("common.notSpecified")}</span>
          </div>
        </fieldset>
      )}

      {canSeeWorkspaceId && (
        <fieldset className="card card--flush card--stack profile-group">
          <legend>{t("profile.teamSection")}</legend>
          <p className="muted">
            {t("profile.teamHintBeforeLink")}{" "}
            <Link to="/users">{t("nav.users")}</Link>.
          </p>
        </fieldset>
      )}

      {(setup?.teams?.length ?? 0) >= 1 && (
        <fieldset className="card card--flush card--stack profile-group">
          <legend>{t("profile.savedTeams")}</legend>
          <p className="muted u-m-0">{t("profile.savedTeamsHint")}</p>
          <ul className="profile-team-list">
            {(setup?.teams ?? []).map((team) => {
              const active =
                team.id === (setup?.active_team_id ?? setup?.activeTeamId ?? "");
              return (
                <li key={team.id} className="profile-team-row">
                  <div className="profile-team-meta">
                    <span className="profile-team-name">{teamLabel(team)}</span>
                    {active ? (
                      <span className="muted">{t("profile.currentTeam")}</span>
                    ) : null}
                  </div>
                  {!active ? (
                    <button
                      type="button"
                      className="btn btn-outline btn-sm"
                      disabled={busy}
                      onClick={() => void onSwitchTeam(team.id)}
                    >
                      {busy ? t("profile.switchTeamBusy") : t("profile.switchTeam")}
                    </button>
                  ) : null}
                </li>
              );
            })}
          </ul>
        </fieldset>
      )}

      <fieldset className="card card--flush card--stack profile-group">
        <legend>{t("profile.appVersion")}</legend>
        <div className="profile-grid">
          <span className="profile-label">{t("profile.currentVersion")}</span>
          <span>{currentVer}</span>
          <span className="profile-label">{t("profile.latestVersion")}</span>
          <span>{latestVer}</span>
        </div>
        {update?.status && <p className="muted">{update.status}</p>}
        {progress != null && (
          <div>
            <p className="progress-label">
              {t("profile.downloadProgress", { percent: progress.toFixed(1) })}
            </p>
            <div
              className="progress-track progress progress--determinate"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(progress)}
            >
              <span
                className="progress-fill"
                style={{ width: `${Math.max(0, Math.min(100, progress))}%` }}
              />
            </div>
          </div>
        )}
        <div className="form-actions">
          <button className="btn btn-ghost" type="button" disabled={busy} onClick={() => void onCheckUpdates()}>
            {t("profile.checkUpdates")}
          </button>
          <button
            className="btn"
            type="button"
            disabled={busy || !hasUpdate}
            onClick={() => void onInstallUpdate()}
          >
            {t("common.refresh")}
          </button>
        </div>
        <div className="profile-releases-label">{t("profile.recentReleases")}</div>
        <pre className="mono-panel mono-panel--on-bg profile-releases">
          {update?.releases_text || "—"}
        </pre>
      </fieldset>

      <fieldset className="card card--flush card--stack profile-group">
        <legend>{t("profile.changePassword")}</legend>
        {pwdError && <p className="error">{pwdError}</p>}
        {pwdOk && <p className="ok-text">{pwdOk}</p>}
        <PasswordField
          label={t("profile.currentPassword")}
          value={currentPassword}
          onChange={setCurrentPassword}
          disabled={busy}
        />
        <PasswordField label={t("profile.newPassword")} value={newPassword} onChange={setNewPassword} disabled={busy} />
        <PasswordField
          label={t("profile.confirmNewPassword")}
          value={confirmPassword}
          onChange={setConfirmPassword}
          disabled={busy}
        />
        <button className="btn" type="button" disabled={busy} onClick={() => void onChangePassword()}>
          {t("profile.changePasswordBtn")}
        </button>
      </fieldset>

      <div className="profile-logout">
        <button className="btn btn-danger" type="button" disabled={busy} onClick={() => void onLogout()}>
          {t("profile.logout")}
        </button>
      </div>
    </div>
  );
}
