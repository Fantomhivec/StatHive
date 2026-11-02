import { useState } from "react";
import { Link, Navigate } from "react-router-dom";
import { openUrl } from "../../bridge/hostApi";
import { shortTeamName } from "../../shared/workspaceTitle";
import "../UsersPage.css";
import UsersInviteDialog from "./UsersInviteDialog";
import UsersOauthTab from "./UsersOauthTab";
import UsersPresetsTab from "./UsersPresetsTab";
import UsersSaPanel from "./UsersSaPanel";
import UsersTeamNameDialog from "./UsersTeamNameDialog";
import UsersTeamTable from "./UsersTeamTable";
import { DISCORD_COMMUNITY_NAME, DISCORD_INVITE_URL } from "./usersPageCache";
import { useUsersPageModel } from "./useUsersPageModel";
import type { UsersTab } from "./useUsersPageModel";

export default function UsersPage() {
  const m = useUsersPageModel();
  const { t } = m;
  const [inviteOpen, setInviteOpen] = useState(false);
  const [teamNameOpen, setTeamNameOpen] = useState(false);

  if (m.me === undefined || !m.gateReady) {
    return <div className="users-page muted">{t("common.loading")}</div>;
  }
  if (!m.me?.is_authorized || !m.canUsersTab) {
    return <Navigate to="/" replace />;
  }

  const teamShort = shortTeamName(m.workspaceTitle) || m.workspaceTitle.trim();
  const inviteDisabled =
    m.busy || !m.saConfigured || m.presets.length === 0 || m.calcConfigs.length === 0;
  const inviteTitle = !m.saConfigured
    ? t("users.inviteNeedsSa")
    : m.presets.length === 0
      ? t("users.noPresetsInvite")
      : m.calcConfigs.length === 0
        ? t("users.noCalcConfigsOption")
        : undefined;

  return (
    <>
      <div className="users-page">
        <div className="users-header">
          <h1 className="page-heading">{t("users.title")}</h1>
          <div className="form-actions users-header-actions">
            {m.isMainAdmin ? (
              <button
                className="btn btn-outline"
                type="button"
                disabled={m.busy || !m.workspaceId}
                title={
                  m.workspaceId
                    ? t("users.openSheetTitle")
                    : t("users.noWorkspaceId")
                }
                onClick={() => void m.onOpenWorkspaceSheet()}
              >
                {t("users.openSheet")}
              </button>
            ) : null}
            <button
              className="btn btn-outline"
              type="button"
              disabled={m.busy}
              title={t("users.discordTitle", { name: DISCORD_COMMUNITY_NAME })}
              onClick={() => void m.withBusy(() => openUrl(DISCORD_INVITE_URL))}
            >
              {t("users.discordBtn", { name: DISCORD_COMMUNITY_NAME })}
            </button>
            {m.isMainAdmin ? (
              <button
                className="btn btn-outline"
                type="button"
                disabled={m.busy}
                onClick={() => void m.onCopyTeamCode()}
              >
                {t("users.copyTeamCode")}
              </button>
            ) : null}
            <button
              className="btn btn-ghost"
              type="button"
              disabled={m.busy}
              onClick={() => void m.withBusy(() => m.loadAll(true))}
            >
              {t("users.refreshFromSheets")}
            </button>
          </div>
        </div>
        <p className="muted users-lead">
          {t("users.intro")}{" "}
          <Link to="/logs">{t("users.introLogsLink")}</Link>.
        </p>

        <div className="users-tabs">
          {(
            [
              ["users", t("profile.teamSection")],
              ["presets", t("users.tabPresets")],
              ...(m.isMainAdmin
                ? ([["oauth", t("users.tabOauth")]] as const)
                : []),
            ] as Array<readonly [UsersTab, string]>
          ).map(([k, label]) => (
            <button
              key={k}
              type="button"
              className={`btn btn-ghost btn-segment${m.tab === k ? " is-active" : ""}`}
              disabled={m.busy}
              onClick={() => m.setTab(k)}
            >
              {label}
            </button>
          ))}
        </div>

        {m.error && <p className="error">{m.error}</p>}
        {m.ok && <p className="ok-text">{m.ok}</p>}

        {m.tab === "users" && (
          <>
            {m.isMainAdmin ? (
              <div className="card card--md card--stack users-team-toolbar">
                <div className="users-team-toolbar-row">
                  <div className="users-team-toolbar-meta">
                    <span className="users-team-toolbar-label">{t("users.teamNameTitle")}</span>
                    <span className="users-team-toolbar-name">
                      {teamShort || t("common.emDash")}
                    </span>
                  </div>
                  <button
                    type="button"
                    className="btn btn-outline btn-sm"
                    disabled={m.busy || !m.workspaceId}
                    onClick={() => {
                      m.setTeamNameDraft(teamShort || "StatHive");
                      setTeamNameOpen(true);
                    }}
                  >
                    {t("users.teamNameEdit")}
                  </button>
                </div>
                <UsersSaPanel
                  busy={m.busy}
                  workspaceId={m.workspaceId}
                  saConfigured={m.saConfigured}
                  saEmail={m.saEmail}
                  saJson={m.saJson}
                  onSaJsonChange={m.setSaJson}
                  onCreate={() => void m.onCreateServiceAccount()}
                  onReplace={() => void m.onReplaceServiceAccount()}
                  onSave={(e) => void m.onSaveServiceAccount(e)}
                />
              </div>
            ) : null}

            <div className="users-table-actions">
              <button
                type="button"
                className="btn btn-primary"
                disabled={inviteDisabled}
                title={inviteTitle}
                onClick={() => setInviteOpen(true)}
              >
                {t("users.inviteOpen")}
              </button>
            </div>

            <UsersTeamTable
              team={m.team}
              busy={m.busy}
              isMainAdmin={m.isMainAdmin}
              presets={m.presets}
              calcConfigs={m.calcConfigs}
              presetOptions={m.presetOptions}
              calcConfigOptions={m.calcConfigOptions}
              onSetPreset={m.onSetPreset}
              onSetCalcConfig={m.onSetCalcConfig}
              onDeleteUser={m.onDeleteUser}
            />
          </>
        )}

        {m.tab === "presets" && (
          <UsersPresetsTab
            busy={m.busy}
            presets={m.presets}
            catalog={m.catalog}
            onSavePreset={m.onSavePreset}
            onDeletePreset={m.onDeletePreset}
          />
        )}

        {m.tab === "oauth" && m.isMainAdmin ? (
          <UsersOauthTab
            busy={m.busy}
            withBusy={m.withBusy}
            setError={m.setError}
            setOk={m.setOk}
          />
        ) : null}
      </div>

      <UsersInviteDialog
        open={inviteOpen}
        onClose={() => setInviteOpen(false)}
        busy={m.busy}
        saConfigured={m.saConfigured}
        presetsEmpty={m.presets.length === 0}
        calcConfigsEmpty={m.calcConfigs.length === 0}
        name={m.name}
        login={m.login}
        password={m.password}
        email={m.email}
        invitePreset={m.invitePreset}
        inviteCalcConfig={m.inviteCalcConfig}
        presetOptions={m.presetOptions}
        calcConfigOptions={m.calcConfigOptions}
        onNameChange={m.setName}
        onLoginChange={m.setLogin}
        onPasswordChange={m.setPassword}
        onEmailChange={m.setEmail}
        onInvitePresetChange={m.setInvitePreset}
        onInviteCalcConfigChange={m.setInviteCalcConfig}
        onSubmit={(e) => void m.onInvite(e, () => setInviteOpen(false))}
      />

      <UsersTeamNameDialog
        open={teamNameOpen}
        onClose={() => setTeamNameOpen(false)}
        busy={m.busy}
        workspaceId={m.workspaceId}
        workspaceTitle={m.workspaceTitle}
        teamNameDraft={m.teamNameDraft}
        onTeamNameDraftChange={m.setTeamNameDraft}
        onSubmit={(e) => void m.onSaveTeamName(e, () => setTeamNameOpen(false))}
      />

      {m.dialogs}
    </>
  );
}
