// Copyright 2026 StatHive
import AppCustomWindow from "./AppCustomWindow";
import { useT } from "../i18n";

export type TableUserPerm = {
  id: string;
  email?: string;
  role?: string;
  type?: string;
  displayName?: string;
  canRemove?: boolean;
};

type Props = {
  tableName: string;
  users: TableUserPerm[];
  canManage: boolean;
  busy?: boolean;
  onRemove: (permissionId: string) => void;
  onClose: () => void;
};

function roleDisplay(role: string | undefined, t: (k: string) => string) {
  const r = (role ?? "").toLowerCase();
  if (r === "owner") return t("allTables.roleOwner");
  if (r === "writer") return t("allTables.roleWriter");
  if (r === "commenter") return t("allTables.roleCommenter");
  if (r === "reader") return t("allTables.roleReader");
  return role || "—";
}

function typeDisplay(type: string | undefined, t: (k: string) => string) {
  const ty = (type ?? "").toLowerCase();
  if (ty === "user") return t("allTables.typeUser");
  if (ty === "group") return t("allTables.typeGroup");
  if (ty === "domain") return t("allTables.typeDomain");
  if (ty === "anyone") return t("allTables.typeAnyone");
  return type || "—";
}

export default function TableUsersDialog({
  tableName,
  users,
  canManage,
  busy,
  onRemove,
  onClose,
}: Props) {
  const t = useT();
  return (
    <AppCustomWindow
      title={t("allTables.usersDialogTitle")}
      width={720}
      onClose={onClose}
      bodyClassName="table-users-body"
      footer={
        <button type="button" className="btn btn-outline" onClick={onClose}>
          {t("allTables.close")}
        </button>
      }
    >
      <div className="table-users-content">
        <h2 className="table-users-name">{tableName}</h2>
        <p className="table-users-desc">{t("allTables.usersDialogDesc")}</p>

        <div className="table-users-grid-wrap">
          <table className="table-users-grid">
            <thead>
              <tr>
                <th>{t("allTables.colName")}</th>
                <th>{t("allTables.colEmail")}</th>
                <th>{t("allTables.colType")}</th>
                <th>{t("allTables.colRole")}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {users.length === 0 ? (
                <tr>
                  <td colSpan={5} className="muted">
                    {t("allTables.noUsers")}
                  </td>
                </tr>
              ) : (
                users.map((u) => (
                  <tr key={u.id}>
                    <td>{u.displayName || u.email || "—"}</td>
                    <td>{u.email || "—"}</td>
                    <td>{typeDisplay(u.type, t)}</td>
                    <td>{roleDisplay(u.role, t)}</td>
                    <td>
                      {canManage && u.canRemove ? (
                        <button
                          type="button"
                          className="btn btn-outline btn-sm"
                          disabled={busy}
                          onClick={() => onRemove(u.id)}
                        >
                          {t("allTables.revoke")}
                        </button>
                      ) : null}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </AppCustomWindow>
  );
}
