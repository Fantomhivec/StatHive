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

import { FormEvent, useState } from "react";
import { useT } from "../../i18n";

export type UsersSaPanelProps = {
  busy: boolean;
  workspaceId: string | null;
  saConfigured: boolean;
  saEmail: string | null;
  saJson: string;
  onSaJsonChange: (value: string) => void;
  onCreate: () => void;
  onReplace: () => void;
  onSave: (e: FormEvent) => void;
};

export default function UsersSaPanel({
  busy,
  workspaceId,
  saConfigured,
  saEmail,
  saJson,
  onSaJsonChange,
  onCreate,
  onReplace,
  onSave,
}: UsersSaPanelProps) {
  const t = useT();
  const [manualOpen, setManualOpen] = useState(false);

  return (
    <div className="users-sa-compact">
      <div className="users-sa-compact-row">
        <div className="users-sa-compact-meta">
          <span className="users-sa-compact-label">{t("users.serviceAccountTitle")}</span>
          {saConfigured ? (
            <span className="ok-text u-m-0">
              {t("users.serviceAccountOk", { email: saEmail || "—" })}
            </span>
          ) : (
            <span className="error u-m-0">{t("users.serviceAccountMissing")}</span>
          )}
          <p className="muted calc-hint u-m-0">{t("users.serviceAccountPerTeamHint")}</p>
        </div>
        <div className="users-sa-compact-actions">
          {!saConfigured ? (
            <button
              className="btn btn-primary btn-sm"
              type="button"
              disabled={busy || !workspaceId}
              title={
                workspaceId
                  ? t("users.serviceAccountCreateTitle")
                  : t("users.errNoWorkspace")
              }
              onClick={onCreate}
            >
              {t("users.serviceAccountCreate")}
            </button>
          ) : (
            <button
              className="btn btn-danger btn-sm"
              type="button"
              disabled={busy || !workspaceId}
              title={t("users.serviceAccountReplaceTitle")}
              onClick={onReplace}
            >
              {t("users.serviceAccountReplace")}
            </button>
          )}
          <button
            type="button"
            className="btn btn-ghost btn-sm"
            disabled={busy}
            onClick={() => setManualOpen((v) => !v)}
          >
            {manualOpen ? t("common.cancel") : t("users.saManualJson")}
          </button>
        </div>
      </div>
      {manualOpen ? (
        <form className="users-sa-manual field-flush" onSubmit={onSave}>
          <div className="field">
            <label htmlFor="u-sa-json">{t("users.serviceAccountJson")}</label>
            <textarea
              id="u-sa-json"
              className="input input-mono users-code-ta"
              rows={3}
              value={saJson}
              onChange={(e) => onSaJsonChange(e.target.value)}
              disabled={busy}
              placeholder='{"type":"service_account",...}'
            />
          </div>
          <div className="form-actions users-form-actions">
            <button
              className="btn btn-outline btn-sm"
              type="submit"
              disabled={busy || !saJson.trim()}
            >
              {t("users.serviceAccountSave")}
            </button>
          </div>
        </form>
      ) : null}
    </div>
  );
}
