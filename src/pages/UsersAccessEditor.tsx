import type { AccessMatrixDto, ScriptCatalogItem } from "../bridge/hostApi";
import { useT } from "../i18n";

const CALC_FLAGS = ["view", "settings", "tables", "kgt", "excel"] as const;
const APP_FLAGS = ["keysCheck", "users"] as const;

type CalcFlag = (typeof CALC_FLAGS)[number];
type AppFlag = (typeof APP_FLAGS)[number];

type Props = {
  id: string;
  title: string;
  calculator: boolean;
  calcAccess?: string[];
  appAccess?: string[];
  scripts: AccessMatrixDto;
  catalog: ScriptCatalogItem[];
  idLocked?: boolean;
  /** Пресет: Admin вводит только название; id создаётся на сервере. */
  autoId?: boolean;
  busy?: boolean;
  onChange: (next: {
    id: string;
    title: string;
    calculator: boolean;
    calcAccess: string[];
    appAccess: string[];
    scripts: AccessMatrixDto;
  }) => void;
};

function normalizeFlags(raw: string[] | undefined, _calculator: boolean): string[] {
  const set = new Set(
    (raw ?? [])
      .map((s) => s.trim().toLowerCase())
      .filter((s) => (CALC_FLAGS as readonly string[]).includes(s)),
  );
  // Пустой набор при сохранении = без калькулятора (старые строки листа читаются отдельно).
  return CALC_FLAGS.filter((f) => set.has(f));
}

function normalizeAppFlags(raw: string[] | undefined): string[] {
  const set = new Set(
    (raw ?? [])
      .map((s) => s.trim())
      .map((s) => (s.toLowerCase() === "keyscheck" ? "keysCheck" : s.toLowerCase() === "users" ? "users" : s))
      .filter((s) => (APP_FLAGS as readonly string[]).includes(s as AppFlag)),
  );
  return APP_FLAGS.filter((f) => set.has(f));
}

