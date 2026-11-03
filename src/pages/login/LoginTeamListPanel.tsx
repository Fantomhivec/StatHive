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

import type { TeamIndexEntry } from "../../bridge/hostApi";
import { useT } from "../../i18n";
import { teamLabel } from "./loginGate";

type Props = {
  busy: boolean;
  teams: TeamIndexEntry[];
  activeId: string;
  onPick: (id: string) => void;
  onDelete: (team: TeamIndexEntry) => void;
  onBack: () => void;
};

export function LoginTeamListPanel({ busy, teams, activeId, onPick, onDelete, onBack }: Props) {
  const t = useT();
  return (
    <div className="login-stack">
      <p className="muted">{t("login.pickTeamHint")}</p>
      {teams.length === 0 ? (
        <p className="muted">{t("login.pickTeamEmpty")}</p>
      ) : (
        <ul className="login-team-list">
          {teams.map((team) => (
            <li key={team.id} className="login-team-row">
              <div className="login-team-row-inner">
                <button
                  type="button"
                  className="btn btn-outline login-team-pick"
                  disabled={busy}
                  onClick={() => onPick(team.id)}
                >
                  <span className="login-team-name">{teamLabel(team)}</span>
                  {team.id === activeId ? (
                    <span className="muted login-team-badge">{t("login.activeTeamMark")}</span>
                  ) : null}
                </button>
                <button
                  type="button"
                  className="btn btn-danger btn-sm"
                  disabled={busy}
                  onClick={() => onDelete(team)}
                >
                  {t("login.deleteTeam")}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
      <button type="button" className="btn btn-ghost" disabled={busy} onClick={onBack}>
        {t("login.backToHub")}
      </button>
    </div>
  );
}
