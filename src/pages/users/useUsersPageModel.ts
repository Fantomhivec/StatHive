import { useCallback, useEffect, useMemo, useState } from "react";
import { isMainAdminRole } from "../../bridge/accessRole";
import {
  ensureTabsLayout,
  peekTabsLayout,
  type AccessPresetDto,
  type ScriptCatalogItem,
  type TeamConfigListItem,
  type TeamUserRow,
} from "../../bridge/hostApi";
import { useAuth } from "../../auth/AuthContext";
import { useAppDialogs } from "../../components/useAppDialogs";
import { useBusyState } from "../../components/useBusyState";
import { useT } from "../../i18n";
import { shortTeamName } from "../../shared/workspaceTitle";
import {
  clearUsersPageSnap,
  emptyPreset,
  fetchUsersPageSnap,
  type UsersPageSnap,
} from "./usersPageCache";
import { createUsersPageMutations } from "./useUsersPageMutations";

export type UsersTab = "users" | "presets" | "oauth";

export function useUsersPageModel() {
  const t = useT();
  const { confirm, prompt, dialogs } = useAppDialogs();
  const { user: me } = useAuth();
  const { busy, error, setError, ok, setOk, withBusy } = useBusyState();
  const [tab, setTab] = useState<UsersTab>("users");
  const [canUsersTab, setCanUsersTab] = useState(false);
  const [isMainAdmin, setIsMainAdmin] = useState(false);
  const [gateReady, setGateReady] = useState(false);
  const [team, setTeam] = useState<TeamUserRow[]>([]);
  const [presets, setPresets] = useState<AccessPresetDto[]>([]);
  const [catalog, setCatalog] = useState<ScriptCatalogItem[]>([]);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [workspaceTitle, setWorkspaceTitle] = useState("");
  const [teamNameDraft, setTeamNameDraft] = useState("");
  const [saConfigured, setSaConfigured] = useState(false);
  const [saEmail, setSaEmail] = useState<string | null>(null);
  const [saJson, setSaJson] = useState("");

  const [name, setName] = useState("");
  const [login, setLogin] = useState("");
  const [password, setPassword] = useState("");
  const [email, setEmail] = useState("");
  const [invitePreset, setInvitePreset] = useState("");
  const [inviteCalcConfig, setInviteCalcConfig] = useState("");
  const [calcConfigs, setCalcConfigs] = useState<TeamConfigListItem[]>([]);
  const [editPreset, setEditPreset] = useState<AccessPresetDto>(emptyPreset);

  const presetOptions = useMemo(
    () => presets.map((p) => ({ value: p.id, label: p.title })),
    [presets],
  );
  const calcConfigOptions = useMemo(
    () =>
      calcConfigs.map((c) => ({
        value: c.id,
        label: c.isDefault
          ? `${c.name} (${t("calculator.settings.teamConfigDefaultBadge")})`
          : c.name,
      })),
    [calcConfigs, t],
  );

  const applySnap = useCallback((snap: UsersPageSnap) => {
    setTeam(snap.team);
    setPresets(snap.presets);
    setCatalog(snap.catalog);
    setCalcConfigs(snap.calcConfigs);
    setWorkspaceId(snap.workspaceId);
    setWorkspaceTitle(snap.workspaceTitle);
    setTeamNameDraft(shortTeamName(snap.workspaceTitle) || "StatHive");
    setSaConfigured(snap.saConfigured);
    setSaEmail(snap.saEmail);
    setInvitePreset((prev) => {
      if (prev && snap.presets.some((x) => x.id === prev)) return prev;
      if (snap.invitePreset && snap.presets.some((x) => x.id === snap.invitePreset)) {
        return snap.invitePreset;
      }
      return snap.presets[0]?.id ?? "";
    });
    setInviteCalcConfig((prev) => {
      if (prev && snap.calcConfigs.some((x) => x.id === prev)) return prev;
      if (
        snap.inviteCalcConfig &&
        snap.calcConfigs.some((x) => x.id === snap.inviteCalcConfig)
      ) {
        return snap.inviteCalcConfig;
      }
      return (
        snap.calcConfigs.find((x) => x.isDefault)?.id ??
        snap.teamCatDefaultId ??
        snap.calcConfigs[0]?.id ??
        ""
      );
    });
  }, []);

  const loadAll = useCallback(
    async (force = false) => {
      const loginKey = (me?.login ?? "").trim().toLowerCase();
      applySnap(await fetchUsersPageSnap(loginKey, force));
    },
    [applySnap, me?.login],
  );

  useEffect(() => {
    if (me === undefined) return;
    if (!me?.is_authorized) {
      clearUsersPageSnap();
      setGateReady(true);
      return;
    }
    let cancelled = false;
    const applyCaps = (res: { isMainAdmin?: boolean; canUsersTab?: boolean }) => {
      const main = !!res.isMainAdmin || isMainAdminRole(me.access_level);
      const usersOk = !!res.canUsersTab || main;
      setIsMainAdmin(main);
      setCanUsersTab(usersOk);
      setGateReady(true);
      return usersOk;
    };
    const peek = peekTabsLayout();
    // Caps из кэша layout / Admin — dashboard сразу параллельно с refresh layout.
    const startedLoad = peek
      ? applyCaps(peek)
      : isMainAdminRole(me.access_level)
        ? applyCaps({ isMainAdmin: true, canUsersTab: true })
        : false;
    // Shell уже открыл вкладку Users: без peek тоже грузим данные рядом с layout (не waterfall).
    const optimisticLoad = startedLoad || !!me.is_authorized;
    void (async () => {
      try {
        const tasks: Promise<unknown>[] = [ensureTabsLayout()];
        if (optimisticLoad) {
          tasks.push(
            loadAll().catch((e: unknown) => {
              if (!cancelled) {
                setError(e instanceof Error ? e.message : String(e));
              }
            }),
          );
        }
        const [layout] = (await Promise.all(tasks)) as [
          Awaited<ReturnType<typeof ensureTabsLayout>>,
          ...unknown[],
        ];
        if (cancelled) return;
        const usersOk = applyCaps(layout);
        if (!usersOk) {
          clearUsersPageSnap();
          setTeam([]);
          setPresets([]);
          setCatalog([]);
          return;
        }
        if (!optimisticLoad) {
          await loadAll();
        }
      } catch (e: unknown) {
        if (cancelled) return;
        const main = isMainAdminRole(me.access_level);
        setIsMainAdmin(main);
        setCanUsersTab(main);
        setGateReady(true);
        if (main && !optimisticLoad) {
          try {
            await loadAll();
          } catch (err: unknown) {
            setError(err instanceof Error ? err.message : String(err));
          }
        } else if (!optimisticLoad) {
          setError(e instanceof Error ? e.message : String(e));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [me, loadAll, setError]);

  const mutations = createUsersPageMutations({
    t,
    confirm,
    prompt,
    withBusy,
    setOk,
    setError,
    loadAll,
    isMainAdmin,
    workspaceId,
    setWorkspaceId,
    workspaceTitle,
    setWorkspaceTitle,
    teamNameDraft,
    setTeamNameDraft,
    saJson,
    setSaJson,
    saConfigured,
    name,
    setName,
    login,
    setLogin,
    password,
    setPassword,
    email,
    setEmail,
    invitePreset,
    inviteCalcConfig,
    setTeam,
    calcConfigs,
    presets,
    editPreset,
    setEditPreset,
  });

  return {
    t,
    dialogs,
    me,
    busy,
    error,
    setError,
    ok,
    setOk,
    withBusy,
    tab,
    setTab,
    canUsersTab,
    isMainAdmin,
    gateReady,
    team,
    presets,
    catalog,
    workspaceId,
    workspaceTitle,
    teamNameDraft,
    setTeamNameDraft,
    saConfigured,
    saEmail,
    saJson,
    setSaJson,
    name,
    setName,
    login,
    setLogin,
    password,
    setPassword,
    email,
    setEmail,
    invitePreset,
    setInvitePreset,
    inviteCalcConfig,
    setInviteCalcConfig,
    calcConfigs,
    editPreset,
    setEditPreset,
    presetOptions,
    calcConfigOptions,
    loadAll,
    ...mutations,
  };
}
