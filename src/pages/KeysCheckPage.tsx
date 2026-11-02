import { lazy, Suspense, useEffect, useState } from "react";
import {
  hostApi,
  hostDialogs,
  KeyCheckLine,
  refreshTabsLayout,
  setAuthStatusCache,
} from "../bridge/hostApi";
import { useAuth } from "../auth/AuthContext";
import { useTabsLayoutCaps } from "../hooks/useTabsLayoutCaps";
import "./KeysCheckPage.css";
import { useT } from "../i18n";

const EnvKeysEditorDialog = lazy(() => import("../components/EnvKeysEditorDialog"));

type GoogleStatus = {
  success: boolean;
  status: string;
  message: string;
  time: string;
  tone: string;
};

export default function KeysCheckPage() {
  const t = useT();
  const { setUser } = useAuth();
  const layoutCaps = useTabsLayoutCaps();
  const [path, setPath] = useState("");
  const [envExists, setEnvExists] = useState(true);
  const [google, setGoogle] = useState<GoogleStatus | null>(null);
  const [googleBusy, setGoogleBusy] = useState(false);
  const [oauthBusy, setOauthBusy] = useState(false);
  const [lines, setLines] = useState<KeyCheckLine[]>([]);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [keysBusy, setKeysBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);
  const [envEditorOpen, setEnvEditorOpen] = useState(false);
  const [oauthJson, setOauthJson] = useState("");
  const isMainAdmin = !!layoutCaps.isMainAdmin;
  const canEditOauthClient = isMainAdmin;
  const canCheckGoogle = !!layoutCaps.canCheckGoogle || isMainAdmin;

  async function applyCheck() {
    const res = await hostApi.envCheckKeys();
    setPath(res.path || "");
    setEnvExists(res.exists !== false);
    setLines(res.lines ?? []);
    setWarnings(res.warnings ?? []);
    // Надёжно обновить нав (Шаблоны/Логи): не полагаться только на Tauri-событие.
    await refreshTabsLayout().catch(() => undefined);
  }

  async function checkGoogle(auto: boolean): Promise<GoogleStatus> {
    setGoogleBusy(true);
    setError(null);
    try {
      let g = await hostApi.googleCheckConnection(undefined, auto);
      // Ручная проверка: нет токена → подтянуть client команды (грант) + браузер Google.
      if (!auto && !g.success) {
        try {
          const u = await hostApi.connectMemberOauth();
          setAuthStatusCache(u);
          setUser(u);
          await refreshTabsLayout();
          g = await hostApi.googleCheckConnection(undefined, true);
        } catch (connectErr) {
          setOk(null);
          setError(
            connectErr instanceof Error ? connectErr.message : String(connectErr),
          );
          setGoogle(g);
          return g;
        }
      }
      setGoogle(g);
      if (!g.success) setOk(null);
      else if (!auto) setOk(t("keys.connectMemberOauthOk"));
      return g;
    } catch (e) {
      setOk(null);
      const fail: GoogleStatus = {
        success: false,
        status: t("common.error"),
        message: e instanceof Error ? e.message : String(e),
        time: new Date().toLocaleTimeString(),
        tone: "danger",
      };
      setGoogle(fail);
      return fail;
    } finally {
      setGoogleBusy(false);
    }
  }

  async function saveOwnOauthJson() {
    setOauthBusy(true);
    setError(null);
    setOk(null);
    try {
      await hostApi.saveOauthClient(oauthJson);
      setOauthJson("");
      setOk(t("keys.oauthPasteOk"));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setOauthBusy(false);
    }
  }

  // При заходе: ключи + тихая проверка Google (только если есть client/грант).
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        await applyCheck();
        if (cancelled) return;
        if (canCheckGoogle) {
          await checkGoogle(true);
        }
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : String(e));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [canCheckGoogle]);

  return (
    <div className="keys-check-page">
      <h1 className="page-title">{t("keys.title")}</h1>
      {error && <p className="error">{error}</p>}
      {ok && <p className="ok-text">{ok}</p>}

      <div className="keys-check-grid">
        <section className="keys-check-col">
          <h2>{t("keys.googleSection")}</h2>
          <div className="card card--stack keys-check-card">
            {canCheckGoogle ? (
              <>
                <div className="form-actions u-mt-0">
                  <button
                    className="btn btn-primary"
                    type="button"
                    disabled={googleBusy || oauthBusy}
                    title={t("keys.connectMemberOauthHint")}
                    onClick={() => void checkGoogle(false)}
                  >
                    {googleBusy || oauthBusy
                      ? t("common.checking")
                      : t("keys.checkGoogleConnection")}
                  </button>
                </div>
                <p className="muted keys-check-hint">{t("keys.connectMemberOauthHint")}</p>
              </>
            ) : (
              <p className="muted keys-check-hint">{t("keys.googleNoAccessHint")}</p>
            )}

            {canEditOauthClient ? (
              <div className="keys-check-oauth-paste">
                <h3>{t("keys.oauthPasteTitle")}</h3>
                <p className="muted keys-check-hint">{t("keys.oauthPasteHint")}</p>
                <div className="field field-flush">
                  <label htmlFor="keys-oauth-json">{t("keys.oauthPasteLabel")}</label>
                  <textarea
                    id="keys-oauth-json"
                    className="input input-mono"
                    rows={4}
                    value={oauthJson}
                    disabled={oauthBusy}
                    onChange={(e) => setOauthJson(e.target.value)}
                    placeholder='{"installed":{…}}'
                  />
                </div>
                <div className="form-actions">
                  <button
                    type="button"
                    className="btn btn-outline"
                    disabled={oauthBusy || !oauthJson.trim()}
                    onClick={() => void saveOwnOauthJson()}
                  >
                    {t("keys.oauthPasteSave")}
                  </button>
                </div>
              </div>
            ) : null}

            <hr className="keys-check-sep" />

            <div className="keys-check-wb">
              <h3>{t("keys.wbCabinets")}</h3>
            <div className="plate plate--pad keys-check-hint">
              <div className="keys-check-hint-title">{t("keys.instructionTitle")}</div>
              <p>{t("keys.instructionWhere")}</p>
              <p>{t("keys.instructionManual")}</p>
              <p>{t("keys.instructionEditor")}</p>
            </div>

              <div className="keys-check-env-row">
                <input className="input input-compact" value={path} readOnly title={path} />
                <button
                  className="btn btn-outline btn-sm keys-check-env-pick"
                  type="button"
                  disabled={keysBusy}
                  onClick={() =>
                    void (async () => {
                      setKeysBusy(true);
                      setError(null);
                      try {
                        const picked = await hostDialogs.pickEnvFile();
                        if (!picked) return;
                        const res = await hostApi.envSetPath(picked);
                        setPath(res.path);
                        await applyCheck();
                      } catch (e) {
                        setError(e instanceof Error ? e.message : String(e));
                      } finally {
                        setKeysBusy(false);
                      }
                    })()
                  }
                >
                  {t("keys.selectEnvFile")}
                </button>
              </div>

              <div className="form-actions">
                <button
                  className="btn btn-outline btn-sm"
                  type="button"
                  disabled={keysBusy}
                  onClick={() =>
                    void (async () => {
                      setKeysBusy(true);
                      try {
                        const res = await hostApi.envCreateDefault();
                        setPath(res.path);
                        await applyCheck();
                      } catch (e) {
                        setError(e instanceof Error ? e.message : String(e));
                      } finally {
                        setKeysBusy(false);
                      }
                    })()
                  }
                >
                  {t("keys.createKeysFile")}
                </button>
                <button
                  className="btn btn-outline btn-sm"
                  type="button"
                  disabled={keysBusy}
                  onClick={() => setEnvEditorOpen(true)}
                >
                  {t("keys.keysEditor")}
                </button>
                <button
                  className="btn btn-primary btn-sm"
                  type="button"
                  disabled={keysBusy}
                  onClick={() =>
                    void (async () => {
                      setKeysBusy(true);
                      setError(null);
                      try {
                        await applyCheck();
                      } catch (e) {
                        setError(e instanceof Error ? e.message : String(e));
                      } finally {
                        setKeysBusy(false);
                      }
                    })()
                  }
                >
                  {t("keys.checkKeys")}
                </button>
              </div>
            </div>
          </div>
        </section>

        <section className="keys-check-col">
          <h2>{t("keys.statusSection")}</h2>
          <div className="card card--stack keys-check-card">
            <div
              className={
                googleBusy
                  ? "keys-check-gs-status info"
                  : google?.tone === "success"
                    ? "keys-check-gs-status ok-text"
                    : google?.tone === "danger"
                      ? "keys-check-gs-status danger"
                      : "keys-check-gs-status"
              }
            >
              {googleBusy ? t("common.checking") : google?.status || t("keys.notChecked")}
            </div>
            {google && (
              <div className="keys-check-gs-details">
                <hr className="keys-check-sep" />
                <div>{google.message}</div>
                <div className="muted-note u-mt-6">
                  {google.time}
                </div>
              </div>
            )}

            <hr className="keys-check-sep" />
            <h3 className="keys-check-keys-heading">{t("common.cabinets")}</h3>
            {warnings.map((w) => (
              <p key={w} className="keys-check-warn">
                {w}
              </p>
            ))}
            <div className="keys-check-lines">
              {!envExists ? (
                <div className="muted-note">{t("keys.envNotFound")}</div>
              ) : lines.length === 0 ? (
                <div className="muted-note">{t("keys.keysNotFound")}</div>
              ) : (
                lines.map((line) => (
                  <div key={line.index} className={`keys-check-line tone-${line.tone}`}>
                    {line.text}
                  </div>
                ))
              )}
            </div>
          </div>
        </section>
      </div>

      {envEditorOpen ? (
        <Suspense fallback={null}>
          <EnvKeysEditorDialog
            onClose={() => setEnvEditorOpen(false)}
            onSaved={() => {
              void applyCheck().catch((e) => setError(String(e)));
            }}
          />
        </Suspense>
      ) : null}
    </div>
  );
}
