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
import HostTitleBar from "./components/HostTitleBar";
import { FAIL_CLOSED_TABS_LAYOUT } from "./hooks/useTabsLayoutCaps";
import { useT } from "./i18n";
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
  const [layout, setLayout] = useState<TabLayoutItem[] | null>(null);
  const [calculatorMode, setCalculatorMode] = useState(false);
  const [forceCalculatorMode, setForceCalculatorMode] = useState(false);
  const [canTemplates, setCanTemplates] = useState(false);
  const [canLogs, setCanLogs] = useState(false);
  const [canTablesTab, setCanTablesTab] = useState(false);
  const [canKeysCheck, setCanKeysCheck] = useState(false);
  const [canUsersTab, setCanUsersTab] = useState(false);
  const [calcAccess, setCalcAccess] = useState<CalcAccessDto>(
    FAIL_CLOSED_TABS_LAYOUT.calcAccess!,
  );
  const [layoutReady, setLayoutReady] = useState(false);
  const { user } = useAuth();
  const isAdmin = isMainAdminRole(user?.access_level);

  const defaultNav = useMemo<NavItem[]>(() => {
    const items: NavItem[] = [];
    if (canKeysCheck || isAdmin) {
      items.push({ tabKey: "KeysCheck", to: "/keys-check", label: t("nav.keysCheck") });
    }
    if (canTemplates) {
      items.push({ tabKey: "Templates", to: "/", label: t("nav.templates"), end: true });
    }
    if (canLogs) {
      items.push({ tabKey: "Logs", to: "/logs", label: t("nav.logs") });
    }
    return items;
  }, [t, canTemplates, canLogs, canKeysCheck, isAdmin]);

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
    ) => {
      setLayout(items);
      setCalculatorMode(mode);
      setForceCalculatorMode(force);
      setCanTemplates(templatesOk);
      setCanLogs(logsOk);
      setCanTablesTab(tablesTabOk);
      setCanKeysCheck(keysOk);
      setCanUsersTab(usersOk);
      setCalcAccess(access ?? FAIL_CLOSED_TABS_LAYOUT.calcAccess!);
      setLayoutReady(true);
    },
    [],
  );

  const homePath = canTemplates
    ? "/"
    : canKeysCheck || isAdmin
      ? "/keys-check"
      : canLogs
        ? "/logs"
        : "/profile";

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
        ),
      );
  }, [applyLayout, isAdmin]);

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
