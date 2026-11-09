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

import { useT } from "../../i18n";
import { LoginOauthGuidePanel } from "./LoginOauthGuidePanel";

type Props = {
  busy: boolean;
  showOauthForm: boolean;
  oauthConfigured: boolean;
  oauthReplacing: boolean;
  oauthJson: string;
  oauthClientHint: string;
  googleConnected: boolean;
  googleEmail: string;
  onOauthJsonChange: (value: string) => void;
  onSaveOauth: () => void;
  onConnectGoogle: () => void;
  onStartReplace: () => void;
  onCancelReplace: () => void;
  onBack: () => void;
};

export function LoginOauthSection({
  busy,
  showOauthForm,
  oauthConfigured,
  oauthReplacing,
  oauthJson,
  oauthClientHint,
  googleConnected,
  googleEmail,
  onOauthJsonChange,
  onSaveOauth,
  onConnectGoogle,
  onStartReplace,
  onCancelReplace,
  onBack,
}: Props) {
  const t = useT();
  return (
    <div className="login-stack login-stack--oauth">
      <div className="login-oauth-layout">
        <LoginOauthGuidePanel />

        <div className="login-oauth-side">
          {showOauthForm ? (
            <>
              <p className="muted login-oauth-side-hint">{t("login.oauthJsonHint")}</p>
              <div className="field field-flush">
                <label htmlFor="oauth-json">OAuth client JSON</label>
                <textarea
                  id="oauth-json"
                  className="input input-mono login-oauth-json"
                  value={oauthJson}
                  onChange={(e) => onOauthJsonChange(e.target.value)}
                  disabled={busy}
                  rows={6}
                  placeholder='{"installed":{"client_id":"...","client_secret":"..."}}'
                />
              </div>
              <div className="form-actions">
                <button
                  className="btn btn-primary"
                  type="button"
                  disabled={busy || !oauthJson.trim()}
                  onClick={onSaveOauth}
                >
                  {t("login.saveAndConnectGoogle")}
                </button>
                {oauthReplacing ? (
                  <button
                    type="button"
                    className="btn btn-ghost"
                    disabled={busy}
                    onClick={onCancelReplace}
                  >
                    {t("common.cancel")}
                  </button>
                ) : null}
              </div>
            </>
          ) : (
            <div className="wb-card login-oauth-current">
              <p className="muted u-m-0">{t("login.oauthSavedHint")}</p>
              {oauthClientHint ? (
                <p className="mono u-m-0">{t("login.oauthCurrentClient", { id: oauthClientHint })}</p>
              ) : null}
              <p className="muted u-m-0">
                {googleConnected && googleEmail.trim()
                  ? t("login.oauthCurrentGoogle", { email: googleEmail.trim() })
                  : t("login.oauthCurrentGoogleNone")}
              </p>
              {!googleConnected ? (
                <button
                  className="btn btn-primary"
                  type="button"
                  disabled={busy}
                  onClick={onConnectGoogle}
                >
                  {t("login.connectGoogle")}
                </button>
              ) : null}
            </div>
          )}
        </div>
      </div>

      <div className="login-oauth-footer form-actions">
        {oauthConfigured && !oauthReplacing ? (
          <button type="button" className="btn btn-outline" disabled={busy} onClick={onStartReplace}>
            {t("login.newOauth")}
          </button>
        ) : null}
        <button type="button" className="btn btn-ghost" disabled={busy} onClick={onBack}>
          {t("common.back")}
        </button>
      </div>
    </div>
  );
}
