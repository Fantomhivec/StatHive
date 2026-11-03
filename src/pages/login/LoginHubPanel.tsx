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

import { openUrl } from "../../bridge/hostApi";
import { DISCORD_COMMUNITY_NAME, DISCORD_INVITE_URL } from "./community";
import { useT } from "../../i18n";

type HubProps = {
  busy: boolean;
  onExisting: () => void;
  onTeamCode: () => void;
  onNewOauth: () => void;
};

export function LoginHubPanel({ busy, onExisting, onTeamCode, onNewOauth }: HubProps) {
  const t = useT();
  return (
    <div className="login-stack">
      <button type="button" className="btn btn-primary" disabled={busy} onClick={onExisting}>
        {t("login.hubExisting")}
      </button>
      <button type="button" className="btn btn-outline" disabled={busy} onClick={onTeamCode}>
        {t("login.hubTeamCode")}
      </button>
      <button type="button" className="btn btn-outline" disabled={busy} onClick={onNewOauth}>
        {t("login.hubNewOauth")}
      </button>
      <DiscordButton busy={busy} />
    </div>
  );
}

type ChooseProps = {
  busy: boolean;
  workspaceConfigured: boolean;
  onGoSignIn: () => void;
  onIAmAdmin: () => void;
  onIAmMember: () => void;
};

export function LoginChoosePanel({
  busy,
  workspaceConfigured,
  onGoSignIn,
  onIAmAdmin,
  onIAmMember,
}: ChooseProps) {
  const t = useT();
  return (
    <div className="login-stack">
      {workspaceConfigured ? (
        <>
          <p className="muted">{t("login.workspaceReadyHint")}</p>
          <button type="button" className="btn btn-primary" disabled={busy} onClick={onGoSignIn}>
            {t("login.goToSignIn")}
          </button>
          <button type="button" className="btn btn-outline" disabled={busy} onClick={onIAmAdmin}>
            {t("login.iAmAdmin")}
          </button>
        </>
      ) : (
        <>
          <p className="muted">{t("login.whoAreYou")}</p>
          <button type="button" className="btn btn-primary" disabled={busy} onClick={onIAmAdmin}>
            {t("login.iAmAdmin")}
          </button>
          <button type="button" className="btn btn-outline" disabled={busy} onClick={onIAmMember}>
            {t("login.iAmMember")}
          </button>
        </>
      )}
      <DiscordButton busy={busy} />
    </div>
  );
}

function DiscordButton({ busy }: { busy: boolean }) {
  const t = useT();
  return (
    <button
      type="button"
      className="btn btn-outline"
      disabled={busy}
      title={t("users.discordTitle", { name: DISCORD_COMMUNITY_NAME })}
      onClick={() => void openUrl(DISCORD_INVITE_URL)}
    >
      {t("users.discordBtn", { name: DISCORD_COMMUNITY_NAME })}
    </button>
  );
}
