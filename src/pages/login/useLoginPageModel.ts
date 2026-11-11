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

import { useEffect, useState, type SubmitEvent } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { takeAccessRevokedMessage, useAuth } from "../../auth/AuthContext";
import { hostApi, type AuthSetupStatus, type TeamIndexEntry } from "../../bridge/hostApi";
import { useAppDialogs } from "../../components/useAppDialogs";
import { useT } from "../../i18n";
import {
  DEFAULT_ADMIN_LOGIN,
  DEFAULT_ADMIN_NAME,
  DEFAULT_ADMIN_PASSWORD,
  DEFAULT_WORKSPACE_TITLE,
  type Gate,
  pickGate,
  teamLabel,
  teamsOf,
} from "./loginGate";

export function useLoginPageModel() {
  const t = useT();
  const navigate = useNavigate();
  const location = useLocation();
  const { setUser } = useAuth();
  const { confirm, dialogs } = useAppDialogs();
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
  const [workspaceTitle, setWorkspaceTitle] = useState(DEFAULT_WORKSPACE_TITLE);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  function primeRegisterDefaults() {
    setLogin(DEFAULT_ADMIN_LOGIN);
    setPassword(DEFAULT_ADMIN_PASSWORD);
    setDisplayName(DEFAULT_ADMIN_NAME);
    setWorkspaceTitle(DEFAULT_WORKSPACE_TITLE);
  }

  async function refreshSetup(prefer?: Gate) {
    const s = await hostApi.authSetupStatus();
    setSetup(s);
    const next = prefer ?? pickGate(s);
    setGate(next);
    if (next === "admin_register") primeRegisterDefaults();
    if (!s.oauth_configured) setOauthReplacing(false);
  }

  /** Auto-create Admin after successful Connect Google only. */
  async function afterGoogleConnected() {
    const s = await hostApi.authSetupStatus();
    setSetup(s);
    if (!s.can_register_admin || s.workspace_configured) {
      setGate(pickGate(s));
      return;
    }
    try {
      const u = await hostApi.registerAdmin({
        login: DEFAULT_ADMIN_LOGIN,
        password: DEFAULT_ADMIN_PASSWORD,
        display_name: DEFAULT_ADMIN_NAME,
        workspace_title: DEFAULT_WORKSPACE_TITLE,
      });
      setUser(u);
      navigate("/", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      primeRegisterDefaults();
      setGate("admin_register");
    }
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
      await afterGoogleConnected();
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
      await afterGoogleConnected();
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

  async function onDeleteTeam(team: TeamIndexEntry) {
    const name = teamLabel(team);
    if (!(await confirm(t("login.deleteTeamConfirm", { name })))) return;
    setError(null);
    setBusy(true);
    try {
      await hostApi.removeLocalTeam(team.id);
      const s = await hostApi.authSetupStatus();
      setSetup(s);
      if (teamsOf(s).length >= 1) setGate("pick_team");
      else setGate(pickGate(s));
      if (!s.oauth_configured) setOauthReplacing(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function onRegister(e: SubmitEvent<HTMLFormElement>) {
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

  async function onApplyCode(e: SubmitEvent<HTMLFormElement>) {
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

  async function onLogin(e: SubmitEvent<HTMLFormElement>) {
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

  return {
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
  };
}
