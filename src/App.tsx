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

import { Navigate, NavLink, Outlet, Route, Routes, useLocation, useNavigate } from "react-router-dom";
import { lazy, Suspense, useCallback, useEffect, useMemo, useState } from "react";
import {
  ensureTabsLayout,
  onTabsLayoutChanged,
  peekLayoutGen,
  peekTabsLayout,
  setTabsLayoutCache,
} from "./bridge/hostApi.layout";
import type { CalcAccessDto, TabLayoutItem } from "./bridge/hostApi.types";
import { isMainAdminRole } from "./bridge/accessRole";
import { AuthProvider, useAuth } from "./auth/AuthContext";
import { useEnsureGoogleOauth } from "./auth/useEnsureGoogleOauth";
import HostTitleBar from "./components/HostTitleBar";
import { FAIL_CLOSED_TABS_LAYOUT } from "./bridge/useTabsLayoutCaps";
import { useT } from "./i18n";
import { ensureTemplatesRunProgressBridge } from "./pages/templates/templatesPageCache";
import "./styles/app.css";

const UpdateAvailableDialog = lazy(() => import("./components/UpdateAvailableDialog"));
const LoginPage = lazy(() => import("./pages/LoginPage"));
const TemplatesPage = lazy(() => import("./pages/TemplatesPage"));
const KeysCheckPage = lazy(() => import("./pages/KeysCheckPage"));
const CalculatorViewPage = lazy(() => import("./pages/CalculatorViewPage"));
const CalculatorTablesPage = lazy(() => import("./pages/CalculatorTablesPage"));
const CalculatorSettingsPage = lazy(() => import("./pages/CalculatorSettingsPage"));
const CalculatorKgtRulesPage = lazy(() => import("./pages/CalculatorKgtRulesPage"));
const TabsSettingsPage = lazy(() => import("./pages/TabsSettingsPage"));
const LogsPage = lazy(() => import("./pages/LogsPage"));
const ProfilePage = lazy(() => import("./pages/ProfilePage"));
const UsersPage = lazy(() => import("./pages/UsersPage"));
const AllTablesPage = lazy(() => import("./pages/AllTablesPage"));
const SettingsPage = lazy(() => import("./pages/SettingsPage"));

type NavItem = { tabKey: string; to: string; label: string; end?: boolean; calcFlag?: keyof CalcAccessDto };

const CALC_PATHS = [
  "/calculator",
  "/calculator-settings",
  "/calculator-tables",
  "/calculator-kgt-rules",
  /** Устаревший alias: раньше ошибочно был в обычном меню. */
  "/tables",
];

/** Ключи, которые нельзя показывать в обычном сайдбаре. */
const CALC_TAB_KEYS = new Set([
  "tables",
  "calculatortables",
  "calculatorview",
  "calculatorsettings",
  "calculatorkgtrules",
  "wbcalculator",
]);

function PageFallback() {
  const t = useT();
  return <div className="login-page">{t("common.loading")}</div>;
}

