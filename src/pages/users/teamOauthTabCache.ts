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

/** Кэш снимка вкладки «OAuth команды» — не дергать Sheets при каждом заходе. */

export type TeamOauthGrantSnap = {
  login: string;
  displayName: string;
  allowed: boolean;
  lockedOn: boolean;
};

export type TeamOauthSnap = {
  workspaceId: string;
  hasClient: boolean;
  hint: string | null;
  grants: TeamOauthGrantSnap[];
};

let oauthSnapCache: TeamOauthSnap | null = null;

export function peekTeamOauthSnap(workspaceId: string): TeamOauthSnap | null {
  const ws = workspaceId.trim();
  if (!ws || !oauthSnapCache || oauthSnapCache.workspaceId !== ws) return null;
  return oauthSnapCache;
}

export function setTeamOauthSnap(snap: TeamOauthSnap) {
  oauthSnapCache = snap;
}

export function clearTeamOauthSnap() {
  oauthSnapCache = null;
}
