import { useMemo } from "react";
import { isMainAdminRole } from "../../bridge/accessRole";
import type { TeamConfigListItem, TeamUserRow } from "../../bridge/hostApi";
import AppComboBox from "../../components/AppComboBox";
import { useT } from "../../i18n";

export type UsersTeamTableProps = {
  team: TeamUserRow[];
  busy: boolean;
  isMainAdmin: boolean;
  presets: { id: string }[];
  calcConfigs: TeamConfigListItem[];
  presetOptions: { value: string; label: string }[];
  calcConfigOptions: { value: string; label: string }[];
  onSetPreset: (rowIndex: number, presetId: string) => void;
  onSetCalcConfig: (rowIndex: number, nextId: string, shownId: string) => void;
  onDeleteUser: (rowIndex: number, login: string) => void;
};

export default function UsersTeamTable({
  team,
  busy,
  isMainAdmin,
  presets,
  calcConfigs,
  presetOptions,
  calcConfigOptions,
  onSetPreset,
  onSetCalcConfig,
  onDeleteUser,
}: UsersTeamTableProps) {
  const t = useT();

  /** Стабильные options на orphan id — без нового массива на каждую строку. */
  const presetOptsByLevel = useMemo(() => {
    const m = new Map<string, { value: string; label: string }[]>();
    for (const u of team) {
      const level = u.access_level;
      if (m.has(level)) continue;
      if (level && !presets.some((p) => p.id === level)) {
        m.set(level, [{ value: level, label: level }, ...presetOptions]);
      } else {
        m.set(level, presetOptions);
      }
    }
    return m;
  }, [team, presets, presetOptions]);

  const calcOptsById = useMemo(() => {
    const m = new Map<string, { value: string; label: string }[]>();
    for (const u of team) {
      const cfgId = (u.calcConfigId ?? u.calc_config_id ?? "").trim();
      if (m.has(cfgId)) continue;
      if (cfgId && !calcConfigs.some((c) => c.id === cfgId)) {
        m.set(cfgId, [{ value: cfgId, label: cfgId }, ...calcConfigOptions]);
      } else {
        m.set(cfgId, calcConfigOptions);
      }
    }
    return m;
  }, [team, calcConfigs, calcConfigOptions]);

  return (
    <section className="card card--flush card--stack users-panel">
      <h2 className="panel-title">{t("users.teamTitle", { count: team.length })}</h2>
      <div className="users-table-wrap">
        <table className="data-table data-table--top">
          <thead>
            <tr>
              <th>{t("common.login")}</th>
              <th>{t("common.name")}</th>
              <th>{t("users.presetRights")}</th>
              <th>{t("users.calcConfig")}</th>
              <th>{t("common.email")}</th>
              {isMainAdmin ? <th /> : null}
            </tr>
          </thead>
          <tbody>
            {team.map((u) => {
              const cfgId = (u.calcConfigId ?? u.calc_config_id ?? "").trim();
              return (
                <tr key={u.row_index}>
                  <td>{u.login}</td>
                  <td>{u.display_name || u.login || "—"}</td>
                  <td className="users-preset-cell">
                    {isMainAdminRole(u.access_level) ? (
                      <span>{t("users.adminRole")}</span>
                    ) : (
                      <AppComboBox
                        className="app-combo-sm users-inline-combo"
                        value={u.access_level}
                        disabled={busy || presets.length === 0}
                        options={presetOptsByLevel.get(u.access_level) ?? presetOptions}
                        onChange={(v) => void onSetPreset(u.row_index, v)}
                      />
                    )}
                  </td>
                  <td className="users-preset-cell">
                    {isMainAdminRole(u.access_level) ? (
                      <span className="muted">—</span>
                    ) : (
                      <AppComboBox
                        className="app-combo-sm users-inline-combo"
                        value={cfgId}
                        disabled={calcConfigs.length === 0}
                        placeholder={t("users.noCalcConfigsOption")}
                        options={calcOptsById.get(cfgId) ?? calcConfigOptions}
                        onChange={(v) => onSetCalcConfig(u.row_index, v, cfgId)}
                      />
                    )}
                  </td>
                  <td className="mono">{u.google_email || "—"}</td>
                  {isMainAdmin ? (
                    <td>
                      {!isMainAdminRole(u.access_level) ? (
                        <button
                          type="button"
                          className="btn btn-danger btn-sm"
                          disabled={busy}
                          onClick={() => void onDeleteUser(u.row_index, u.login)}
                        >
                          {t("users.deleteUser")}
                        </button>
                      ) : null}
                    </td>
                  ) : null}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </section>
  );
}
