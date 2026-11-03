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

import type { AuthSetupStatus, TeamIndexEntry } from "../../bridge/hostApi";
import { shortTeamName } from "../../bridge/workspaceTitle";

export type Gate =
  | "hub"
  | "choose"
  | "pick_team"
  | "admin_oauth"
  | "admin_register"
  | "member_code"
  | "login";

/** Defaults after Connect Google; Admin may change them on the register form. */
export const DEFAULT_ADMIN_LOGIN = "Admin";
export const DEFAULT_ADMIN_PASSWORD = "Admin123";
export const DEFAULT_ADMIN_NAME = "Admin";
export const DEFAULT_WORKSPACE_TITLE = "Команда";

export function teamsOf(s: AuthSetupStatus): TeamIndexEntry[] {
  return s.teams ?? [];
}

export function pickGate(s: AuthSetupStatus): Gate {
  // Incomplete Admin slot (incl. hub → «Новый OAuth»): never login/hub until workspace exists.
  if (s.oauth_configured && s.can_register_admin && !s.workspace_configured) {
    return s.google_connected ? "admin_register" : "admin_oauth";
  }
  if (teamsOf(s).length >= 1) return "hub";
  if (s.oauth_configured && s.workspace_configured) return "login";
  return "choose";
}

export function teamLabel(t: TeamIndexEntry): string {
  const title = (t.title ?? "").trim();
  if (title) return shortTeamName(title) || title;
  return t.id;
}
