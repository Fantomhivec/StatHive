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

import { useCallback, useEffect, useState } from "react";
import { hostApi } from "../../bridge/hostApi";
import { useT } from "../../i18n";
import {
  clearTeamOauthSnap,
  peekTeamOauthSnap,
  setTeamOauthSnap,
  type TeamOauthGrantSnap,
  type TeamOauthSnap,
} from "./teamOauthTabCache";

type Props = {
  workspaceId: string;
  busy: boolean;
  withBusy: (fn: () => Promise<void>) => Promise<void>;
  setError: (msg: string | null) => void;
  setOk: (msg: string | null) => void;
};

/** Подвкладка: лист TeamOAuth на таблице Users + гранты (Admin locked on). */
export default function UsersOauthTab({
  workspaceId,
  busy,
  withBusy,
  setError,
  setOk,
}: Props) {
  const t = useT();
  const ws = workspaceId.trim();
  const cached = peekTeamOauthSnap(ws);
  const [hasClient, setHasClient] = useState(!!cached?.hasClient);
  const [hint, setHint] = useState<string | null>(cached?.hint ?? null);
  const [grants, setGrants] = useState<TeamOauthGrantSnap[]>(cached?.grants ?? []);
  const [loading, setLoading] = useState(!cached);

  const apply = useCallback((snap: TeamOauthSnap) => {
    setTeamOauthSnap(snap);
    setHasClient(snap.hasClient);
    setHint(snap.hint);
    setGrants(snap.grants);
  }, []);

  const load = useCallback(
    async (force: boolean) => {
      if (!force) {
        const hit = peekTeamOauthSnap(ws);
        if (hit) {
          apply(hit);
          return;
        }
      }
      const s = await hostApi.teamOauthGet();
      apply({
        workspaceId: ws,
        hasClient: !!s.hasClient,
        hint: s.clientIdHint ?? null,
        grants: (s.grants ?? []).map((g) => ({
          login: g.login,
          displayName: g.displayName,
          allowed: !!g.allowed,
          lockedOn: !!g.lockedOn,
        })),
      });
    },
    [apply, ws],
  );

  useEffect(() => {
    const hit = peekTeamOauthSnap(ws);
    if (hit) {
      apply(hit);
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    void (async () => {
      try {
        await load(true);
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [ws, apply, load, setError]);

  function toggle(login: string, next: boolean) {
    setGrants((prev) =>
      prev.map((g) => (g.login === login && !g.lockedOn ? { ...g, allowed: next } : g)),
    );
  }

  const uiBusy = busy || loading;

  return (
    <div className="card card--md card--stack users-oauth-tab">
      <p className="muted users-oauth-intro">{t("users.oauthTabIntro")}</p>

      <div className="users-oauth-status">
        <p className="users-oauth-status-text u-m-0">
          {loading
            ? t("common.loading")
            : hasClient
              ? t("users.oauthPublished", { hint: hint || t("common.emDash") })
              : t("users.oauthNotPublished")}
        </p>
        <div className="form-actions u-mt-0">
          <button
            type="button"
            className="btn btn-outline"
            disabled={uiBusy}
            onClick={() =>
              void withBusy(async () => {
                await hostApi.teamOauthPublish();
                setOk(t("users.oauthPublishOk"));
                clearTeamOauthSnap();
                await load(true);
              })
            }
          >
            {t("users.oauthPublish")}
          </button>
          <button
            type="button"
            className="btn btn-ghost"
            disabled={uiBusy}
            onClick={() =>
              void withBusy(async () => {
                clearTeamOauthSnap();
                await load(true);
              })
            }
          >
            {t("common.refresh")}
          </button>
        </div>
      </div>

      <div className="users-oauth-grants">
        <h3 className="users-oauth-grants-title">{t("users.oauthGrantsTitle")}</h3>
        <ul className="users-oauth-grant-list">
          {grants.map((g) => (
            <li key={g.login} className="users-oauth-grant-item">
              <label className="toggle-switch users-oauth-grant-row">
                <input
                  type="checkbox"
                  checked={g.allowed}
                  disabled={uiBusy || g.lockedOn}
                  onChange={(e) => toggle(g.login, e.target.checked)}
                />
                <span className="toggle-track" aria-hidden />
                <span className="users-oauth-grant-name">
                  {g.displayName || g.login}
                  {g.lockedOn ? ` (${t("users.oauthAdminDefault")})` : ""}
                </span>
              </label>
            </li>
          ))}
        </ul>
        <div className="form-actions">
          <button
            type="button"
            className="btn btn-primary"
            disabled={uiBusy}
            onClick={() =>
              void withBusy(async () => {
                const allowed = grants.filter((g) => g.allowed && !g.lockedOn).map((g) => g.login);
                await hostApi.teamOauthSetGrants(allowed);
                setOk(t("users.oauthGrantsSaved"));
                clearTeamOauthSnap();
                await load(true);
              })
            }
          >
            {t("users.oauthSaveGrants")}
          </button>
        </div>
      </div>
    </div>
  );
}
