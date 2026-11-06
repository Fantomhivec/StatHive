// Copyright 2026 StatHive
import { useMemo, useState } from "react";
import { hostApi, openUrl } from "../bridge/hostApi";
import AppComboBox from "../components/AppComboBox";
import TableUsersDialog, {
  type TableUserPerm,
} from "../components/TableUsersDialog";
import { useAppDialogs } from "../components/useAppDialogs";
import { useT } from "../i18n";
import "./AllTablesPage.css";

type TableRow = {
  id: string;
  name: string;
  url?: string | null;
  created?: string;
  createdTs?: number;
  accessStatus?: string;
  usersCount?: number;
  isOwner?: boolean;
};

export default function AllTablesPage() {
  const t = useT();
  const { confirm, dialogs } = useAppDialogs();
  const [tables, setTables] = useState<TableRow[]>([]);
  const [filter, setFilter] = useState("");
  const [accessFilter, setAccessFilter] = useState("");
  const [sharedOnly, setSharedOnly] = useState(false);
  const [canManage, setCanManage] = useState(false);
  /** Только прогон списка — UI (поиск/фильтр/навигация) не блокируем. */
  const [loading, setLoading] = useState(false);
  /** Удаление / permissions — точечно, не весь экран. */
  const [actionBusy, setActionBusy] = useState(false);
  const [status, setStatus] = useState(t("allTables.ready"));
  const [error, setError] = useState<string | null>(null);
  const [permsFor, setPermsFor] = useState<TableRow | null>(null);
  const [perms, setPerms] = useState<TableUserPerm[]>([]);

  async function load() {
    if (loading) return;
    setLoading(true);
    setError(null);
    setStatus(t("allTables.loading"));
    try {
      const res = await hostApi.listOwnedSheets({ sharedOnly });
      setCanManage(!!res.canManage);
      const items = [...(res.items ?? [])].sort(
        (a, b) => (b.createdTs ?? 0) - (a.createdTs ?? 0),
      );
      setTables(items);
      setStatus(
        t("allTables.loaded").replace("{count}", String(items.length)),
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStatus(t("common.error"));
    } finally {
      setLoading(false);
    }
  }

  async function remove(row: TableRow) {
    if (!canManage) {
      setError(t("allTables.noManage"));
      return;
    }
    if (actionBusy) return;
    const ok = await confirm(
      t("allTables.deleteConfirm").replace("{name}", row.name),
      t("allTables.deleteTitle"),
    );
    if (!ok) return;
    setActionBusy(true);
    try {
      await hostApi.deleteSheet(row.id);
      setTables((prev) => prev.filter((x) => x.id !== row.id));
      setStatus(t("allTables.deleted"));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setActionBusy(false);
    }
  }

  async function showUsers(row: TableRow) {
    if (actionBusy) return;
    setActionBusy(true);
    setError(null);
    try {
      setPerms(await hostApi.getSheetPermissions(row.id));
      setPermsFor(row);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setActionBusy(false);
    }
  }

  async function revoke(permissionId: string) {
    if (!permsFor || actionBusy) return;
    setActionBusy(true);
    try {
      await hostApi.removePermission(permsFor.id, permissionId);
      setPerms(await hostApi.getSheetPermissions(permsFor.id));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setActionBusy(false);
    }
  }

  const accessLevels = useMemo(() => {
    const set = new Set(
      tables.map((x) => x.accessStatus).filter(Boolean) as string[],
    );
    return Array.from(set).sort();
  }, [tables]);

  const accessOptions = useMemo(
    () => [
      { value: "", label: t("allTables.accessAll") },
      ...accessLevels.map((l) => ({ value: l, label: l })),
    ],
    [accessLevels, t],
  );

  const visible = useMemo(() => {
    const f = filter.trim().toLowerCase();
    return tables.filter((row) => {
      if (
        f &&
        !row.name.toLowerCase().includes(f) &&
        !row.id.toLowerCase().includes(f)
      ) {
        return false;
      }
      if (accessFilter && row.accessStatus !== accessFilter) return false;
      return true;
    });
  }, [tables, filter, accessFilter]);

  const statusText =
    tables.length === 0
      ? status
      : !filter && !accessFilter
        ? t("allTables.loaded").replace("{count}", String(tables.length))
        : visible.length === 0
          ? t("allTables.noMatch").replace("{total}", String(tables.length))
          : t("allTables.filtered")
              .replace("{visible}", String(visible.length))
              .replace("{total}", String(tables.length));

  return (
    <div className="all-tables-page">
      <h1>{t("allTables.title")}</h1>
      {error && <p className="all-tables-error">{error}</p>}

      <div className="all-tables-card card combo-clip-ok">
        <p className="all-tables-intro">{t("allTables.intro")}</p>
        <div className="all-tables-toolbar">
          <input
            type="text"
            className="input all-tables-search"
            title={t("allTables.searchTitle")}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={t("allTables.searchTitle")}
          />
          <AppComboBox
            className="all-tables-access-combo"
            value={accessFilter}
            options={accessOptions}
            onChange={setAccessFilter}
            title={t("allTables.accessFilterTitle")}
          />
          <button
            type="button"
            className="btn btn-primary all-tables-load"
            disabled={loading}
            onClick={() => void load()}
          >
            {t("allTables.load")}
          </button>
        </div>
        <label
          className="all-tables-shared"
          title={t("allTables.sharedOnlyTitle")}
        >
          <input
            type="checkbox"
            checked={sharedOnly}
            onChange={(e) => setSharedOnly(e.target.checked)}
          />
          <span>{t("allTables.sharedOnly")}</span>
        </label>
        <p className="all-tables-status">
          {loading ? t("allTables.loading") : statusText}
        </p>
      </div>

      <h2>{t("allTables.listTitle")}</h2>
      <div className="all-tables-card card all-tables-list-card">
        {visible.length === 0 ? (
          <p className="muted">{t("allTables.empty")}</p>
        ) : (
          <ul className="all-tables-list">
            {visible.map((row) => (
              <li key={row.id} className="all-tables-row">
                <div className="all-tables-row-main">
                  <div className="all-tables-row-name">{row.name}</div>
                  <div className="all-tables-row-id">{row.id}</div>
                  <div className="all-tables-row-meta">
                    {t("allTables.createdAt").replace(
                      "{date}",
                      row.created || t("allTables.unknown"),
                    )}
                  </div>
                </div>
                <div className="all-tables-row-actions">
                  <span className="all-tables-access">{row.accessStatus}</span>
                  <span className="all-tables-users">
                    {t("allTables.usersCount").replace(
                      "{count}",
                      String(row.usersCount ?? 0),
                    )}
                  </span>
                  <button
                    type="button"
                    className="btn btn-outline all-tables-btn"
                    disabled={actionBusy}
                    onClick={() => void showUsers(row)}
                  >
                    {t("allTables.users")}
                  </button>
                  <button
                    type="button"
                    className="btn btn-outline all-tables-btn"
                    title={t("allTables.copyLinkTitle")}
                    onClick={() => {
                      const link =
                        row.url ||
                        `https://docs.google.com/spreadsheets/d/${row.id}`;
                      void navigator.clipboard?.writeText(link);
                    }}
                  >
                    {t("allTables.copyLink")}
                  </button>
                  <button
                    type="button"
                    className="btn btn-outline all-tables-btn"
                    onClick={() => {
                      const link =
                        row.url ||
                        `https://docs.google.com/spreadsheets/d/${row.id}`;
                      void openUrl(link).catch((e) =>
                        setError(e instanceof Error ? e.message : String(e)),
                      );
                    }}
                  >
                    {t("allTables.open")}
                  </button>
                  {canManage && (
                    <button
                      type="button"
                      className="btn btn-danger all-tables-btn"
                      disabled={actionBusy}
                      onClick={() => void remove(row)}
                    >
                      {t("allTables.delete")}
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      {permsFor && (
        <TableUsersDialog
          tableName={permsFor.name}
          users={perms}
          canManage={canManage}
          busy={actionBusy}
          onRemove={(id) => void revoke(id)}
          onClose={() => setPermsFor(null)}
        />
      )}
      {dialogs}
    </div>
  );
}
