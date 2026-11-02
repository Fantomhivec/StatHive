import { useCallback, useEffect, useMemo, useState } from "react";
import { hostApi, type TeamConfigListItem } from "../../bridge/hostApi";
import AppComboBox from "../../components/AppComboBox";
import AppCustomWindow from "../../components/AppCustomWindow";
import { useT } from "../../i18n";

type Props = {
  open: boolean;
  isAdmin: boolean;
  busy: boolean;
  onClose: () => void;
  onBusy: (v: boolean) => void;
  onError: (msg: string | null) => void;
  onMessage: (title: string, message: string, tone?: "success" | "warning" | "error") => void;
  onLoaded: () => void;
};

export default function TeamConfigsDialog({
  open,
  isAdmin,
  busy,
  onClose,
  onBusy,
  onError,
  onMessage,
  onLoaded,
}: Props) {
  const t = useT();
  const [items, setItems] = useState<TeamConfigListItem[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [name, setName] = useState("");
  const [makeDefault, setMakeDefault] = useState(false);

  const reload = useCallback(async () => {
    const cat = await hostApi.wbCalculator.listTeamConfigs();
    const next: TeamConfigListItem[] = cat.items ?? [];
    setItems(next);
    const def =
      next.find((x: TeamConfigListItem) => x.isDefault)?.id ??
      cat.defaultId ??
      next[0]?.id ??
      "";
    setSelectedId((prev) =>
      prev && next.some((x: TeamConfigListItem) => x.id === prev) ? prev : def,
    );
  }, []);

  const itemOptions = useMemo(
    () =>
      items.map((x) => ({
        value: x.id,
        label: x.isDefault
          ? `${x.name} (${t("calculator.settings.teamConfigDefaultBadge")})`
          : x.name,
      })),
    [items, t],
  );

  useEffect(() => {
    if (!open) return;
    void reload().catch((e) => onError(e instanceof Error ? e.message : String(e)));
  }, [open, reload, onError]);

  if (!open) return null;

  const selected = items.find((x) => x.id === selectedId);

  async function save() {
    const n = name.trim() || selected?.name || "";
    if (!n) {
      onError(t("calculator.settings.teamConfigNamePlaceholder"));
      return;
    }
    onBusy(true);
    onError(null);
    try {
      const res = await hostApi.wbCalculator.publishTeamConfig(n, makeDefault);
      setName("");
      setMakeDefault(false);
      await reload();
      if (res.id) setSelectedId(res.id);
      onMessage(
        t("calculator.settings.exportSavedTitle"),
        t("calculator.settings.teamConfigSaved", { name: res.name ?? n }),
        "success",
      );
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      onBusy(false);
    }
  }

  async function loadLocal() {
    if (!selectedId) return;
    onBusy(true);
    onError(null);
    try {
      const res = await hostApi.wbCalculator.pullTeamConfig(selectedId);
      if (!res.pulled) {
        onMessage(
          t("calculator.settings.exportSavedTitle"),
          t("calculator.settings.teamConfigEmpty"),
          "warning",
        );
        return;
      }
      onLoaded();
      onMessage(
        t("calculator.settings.exportSavedTitle"),
        t("calculator.settings.teamConfigLoaded", {
          name: res.name ?? selected?.name ?? selectedId,
        }),
        "success",
      );
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      onBusy(false);
    }
  }

  async function setDefault() {
    if (!selectedId) return;
    onBusy(true);
    onError(null);
    try {
      await hostApi.wbCalculator.setDefaultTeamConfig(selectedId);
      await reload();
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      onBusy(false);
    }
  }

  async function remove() {
    if (!selectedId) return;
    onBusy(true);
    onError(null);
    try {
      await hostApi.wbCalculator.deleteTeamConfig(selectedId);
      await reload();
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      onBusy(false);
    }
  }

  return (
    <AppCustomWindow
      title={t("calculator.settings.teamConfigs")}
      onClose={onClose}
      width={480}
      footer={
        <div className="dialog-footer">
          <button type="button" className="btn btn-ghost" disabled={busy} onClick={onClose}>
            {t("common.close")}
          </button>
        </div>
      }
    >
      <div className="team-configs-dialog combo-clip-ok field-flush">
        <p className="muted">{t("calculator.settings.teamConfigsTitle")}</p>
        {items.length === 0 ? (
          <p className="muted">{t("calculator.settings.teamConfigEmpty")}</p>
        ) : (
          <div className="field">
            <label id="team-cfg-pick">{t("calculator.settings.teamConfigPick")}</label>
            <AppComboBox
              value={selectedId}
              disabled={busy}
              options={itemOptions}
              onChange={setSelectedId}
            />
          </div>
        )}
        <div className="form-actions">
          <button
            type="button"
            className="btn btn-primary"
            disabled={busy || !selectedId}
            onClick={() => void loadLocal()}
          >
            {t("calculator.settings.teamConfigLoad")}
          </button>
          {isAdmin && (
            <>
              <button
                type="button"
                className="btn btn-outline"
                disabled={busy || !selectedId || selected?.isDefault}
                onClick={() => void setDefault()}
              >
                {t("calculator.settings.teamConfigSetDefault")}
              </button>
              <button
                type="button"
                className="btn btn-danger"
                disabled={busy || !selectedId}
                onClick={() => void remove()}
              >
                {t("calculator.settings.teamConfigDelete")}
              </button>
            </>
          )}
        </div>
        {isAdmin && (
          <>
            <div className="field">
              <label htmlFor="team-cfg-name">{t("calculator.settings.teamConfigName")}</label>
              <input
                id="team-cfg-name"
                className="input"
                value={name}
                disabled={busy}
                placeholder={
                  selected?.name || t("calculator.settings.teamConfigNamePlaceholder")
                }
                onChange={(e) => setName(e.target.value)}
              />
            </div>
            <label className="users-check-row">
              <input
                type="checkbox"
                checked={makeDefault}
                disabled={busy}
                onChange={(e) => setMakeDefault(e.target.checked)}
              />
              <span>{t("calculator.settings.teamConfigMakeDefault")}</span>
            </label>
            <div className="form-actions">
              <button
                type="button"
                className="btn btn-primary"
                disabled={busy}
                onClick={() => void save()}
              >
                {t("calculator.settings.teamConfigSave")}
              </button>
            </div>
          </>
        )}
      </div>
    </AppCustomWindow>
  );
}
