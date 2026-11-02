import { FormEvent, type Dispatch, type SetStateAction } from "react";
import { isMainAdminRole } from "../../bridge/accessRole";
import {
  hostApi,
  openUrl,
  type AccessPresetDto,
  type TeamConfigListItem,
  type TeamUserRow,
} from "../../bridge/hostApi";
import {
  shortTeamName,
  WORKSPACE_TITLE_CHANGED,
} from "../../shared/workspaceTitle";
import {
  emptyPreset,
  normalizeTeamRow,
  setUsersPageSnap,
  sheetsUrl,
  usersPageSnap,
} from "./usersPageCache";

type TFn = (key: string, params?: Record<string, string | number>) => string;

type BusyFn = (fn: () => Promise<void>) => Promise<void>;

export type UsersPageMutationsArgs = {
  t: TFn;
  confirm: (message: string, title?: string) => Promise<boolean>;
  prompt: (label: string, title?: string, initial?: string) => Promise<string | null>;
  withBusy: BusyFn;
  setOk: (msg: string | null) => void;
  setError: (msg: string | null) => void;
  loadAll: (force?: boolean) => Promise<void>;
  isMainAdmin: boolean;
  workspaceId: string | null;
  setWorkspaceId: Dispatch<SetStateAction<string | null>>;
  workspaceTitle: string;
  setWorkspaceTitle: Dispatch<SetStateAction<string>>;
  teamNameDraft: string;
  setTeamNameDraft: Dispatch<SetStateAction<string>>;
  saJson: string;
  setSaJson: Dispatch<SetStateAction<string>>;
  saConfigured: boolean;
  name: string;
  setName: Dispatch<SetStateAction<string>>;
  login: string;
  setLogin: Dispatch<SetStateAction<string>>;
  password: string;
  setPassword: Dispatch<SetStateAction<string>>;
  email: string;
  setEmail: Dispatch<SetStateAction<string>>;
  invitePreset: string;
  inviteCalcConfig: string;
  setTeam: Dispatch<SetStateAction<TeamUserRow[]>>;
  calcConfigs: TeamConfigListItem[];
  presets: AccessPresetDto[];
  editPreset: AccessPresetDto;
  setEditPreset: Dispatch<SetStateAction<AccessPresetDto>>;
};

