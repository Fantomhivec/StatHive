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
