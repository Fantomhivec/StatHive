import { useEffect, useRef, useState, type MouseEvent, type ReactNode } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import appIcon from "../assets/app-icon.png";
import { hostApi } from "../bridge/hostApi";
import { hostWindow } from "../bridge/hostApiChrome";
import { useAuth } from "../auth/AuthContext";
import { useTabsLayoutCaps } from "../hooks/useTabsLayoutCaps";
import { shortTeamName, WORKSPACE_TITLE_CHANGED } from "../shared/workspaceTitle";
import { IconGear } from "./icons";
import "./HostTitleBar.css";
import { useT } from "../i18n";

const APP_TITLE = "StatHive";

function IconMin() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M1 5.5h8" stroke="currentColor" strokeWidth="1" />
    </svg>
  );
}

function IconMax({ restored }: { restored: boolean }) {
  if (restored) {
    return (
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <rect x="2.5" y="0.5" width="7" height="7" fill="none" stroke="currentColor" strokeWidth="1" />
        <rect x="0.5" y="2.5" width="7" height="7" fill="var(--panel)" stroke="currentColor" strokeWidth="1" />
      </svg>
    );
  }
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" strokeWidth="1" />
    </svg>
  );
}

function IconClose() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M1.5 1.5l7 7M8.5 1.5l-7 7" stroke="currentColor" strokeWidth="1" />
    </svg>
  );
}

type Props = {
  /** shell: user + gear; login: только title + кнопки окна */
  variant?: "shell" | "login";
  title?: string;
};

export default function HostTitleBar({ variant, title }: Props) {
  const t = useT();
  const navigate = useNavigate();
  const location = useLocation();
  const isLogin =
    variant === "login" ||
    (variant == null && (location.pathname === "/login" || location.pathname.endsWith("/login")));
  const shell = !isLogin;

  const { user } = useAuth();
  const layoutCaps = useTabsLayoutCaps();
  const [menuOpen, setMenuOpen] = useState(false);
  const [maximized, setMaximized] = useState(false);
  const [workspaceTitle, setWorkspaceTitle] = useState("");
  const hideTabsSettings = shell && !!layoutCaps.forceCalculatorMode;
  const menuRef = useRef<HTMLDivElement>(null);

  const userLabel = (() => {
    if (!shell || !user) return "";
    const name = (user.display_name || "").trim();
    const login = (user.login || "").trim();
    return name && name !== login ? name : login;
  })();

  const teamLabel = shortTeamName(workspaceTitle);
  const profileLabel =
    teamLabel && userLabel ? `${teamLabel} - ${userLabel}` : userLabel;

  useEffect(() => {
    if (!shell || !user) {
      setWorkspaceTitle("");
      return;
    }
    let cancelled = false;
    const load = () => {
      void hostApi
        .authSetupStatus()
        .then((s) => {
          if (!cancelled) setWorkspaceTitle((s.workspace_title ?? "").trim());
        })
        .catch(() => undefined);
    };
    load();
    function onTitleChanged(ev: Event) {
      const detail = (ev as CustomEvent<string>).detail;
      if (typeof detail === "string" && detail.trim()) {
        setWorkspaceTitle(detail.trim());
      } else {
        load();
      }
    }
    window.addEventListener(WORKSPACE_TITLE_CHANGED, onTitleChanged);
    return () => {
      cancelled = true;
      window.removeEventListener(WORKSPACE_TITLE_CHANGED, onTitleChanged);
    };
  }, [shell, user]);

  useEffect(() => {
    if (!menuOpen) return;
    function onDoc(e: globalThis.MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setMenuOpen(false);
    }
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [menuOpen]);

  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    void hostWindow.isMaximized().then((v) => {
      if (alive) setMaximized(v);
    });
    void hostWindow
      .onResized(() => {
        void hostWindow.isMaximized().then((v) => {
          if (alive) setMaximized(v);
        });
      })
      .then((fn) => {
        if (!alive) fn();
        else unlisten = fn;
      });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  function onTitleMouseDown(e: MouseEvent<HTMLElement>) {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest("button, a, input, .host-tb-no-drag")) return;
    const refresh = () => {
      void hostWindow.isMaximized().then(setMaximized).catch(() => undefined);
    };
    if (e.detail === 2) {
      if (isLogin) return;
      void hostWindow.toggleMaximize().then(refresh).catch(() => undefined);
      return;
    }
    void hostWindow.startDragging().then(refresh).catch(() => undefined);
  }

  const barTitle = title ?? (isLogin ? t("host.loginTitle") : APP_TITLE);

  return (
    <header
      className={isLogin ? "host-titlebar host-titlebar--login" : "host-titlebar"}
      onMouseDown={onTitleMouseDown}
    >
      <div className="host-titlebar-left">
        <img className="host-app-icon" src={appIcon} alt="" draggable={false} />
        <div className="host-title">{barTitle}</div>
      </div>
      <div className="host-titlebar-actions host-tb-no-drag">
        {shell && profileLabel ? (
          <button
            type="button"
            className="btn btn-ghost btn-host-username host-username"
            onClick={() => navigate("/profile")}
            title={t("host.openProfileTitle")}
          >
            {profileLabel}
          </button>
        ) : null}

        {shell ? (
          <div className="host-menu-wrap" ref={menuRef}>
            <button
              type="button"
              className="btn btn-chrome-sm"
              title={t("common.settings")}
              aria-label={t("host.settingsMenuAria")}
              aria-expanded={menuOpen}
              onClick={() => setMenuOpen((v) => !v)}
            >
              <IconGear />
            </button>
            {menuOpen ? (
              <div className="popover popover--pad-sm host-menu">
                {!hideTabsSettings ? (
                  <button
                    type="button"
                    className="menu-item menu-item--accent"
                    onClick={() => {
                      setMenuOpen(false);
                      navigate("/tabs-settings");
                    }}
                  >
                    {t("host.tabsSettings")}
                  </button>
                ) : null}
                <button
                  type="button"
                  className="menu-item menu-item--accent"
                  onClick={() => {
                    setMenuOpen(false);
                    navigate("/settings");
                  }}
                >
                  {t("host.uiSettings")}
                </button>
                <button
                  type="button"
                  className="menu-item menu-item--accent"
                  onClick={() => {
                    setMenuOpen(false);
                    navigate("/profile");
                  }}
                >
                  {t("host.profileMenu")}
                </button>
              </div>
            ) : null}
          </div>
        ) : null}

        <WinButton label={t("common.collapse")} onClick={() => void runWin(() => hostWindow.minimize())}>
          <IconMin />
        </WinButton>
        {!isLogin ? (
          <WinButton
            label={maximized ? t("host.restoreDown") : t("common.expand")}
            onClick={() =>
              void runWin(async () => {
                await hostWindow.toggleMaximize();
                setMaximized(await hostWindow.isMaximized());
              })
            }
          >
            <IconMax restored={maximized} />
          </WinButton>
        ) : null}
        <WinButton label={t("common.close")} close onClick={() => void runWin(() => hostWindow.close())}>
          <IconClose />
        </WinButton>
      </div>
    </header>
  );
}

function WinButton({
  label,
  close,
  onClick,
  children,
}: {
  label: string;
  close?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className={close ? "btn-win btn-win-close" : "btn-win"}
      title={label}
      aria-label={label}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

async function runWin(action: () => Promise<void>) {
  try {
    await action();
  } catch {
    /* окно уже закрывается или API недоступен */
  }
}
