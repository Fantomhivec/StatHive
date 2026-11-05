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

import { useEffect } from "react";
import { hostApi, setAuthStatusCache } from "../bridge/hostApi";
import { refreshTabsLayout } from "../bridge/hostApi.layout";
import type { UserInfo } from "../bridge/hostApi.types";

/** One interactive OAuth attempt per session (survives StrictMode remount). */
const oauthEnsureStarted = new Set<string>();

/**
 * After login: if Admin / TeamOAuth grant and no cached Google token,
 * open the browser once per session (scripts stall without Sheets OAuth).
 */
export function useEnsureGoogleOauth(opts: {
  enabled: boolean;
  sessionId: string | undefined;
  canCheckGoogle: boolean;
  setUser: (u: UserInfo | null) => void;
}) {
  const { enabled, sessionId, canCheckGoogle, setUser } = opts;

  useEffect(() => {
    if (!enabled || !canCheckGoogle || !sessionId) return;
    if (oauthEnsureStarted.has(sessionId)) return;

    let cancelled = false;
    void (async () => {
      try {
        const setup = await hostApi.authSetupStatus();
        if (cancelled) return;
        if (setup.google_connected) {
          oauthEnsureStarted.add(sessionId);
          return;
        }
        if (cancelled) return;
        oauthEnsureStarted.add(sessionId);
        const u = await hostApi.connectMemberOauth();
        if (cancelled) return;
        setAuthStatusCache(u);
        setUser(u);
        await refreshTabsLayout().catch(() => undefined);
      } catch {
        // Cancelled consent / client not ready — Keys Check stays the manual path.
        // Keep session marked so we do not reopen the browser in a loop.
        oauthEnsureStarted.add(sessionId);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [enabled, sessionId, canCheckGoogle, setUser]);
}
