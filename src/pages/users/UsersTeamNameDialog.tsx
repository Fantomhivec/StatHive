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

import { FormEvent } from "react";
import AppCustomWindow from "../../components/AppCustomWindow";
import { useT } from "../../i18n";
import { shortTeamName } from "../../bridge/workspaceTitle";

const FORM_ID = "users-team-name-dialog-form";

type Props = {
  open: boolean;
  busy: boolean;
  workspaceId: string | null;
  workspaceTitle: string;
  teamNameDraft: string;
  onTeamNameDraftChange: (value: string) => void;
  onSubmit: (e: FormEvent) => void;
  onClose: () => void;
};

export default function UsersTeamNameDialog({
  open,
  busy,
  workspaceId,
  workspaceTitle,
  teamNameDraft,
  onTeamNameDraftChange,
  onSubmit,
  onClose,
}: Props) {
  const t = useT();
  if (!open) return null;

  const short = shortTeamName(workspaceTitle);

  return (
    <AppCustomWindow
      title={t("users.teamNameTitle")}
      onClose={onClose}
      width={420}
      bodyClassName="users-dialog-body"
      footer={
        <>
          <button type="button" className="btn btn-ghost" disabled={busy} onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button
            className="btn btn-primary"
            type="submit"
            form={FORM_ID}
            disabled={busy || !workspaceId || !teamNameDraft.trim()}
          >
            {t("users.teamNameSave")}
          </button>
        </>
      }
    >
      <form id={FORM_ID} className="users-dialog-stack field-flush" onSubmit={onSubmit}>
        <p className="muted u-m-0">{t("users.teamNameHint")}</p>
        {short ? (
          <p className="ok-text u-m-0">{t("users.teamNameCurrent", { name: short })}</p>
        ) : null}
        <div className="field">
          <label htmlFor="u-team-name-dlg">{t("users.teamNameLabel")}</label>
          <input
            id="u-team-name-dlg"
            className="input"
            value={teamNameDraft}
            onChange={(e) => onTeamNameDraftChange(e.target.value)}
            disabled={busy || !workspaceId}
            placeholder="StatHive"
            autoComplete="off"
            autoFocus
          />
        </div>
      </form>
    </AppCustomWindow>
  );
}
