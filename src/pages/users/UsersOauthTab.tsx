import { useCallback, useEffect, useState } from "react";
import { hostApi } from "../../bridge/hostApi";
import { useT } from "../../i18n";

type Grant = {
  login: string;
  displayName: string;
  allowed: boolean;
  lockedOn: boolean;
};

type Props = {
  busy: boolean;
  withBusy: (fn: () => Promise<void>) => Promise<void>;
  setError: (msg: string | null) => void;
  setOk: (msg: string | null) => void;
};

/** Подвкладка: лист TeamOAuth на таблице Users + гранты (Admin locked on). */
export default function UsersOauthTab({ busy, withBusy, setError, setOk }: Props) {
  const t = useT();
  const [hasClient, setHasClient] = useState(false);
  const [hint, setHint] = useState<string | null>(null);
  const [grants, setGrants] = useState<Grant[]>([]);

  const load = useCallback(async () => {
    const s = await hostApi.teamOauthGet();
    setHasClient(!!s.hasClient);
    setHint(s.clientIdHint ?? null);
    setGrants(
      (s.grants ?? []).map((g) => ({
        login: g.login,
        displayName: g.displayName,
        allowed: !!g.allowed,
        lockedOn: !!g.lockedOn,
      })),
    );
  }, []);

  useEffect(() => {
    void withBusy(async () => {
      try {
        await load();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    });
  }, [load, setError, withBusy]);

  function toggle(login: string, next: boolean) {
    setGrants((prev) =>
      prev.map((g) => (g.login === login && !g.lockedOn ? { ...g, allowed: next } : g)),
    );
  }

  return (
    <div className="card card--md card--stack users-oauth-tab">
      <p className="muted users-oauth-intro">{t("users.oauthTabIntro")}</p>

      <div className="users-oauth-status">
        <p className="users-oauth-status-text u-m-0">
          {hasClient
            ? t("users.oauthPublished", { hint: hint || t("common.emDash") })
            : t("users.oauthNotPublished")}
        </p>
        <div className="form-actions u-mt-0">
          <button
            type="button"
            className="btn btn-outline"
            disabled={busy}
            onClick={() =>
              void withBusy(async () => {
                await hostApi.teamOauthPublish();
                setOk(t("users.oauthPublishOk"));
                await load();
              })
            }
          >
            {t("users.oauthPublish")}
          </button>
          <button
            type="button"
            className="btn btn-ghost"
            disabled={busy}
            onClick={() =>
              void withBusy(async () => {
                await load();
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
                  disabled={busy || g.lockedOn}
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
            disabled={busy}
            onClick={() =>
              void withBusy(async () => {
                const allowed = grants.filter((g) => g.allowed && !g.lockedOn).map((g) => g.login);
                await hostApi.teamOauthSetGrants(allowed);
                setOk(t("users.oauthGrantsSaved"));
                await load();
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
