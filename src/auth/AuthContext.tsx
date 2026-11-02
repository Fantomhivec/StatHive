import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  clearAuthStatusCache,
  ensureAuthStatus,
  setAuthStatusCache,
} from "../bridge/hostApi.auth";
import { onEvent } from "../bridge/hostApi.events";
import {
  clearTabsLayoutCache,
  refreshTabsLayout,
} from "../bridge/hostApi.layout";
import type { UserInfo } from "../bridge/hostApi.types";

const ACCESS_REVOKED_KEY = "auth.accessRevoked";

export function takeAccessRevokedMessage(): string | null {
  try {
    const v = sessionStorage.getItem(ACCESS_REVOKED_KEY);
    if (!v) return null;
    sessionStorage.removeItem(ACCESS_REVOKED_KEY);
    return v;
  } catch {
    return null;
  }
}

type AuthContextValue = {
  /** undefined = ещё грузим; null = нет сессии */
  user: UserInfo | null | undefined;
  setUser: (u: UserInfo | null) => void;
  refresh: () => Promise<UserInfo | null>;
};

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUserState] = useState<UserInfo | null | undefined>(undefined);

  const setUser = useCallback((u: UserInfo | null) => {
    setAuthStatusCache(u);
    setUserState(u);
  }, []);

  const refresh = useCallback(async () => {
    clearAuthStatusCache();
    const u = await ensureAuthStatus();
    setUserState(u);
    return u;
  }, []);

  useEffect(() => {
    void ensureAuthStatus()
      .then(setUserState)
      .catch(() => setUserState(null));
  }, []);

  useEffect(() => {
    return onEvent<{ reason?: string }>("auth.accessRevoked", (payload) => {
      const reason =
        typeof payload?.reason === "string" && payload.reason.trim()
          ? payload.reason.trim()
          : "";
      try {
        sessionStorage.setItem(ACCESS_REVOKED_KEY, reason || "1");
      } catch {
        /* ignore */
      }
      clearAuthStatusCache();
      clearTabsLayoutCache();
      setUserState(null);
    });
  }, []);

  useEffect(() => {
    return onEvent<UserInfo>("auth.sessionUpdated", (payload) => {
      if (!payload || typeof payload !== "object") return;
      setAuthStatusCache(payload);
      setUserState(payload);
      void refreshTabsLayout().catch(() => {
        clearTabsLayoutCache();
      });
    });
  }, []);

  const value = useMemo(
    () => ({ user, setUser, refresh }),
    [user, setUser, refresh],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth outside AuthProvider");
  }
  return ctx;
}