export function createUsersPageMutations(a: UsersPageMutationsArgs) {
  async function onOpenWorkspaceSheet() {
    await a.withBusy(async () => {
      let sid = a.workspaceId?.trim() || "";
      if (!sid) {
        const setup = await hostApi.authSetupStatus();
        sid = setup.workspace_id?.trim() || "";
        a.setWorkspaceId(sid || null);
      }
      if (!sid) {
        throw new Error(a.t("users.errNoWorkspace"));
      }
      await openUrl(sheetsUrl(sid));
    });
  }

  async function reloadAfterSa(okMsg: string) {
    a.setOk(okMsg);
    try {
      await a.loadAll(true);
    } catch (e) {
      a.setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function onCreateServiceAccount() {
    await a.withBusy(async () => {
      const emailSaved = await hostApi.createWorkspaceServiceAccount();
      a.setSaJson("");
      await reloadAfterSa(a.t("users.serviceAccountCreated", { email: emailSaved }));
    });
  }

  async function onReplaceServiceAccount() {
    const okReplace = await a.confirm(
      a.t("users.serviceAccountReplaceConfirm"),
      a.t("dialogs.confirm"),
    );
    if (!okReplace) return;
    await a.withBusy(async () => {
      const emailSaved = await hostApi.replaceWorkspaceServiceAccount();
      a.setSaJson("");
      await reloadAfterSa(a.t("users.serviceAccountReplaced", { email: emailSaved }));
    });
  }

  async function onSaveServiceAccount(e: FormEvent) {
    e.preventDefault();
    await a.withBusy(async () => {
      if (!a.saJson.trim()) {
        throw new Error(a.t("users.errNoServiceAccount"));
      }
      const emailSaved = await hostApi.saveServiceAccount(a.saJson.trim());
      a.setSaJson("");
      await reloadAfterSa(a.t("users.serviceAccountSaved", { email: emailSaved }));
    });
  }

  function appendTeamRow(raw: TeamUserRow) {
    const row = normalizeTeamRow(raw);
    a.setTeam((prev) => {
      if (
        prev.some(
          (u) =>
            u.row_index === row.row_index ||
            u.login.toLowerCase() === row.login.toLowerCase(),
        )
      ) {
        return prev;
      }
      return [...prev, row];
    });
    if (usersPageSnap) {
      const exists = usersPageSnap.team.some(
        (u) =>
          u.row_index === row.row_index ||
          u.login.toLowerCase() === row.login.toLowerCase(),
      );
      if (!exists) {
        setUsersPageSnap({
          ...usersPageSnap,
          team: [...usersPageSnap.team, row],
        });
      }
    }
  }

  async function onInvite(e: FormEvent, onSuccess?: () => void) {
    e.preventDefault();
    if (!a.saConfigured) {
      a.setError(a.t("users.errNeedServiceAccount"));
      return;
    }
    if (!a.invitePreset.trim()) {
      a.setError(a.t("users.errNoPreset"));
      return;
    }
    const payload = {
      login: a.login.trim(),
      password: a.password,
      display_name: a.name.trim() || a.login.trim(),
      google_email: a.email.trim(),
      preset_id: a.invitePreset,
      calc_config_id: a.inviteCalcConfig,
    };
    await a.withBusy(async () => {
      a.setOk(a.t("users.inviteStarted"));
      try {
        const created = await hostApi.inviteUser(payload);
        a.setName("");
        a.setLogin("");
        a.setPassword("");
        a.setEmail("");
        appendTeamRow(created);
        a.setOk(a.t("users.inviteOk", { login: payload.login }));
        onSuccess?.();
      } catch (err) {
        try {
          await a.loadAll(true);
        } catch {
          /* оставляем исходную ошибку invite */
        }
        throw err instanceof Error ? err : new Error(String(err));
      }
    });
  }

  function onSetCalcConfig(rowIndex: number, nextId: string, shownId: string) {
    if (!nextId || nextId === shownId) return;
    a.setTeam((prev) =>
      prev.map((u) =>
        u.row_index === rowIndex
          ? { ...u, calc_config_id: nextId, calcConfigId: nextId }
          : u,
      ),
    );
    if (usersPageSnap) {
      setUsersPageSnap({
        ...usersPageSnap,
        team: usersPageSnap.team.map((u) =>
          u.row_index === rowIndex
            ? { ...u, calc_config_id: nextId, calcConfigId: nextId }
            : u,
        ),
      });
    }
    const cfgName = a.calcConfigs.find((x) => x.id === nextId)?.name ?? nextId;
    a.setOk(a.t("users.calcConfigAssigned", { name: cfgName }));
    void hostApi.setUserCalcConfig(rowIndex, nextId).catch((e: unknown) => {
      a.setError(e instanceof Error ? e.message : String(e));
    });
  }

  async function onCopyTeamCode() {
    const passphrase = await a.prompt(
      a.t("users.teamCodePassphraseLabel"),
      a.t("users.copyTeamCode"),
      "",
    );
    if (passphrase == null) return;
    if (passphrase.trim().length < 8) {
      a.setError(a.t("users.teamCodePassphraseMin"));
      return;
    }
    await a.withBusy(async () => {
      const code = await hostApi.exportTeamInvite(passphrase.trim());
      try {
        await navigator.clipboard.writeText(code);
        a.setOk(a.t("users.teamCodeCopied"));
      } catch {
        // Буфер недоступен — показать код в prompt для ручного копирования.
        await a.prompt(
          a.t("users.teamCodeCopyManual"),
          a.t("users.copyTeamCode"),
          code,
        );
        a.setOk(a.t("users.teamCodeCopyManual"));
      }
    });
  }

  async function onSetPreset(rowIndex: number, presetId: string) {
    if (!presetId || isMainAdminRole(presetId)) return;
    await a.withBusy(async () => {
      await hostApi.accessSetUserPreset(rowIndex, presetId);
      const title = a.presets.find((p) => p.id === presetId)?.title ?? presetId;
      a.setOk(a.t("users.presetAssigned", { title }));
      await a.loadAll(true);
    });
  }

  async function onSavePreset(preset: AccessPresetDto, onSuccess?: () => void) {
    await a.withBusy(async () => {
      const saved = await hostApi.accessSavePreset(preset);
      a.setOk(
        preset.id.trim()
          ? a.t("users.presetUpdated", { title: saved.title })
          : a.t("users.presetCreated", { title: saved.title }),
      );
      a.setEditPreset(emptyPreset());
      await a.loadAll(true);
      onSuccess?.();
    });
  }

  async function onDeletePreset(presetId: string) {
    const title = a.presets.find((p) => p.id === presetId)?.title ?? presetId;
    await a.withBusy(async () => {
      await hostApi.accessDeletePreset(presetId);
      if (a.editPreset.id === presetId) a.setEditPreset(emptyPreset());
      a.setOk(a.t("users.presetDeleted", { title }));
      await a.loadAll(true);
    });
  }

  async function onDeleteUser(rowIndex: number, loginName: string) {
    if (!a.isMainAdmin) return;
    const okDel = await a.confirm(
      a.t("users.deleteUserConfirm", { login: loginName }),
      a.t("dialogs.confirm"),
    );
    if (!okDel) return;
    await a.withBusy(async () => {
      await hostApi.deleteUser(rowIndex);
      a.setOk(a.t("users.deleteUserOk", { login: loginName }));
      await a.loadAll(true);
    });
  }

  async function onSaveTeamName(e: FormEvent, onSuccess?: () => void) {
    e.preventDefault();
    if (!a.isMainAdmin) return;
    await a.withBusy(async () => {
      const full = await hostApi.renameWorkspace(a.teamNameDraft.trim());
      a.setWorkspaceTitle(full);
      a.setTeamNameDraft(shortTeamName(full));
      if (usersPageSnap) {
        setUsersPageSnap({ ...usersPageSnap, workspaceTitle: full });
      }
      window.dispatchEvent(new CustomEvent(WORKSPACE_TITLE_CHANGED, { detail: full }));
      a.setOk(a.t("users.teamNameSaved"));
      onSuccess?.();
    });
  }

  return {
    onOpenWorkspaceSheet,
    onCreateServiceAccount,
    onReplaceServiceAccount,
    onSaveServiceAccount,
    onSaveTeamName,
    onInvite,
    onSetCalcConfig,
    onCopyTeamCode,
    onSetPreset,
    onSavePreset,
    onDeletePreset,
    onDeleteUser,
  };
}