function RequireAuth({ children }: { children: React.ReactNode }) {
  const { user } = useAuth();
  // Layout подтягивает ShellLayout / useTabsLayoutCaps — без дубля IPC здесь.
  if (user === undefined) return <PageFallback />;
  if (!user?.is_authorized) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function ShellLayout() {
  const t = useT();
  const navigate = useNavigate();
  const location = useLocation();
  // Keep templates.is_running cache alive on Logs / other tabs.
  useEffect(() => {
    ensureTemplatesRunProgressBridge();
  }, []);
  const [layout, setLayout] = useState<TabLayoutItem[] | null>(null);
  const [calculatorMode, setCalculatorMode] = useState(false);
  const [forceCalculatorMode, setForceCalculatorMode] = useState(false);
  const [canTemplates, setCanTemplates] = useState(false);
  const [canLogs, setCanLogs] = useState(false);
  const [canTablesTab, setCanTablesTab] = useState(false);
  const [canKeysCheck, setCanKeysCheck] = useState(false);
  const [canUsersTab, setCanUsersTab] = useState(false);
  const [canAllTables, setCanAllTables] = useState(false);
  const [calcAccess, setCalcAccess] = useState<CalcAccessDto>(
    FAIL_CLOSED_TABS_LAYOUT.calcAccess!,
  );
  const [layoutReady, setLayoutReady] = useState(false);
  const { user, setUser } = useAuth();
  const isAdmin = isMainAdminRole(user?.access_level);
  const canCheckGoogle =
    !!peekTabsLayout()?.canCheckGoogle || isAdmin;
  useEnsureGoogleOauth({
    enabled: layoutReady && !!user?.is_authorized,
    sessionId: user?.session_id ?? undefined,
    canCheckGoogle,
    setUser,
  });

  const defaultNav = useMemo<NavItem[]>(() => {
    const items: NavItem[] = [];
    if (canKeysCheck || isAdmin) {
      items.push({ tabKey: "KeysCheck", to: "/keys-check", label: t("nav.keysCheck") });
    }
    if (canTemplates) {
      items.push({ tabKey: "Templates", to: "/", label: t("nav.templates"), end: true });
    }
    if (canAllTables || isAdmin) {
      items.push({ tabKey: "AllTables", to: "/all-tables", label: t("nav.allTables") });
    }
    if (canLogs) {
      items.push({ tabKey: "Logs", to: "/logs", label: t("nav.logs") });
    }
    return items;
  }, [t, canTemplates, canLogs, canKeysCheck, canAllTables, isAdmin]);

  const adminUsersNav = useMemo<NavItem>(
    () => ({
      tabKey: "Users",
      to: "/users",
      label: t("nav.users"),
    }),
    [t],
  );

  const calculatorNavAll = useMemo<NavItem[]>(
    () => [
      { tabKey: "CalculatorView", to: "/calculator", label: t("nav.calculator"), calcFlag: "view" },
      {
        tabKey: "CalculatorSettings",
        to: "/calculator-settings",
        label: t("nav.calculatorSettings"),
        calcFlag: "settings",
      },
      {
        tabKey: "CalculatorTables",
        to: "/calculator-tables",
        label: t("nav.calculatorTables"),
        calcFlag: "tables",
      },
      {
        tabKey: "CalculatorKgtRules",
        to: "/calculator-kgt-rules",
        label: t("nav.calculatorKgtRules"),
        calcFlag: "kgt",
      },
    ],
    [t],
  );

  const calculatorNav = useMemo(
    () =>
      calculatorNavAll.filter((n) => {
        if (!n.calcFlag || !calcAccess[n.calcFlag]) return false;
        if (n.calcFlag === "tables" && !canTablesTab) return false;
        return true;
      }),
    [calculatorNavAll, calcAccess, canTablesTab],
  );

  const firstCalcPath = calculatorNav[0]?.to ?? "/calculator";

  const applyLayout = useCallback(
    (
      items: TabLayoutItem[],
      mode: boolean,
      templatesOk: boolean,
      logsOk: boolean,
      tablesTabOk: boolean,
      force: boolean,
      access: CalcAccessDto | undefined,
      keysOk: boolean,
      usersOk: boolean,
      allTablesOk: boolean,
    ) => {
      setLayout(items);
      setCalculatorMode(mode);
      setForceCalculatorMode(force);
      setCanTemplates(templatesOk);
      setCanLogs(logsOk);
      setCanTablesTab(tablesTabOk);
      setCanKeysCheck(keysOk);
      setCanUsersTab(usersOk);
      setCanAllTables(allTablesOk);
      setCalcAccess(access ?? FAIL_CLOSED_TABS_LAYOUT.calcAccess!);
      setLayoutReady(true);
    },
    [],
  );

  const homePath = canTemplates
    ? "/"
    : canKeysCheck || isAdmin
      ? "/keys-check"
      : canAllTables || isAdmin
        ? "/all-tables"
        : canLogs
          ? "/logs"
          : "/profile";

  const sessionId = user?.session_id ?? "";
  const accessLevel = user?.access_level ?? "";

  // Смена команды/сессии в одном процессе: сброс shell до apply нового layout.
  useEffect(() => {
    setLayoutReady(false);
    setCalculatorMode(false);
    setForceCalculatorMode(false);
    const peek = peekTabsLayout();
    if (peek) {
      applyLayout(
        peek.items ?? [],
        !!peek.calculatorMode,
        peek.canTemplates === true,
        peek.canLogs === true,
        peek.canTablesTab === true,
        !!peek.forceCalculatorMode,
        peek.calcAccess,
        !!peek.canKeysCheck || isAdmin,
        !!peek.canUsersTab || isAdmin,
        !!peek.canAllTables || isAdmin,
      );
      return;
    }
    applyLayout(
      [],
      false,
      false,
      false,
      false,
      false,
      FAIL_CLOSED_TABS_LAYOUT.calcAccess,
      isAdmin,
      isAdmin,
      isAdmin,
    );
    setLayoutReady(false);
  }, [sessionId, accessLevel, applyLayout, isAdmin]);

  useEffect(() => {
    const genAtStart = peekLayoutGen();
    void ensureTabsLayout()
      .then((res) => {
        // Событие/refresh успели обновить кэш — берём peek, не stale IPC.
        const fresh = peekLayoutGen() !== genAtStart ? peekTabsLayout() ?? res : res;
        if (peekLayoutGen() === genAtStart) {
          setTabsLayoutCache(fresh);
        }
        applyLayout(
          fresh.items ?? [],
          !!fresh.calculatorMode,
          fresh.canTemplates === true,
          fresh.canLogs === true,
          fresh.canTablesTab === true,
          !!fresh.forceCalculatorMode,
          fresh.calcAccess,
          !!fresh.canKeysCheck || isAdmin,
          !!fresh.canUsersTab || isAdmin,
          !!fresh.canAllTables || isAdmin,
        );
      })
      .catch(() =>
        applyLayout(
          [],
          false,
          false,
          false,
          false,
          false,
          FAIL_CLOSED_TABS_LAYOUT.calcAccess,
          isAdmin,
          isAdmin,
          isAdmin,
        ),
      );
  }, [applyLayout, isAdmin, sessionId, accessLevel]);

  useEffect(() => {
    return onTabsLayoutChanged((res) => {
      // setTabsLayoutCache уже внутри onTabsLayoutChanged (Tauri path);
      // локальный ur.tabsLayout приходит после save/refresh — кэш уже свежий.
      applyLayout(
        res.items ?? [],
        !!res.calculatorMode,
        res.canTemplates === true,
        res.canLogs === true,
        res.canTablesTab === true,
        !!res.forceCalculatorMode,
        res.calcAccess,
        !!res.canKeysCheck || isAdmin,
        !!res.canUsersTab || isAdmin,
        !!res.canAllTables || isAdmin,
      );
    });
  }, [applyLayout, isAdmin]);

  useEffect(() => {
    if (!layoutReady) return;
    const path = location.pathname;

    if (calculatorMode) {
      const allowedShell = ["/profile", "/settings", "/login"];
      if (!forceCalculatorMode) {
        allowedShell.push("/tabs-settings");
      }
      const calcAllowed = calculatorNav.map((n) => n.to);
      const pathOk =
        allowedShell.some((p) => path === p || path.startsWith(`${p}/`)) ||
        calcAllowed.some((p) => path === p || path.startsWith(`${p}/`)) ||
        (path === "/tables" && calcAccess.tables);
      if (!pathOk) {
        navigate(firstCalcPath, { replace: true });
      }
      return;
    }

    if (path === "/users" || path.startsWith("/users/")) {
      if (user && !canUsersTab && !isAdmin) {
        navigate(homePath, { replace: true });
      }
      return;
    }

    if (
      !canKeysCheck &&
      !isAdmin &&
      (path === "/keys-check" || path.startsWith("/keys-check/"))
    ) {
      navigate(homePath, { replace: true });
      return;
    }

    if (
      !canAllTables &&
      !isAdmin &&
      (path === "/all-tables" || path.startsWith("/all-tables/"))
    ) {
      navigate(homePath, { replace: true });
      return;
    }

    if (!canTemplates && (path === "/" || path.startsWith("/templates"))) {
      navigate(homePath, { replace: true });
      return;
    }

    if (!canLogs && (path === "/logs" || path.startsWith("/logs/"))) {
      navigate(homePath, { replace: true });
      return;
    }

    if (CALC_PATHS.some((p) => path === p || path.startsWith(`${p}/`))) {
      navigate(homePath, { replace: true });
    }
  }, [
    calculatorMode,
    forceCalculatorMode,
    canTemplates,
    canLogs,
    homePath,
    layoutReady,
    location.pathname,
    navigate,
    user,
    isAdmin,
    canKeysCheck,
    canUsersTab,
    canAllTables,
    calculatorNav,
    firstCalcPath,
    calcAccess.tables,
  ]);

  const navItems = useMemo(() => {
    if (calculatorMode) return calculatorNav;
    let base: NavItem[];
    if (!layout || layout.length === 0) {
      base = [...defaultNav];
    } else {
      const byKey = new Map(defaultNav.map((n) => [n.tabKey.toLowerCase(), n]));
      const ordered = [...layout]
        .sort((a, b) => a.order - b.order)
        .filter((it) => it.isVisible)
        .filter((it) => !CALC_TAB_KEYS.has((it.tabKey || "").toLowerCase()))
        .filter((it) => {
          const k = (it.tabKey || "").toLowerCase();
          if (k === "templates" && !canTemplates) return false;
          if (k === "logs" && !canLogs) return false;
          if (k === "keyscheck" && !canKeysCheck && !isAdmin) return false;
          if (k === "alltables" && !canAllTables && !isAdmin) return false;
          return true;
        })
        .map((it) => {
          const def = byKey.get((it.tabKey || "").toLowerCase());
          if (!def) return null;
          return { ...def, label: it.title || def.label };
        })
        .filter(Boolean) as NavItem[];
      base = ordered.length > 0 ? ordered : [...defaultNav];
    }
    if ((canUsersTab || isAdmin) && !base.some((n) => n.tabKey === "Users")) {
      const logsIdx = base.findIndex((n) => n.tabKey === "Logs");
      if (logsIdx >= 0) {
        base = [...base.slice(0, logsIdx), adminUsersNav, ...base.slice(logsIdx)];
      } else {
        base = [...base, adminUsersNav];
      }
    }
    return base;
  }, [
    layout,
    calculatorMode,
    canTemplates,
    canLogs,
    canKeysCheck,
    canUsersTab,
    canAllTables,
    isAdmin,
    calculatorNav,
    defaultNav,
    adminUsersNav,
  ]);

  return (
    <div className="shell">
      <nav className="shell-nav" aria-label={t("nav.sectionsAria")}>
        {navItems.map((l) => (
          <NavLink
            key={l.tabKey}
            to={l.to}
            end={l.end}
            className={({ isActive }) => (isActive ? "active" : undefined)}
          >
            {l.label}
          </NavLink>
        ))}
      </nav>
      <main className="shell-main">
        <Outlet />
      </main>
      <Suspense fallback={null}>
        <UpdateAvailableDialog />
      </Suspense>
    </div>
  );
}

export default function App() {
  return (
    <AuthProvider>
      <div className="app-frame">
        <HostTitleBar />
        <div className="app-body">
          <AppRoutes />
        </div>
      </div>
    </AuthProvider>
  );
}

function AppRoutes() {
  return (
    <Suspense fallback={<PageFallback />}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route
          path="/"
          element={
            <RequireAuth>
              <ShellLayout />
            </RequireAuth>
          }
        >
          <Route index element={<TemplatesPage />} />
          <Route path="keys-check" element={<KeysCheckPage />} />
          <Route path="all-tables" element={<AllTablesPage />} />
          {/* /tables — устаревший alias; только calculator-tables в режиме калькулятора */}
          <Route path="tables" element={<Navigate to="/calculator-tables" replace />} />
          <Route path="calculator" element={<CalculatorViewPage />} />
          <Route path="calculator-tables" element={<CalculatorTablesPage />} />
          <Route path="calculator-settings" element={<CalculatorSettingsPage />} />
          <Route path="calculator-kgt-rules" element={<CalculatorKgtRulesPage />} />
          <Route path="tabs-settings" element={<TabsSettingsPage />} />
          <Route path="logs" element={<LogsPage />} />
          <Route path="users" element={<UsersPage />} />
          <Route path="profile" element={<ProfilePage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Suspense>
  );
}
