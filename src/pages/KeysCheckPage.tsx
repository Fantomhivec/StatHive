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

import { lazy, Suspense, useEffect, useState } from "react";
import {
  hostApi,
  hostDialogs,
  KeyCheckLine,
  refreshTabsLayout,
  setAuthStatusCache,
} from "../bridge/hostApi";
import { useAuth } from "../auth/AuthContext";
import { useTabsLayoutCaps } from "../bridge/useTabsLayoutCaps";
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
  const [lines, setLines] = useState<KeyCheckLine[]>([]);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [keysBusy, setKeysBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);
  const [envEditorOpen, setEnvEditorOpen] = useState(false);
  const isMainAdmin = !!layoutCaps.isMainAdmin;
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

  // Mount: keys + quiet Google in parallel (manual OAuth path stays in checkGoogle(false)).
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const keysP = applyCheck();
        const googleP = canCheckGoogle
          ? checkGoogle(true)
          : Promise.resolve(null);
        await Promise.all([keysP, googleP]);
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
                    disabled={googleBusy}
                    title={t("keys.connectMemberOauthHint")}
                    onClick={() => void checkGoogle(false)}
                  >
                    {googleBusy
                      ? t("common.checking")
                      : t("keys.checkGoogleConnection")}
                  </button>
                </div>
                <p className="muted keys-check-hint">{t("keys.connectMemberOauthHint")}</p>
              </>
            ) : (
              <p className="muted keys-check-hint">{t("keys.googleNoAccessHint")}</p>
            )}

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
                      : google?.tone === "warn"
                        ? "keys-check-gs-status warn"
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