export default function UsersAccessEditor({
  id,
  title,
  calculator,
  calcAccess,
  appAccess,
  scripts,
  catalog,
  idLocked,
  autoId,
  busy,
  onChange,
}: Props) {
  const t = useT();
  const flags = normalizeFlags(calcAccess, calculator);
  const appFlags = normalizeAppFlags(appAccess);

  function emit(p: {
    id?: string;
    title?: string;
    calcAccess?: string[];
    appAccess?: string[];
    scripts?: AccessMatrixDto;
  }) {
    const nextFlags = p.calcAccess ?? flags;
    const hasTab = nextFlags.some((f) => f !== "excel");
    onChange({
      id: p.id ?? id,
      title: p.title ?? title,
      calculator: hasTab,
      calcAccess: nextFlags,
      appAccess: p.appAccess ?? appFlags,
      scripts: p.scripts ?? scripts,
    });
  }

  function toggleFlag(flag: CalcFlag) {
    const set = new Set(flags);
    if (set.has(flag)) set.delete(flag);
    else set.add(flag);
    emit({ calcAccess: CALC_FLAGS.filter((f) => set.has(f)) });
  }

  function toggleAppFlag(flag: AppFlag) {
    const set = new Set(appFlags);
    if (set.has(flag)) set.delete(flag);
    else set.add(flag);
    emit({ appAccess: APP_FLAGS.filter((f) => set.has(f)) });
  }

  function toggleKey(key: string) {
    if (scripts.all) return;
    const set = new Set(scripts.keys);
    if (set.has(key)) set.delete(key);
    else set.add(key);
    emit({ scripts: { all: false, keys: [...set] } });
  }

  function selectAllKeys() {
    emit({ scripts: { all: false, keys: catalog.map((s) => s.key) } });
  }

  function clearKeys() {
    emit({ scripts: { all: false, keys: [] } });
  }

  const keyChecked = (key: string) => scripts.all || scripts.keys.includes(key);

  const flagLabel = (flag: CalcFlag) => {
    switch (flag) {
      case "view":
        return t("nav.calculator");
      case "settings":
        return t("nav.calculatorSettings");
      case "tables":
        return t("nav.calculatorTables");
      case "kgt":
        return t("nav.calculatorKgtRules");
      case "excel":
        return t("users.access.calcExcel");
      default:
        return flag;
    }
  };

  const appFlagLabel = (flag: AppFlag) => {
    switch (flag) {
      case "keysCheck":
        return t("nav.keysCheck");
      case "users":
        return t("nav.users");
      default:
        return flag;
    }
  };

  return (
    <div className="users-access-editor">
      {!autoId && (
        <div className="field">
          <label htmlFor="access-editor-id">Id</label>
          <input
            id="access-editor-id"
            value={id}
            disabled={busy || idLocked}
            onChange={(e) => emit({ id: e.target.value })}
          />
        </div>
      )}
      <div className="field">
        <label htmlFor="access-editor-title">{t("common.title")}</label>
        <input
          id="access-editor-title"
          value={title}
          disabled={busy}
          onChange={(e) => emit({ title: e.target.value })}
        />
      </div>
      <div className="users-scripts-block">
        <p className="muted users-scripts-hint">{t("users.access.calcHint")}</p>
        <div className="plate plate--pad users-flag-grid">
          {CALC_FLAGS.map((flag) => (
            <label key={flag} className="users-check-row">
              <input
                type="checkbox"
                checked={flags.includes(flag)}
                disabled={busy}
                onChange={() => toggleFlag(flag)}
              />
              <span>{flagLabel(flag)}</span>
            </label>
          ))}
        </div>
        {flags.includes("tables") ? (
          <p className="muted users-scripts-hint">{t("users.access.tablesDriveHint")}</p>
        ) : null}
      </div>

      <div className="users-scripts-block">
        <p className="muted users-scripts-hint">{t("users.access.appHint")}</p>
        <div className="plate plate--pad users-flag-grid users-flag-grid--app">
          {APP_FLAGS.map((flag) => (
            <label key={flag} className="users-check-row">
              <input
                type="checkbox"
                checked={appFlags.includes(flag)}
                disabled={busy}
                onChange={() => toggleAppFlag(flag)}
              />
              <span>{appFlagLabel(flag)}</span>
            </label>
          ))}
        </div>
      </div>

      <div className="users-scripts-block users-scripts-block--catalog">
        <p className="muted users-scripts-hint">{t("users.access.scriptsHint")}</p>
        <label className="users-check-row">
          <input
            type="checkbox"
            checked={scripts.all}
            disabled={busy}
            onChange={(e) =>
              emit({
                scripts: e.target.checked
                  ? { all: true, keys: [] }
                  : { all: false, keys: [] },
              })
            }
          />
          <span>{t("users.access.allScripts")}</span>
        </label>
        {!scripts.all && (
          <div className="users-scripts-bulk">
            <button type="button" className="btn btn-ghost btn-sm" disabled={busy || catalog.length === 0} onClick={selectAllKeys}>
              {t("common.selectAll")}
            </button>
            <button type="button" className="btn btn-ghost btn-sm" disabled={busy || scripts.keys.length === 0} onClick={clearKeys}>
              {t("common.deselectAll")}
            </button>
            <span className="muted">
              {t("common.selectedCount", {
                selected: scripts.keys.length,
                total: catalog.length,
              })}
            </span>
          </div>
        )}
        <div className="plate plate--pad users-script-catalog">
          {catalog.length === 0 ? (
            <p className="muted u-m-0">{t("users.access.catalogEmpty")}</p>
          ) : (
            catalog.map((s) => (
              <label key={s.key} className="users-check-row">
                <input
                  type="checkbox"
                  checked={keyChecked(s.key)}
                  disabled={busy || scripts.all}
                  onChange={() => toggleKey(s.key)}
                />
                <span title={s.key}>
                  {s.title || s.key}
                  {s.memberOk ? ` (${t("users.access.memberOkBadge")})` : ""}
                </span>
              </label>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
