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

import { useMemo, useState } from "react";
import type { AccessMatrixDto, ScriptCatalogItem } from "../bridge/hostApi";
import { useT } from "../i18n";

function scriptLabel(s: ScriptCatalogItem): string {
  return (s.title || s.key).trim();
}

function compareScripts(a: ScriptCatalogItem, b: ScriptCatalogItem): number {
  const byTitle = scriptLabel(a).localeCompare(scriptLabel(b), "ru", {
    sensitivity: "base",
  });
  if (byTitle !== 0) return byTitle;
  return a.key.localeCompare(b.key, "en", { sensitivity: "base" });
}

const CALC_FLAGS = ["view", "settings", "tables", "kgt", "excel"] as const;
const APP_FLAGS = ["keysCheck", "users", "allTables"] as const;

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
  const [scriptQuery, setScriptQuery] = useState("");
  const flags = normalizeFlags(calcAccess, calculator);
  const appFlags = normalizeAppFlags(appAccess);

  const sortedCatalog = useMemo(
    () => [...catalog].sort(compareScripts),
    [catalog],
  );

  const visibleCatalog = useMemo(() => {
    const q = scriptQuery.trim().toLocaleLowerCase("ru");
    if (!q) return sortedCatalog;
    return sortedCatalog.filter((s) => {
      const title = scriptLabel(s).toLocaleLowerCase("ru");
      const key = s.key.toLocaleLowerCase("ru");
      return title.includes(q) || key.includes(q);
    });
  }, [sortedCatalog, scriptQuery]);

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
      case "allTables":
        return t("nav.allTables");
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

      <section className="card card--md card--stack users-access-section">
        <div className="users-section-head">
          <h3 className="users-section-title">{t("users.access.calcTitle")}</h3>
          <p className="users-section-desc">{t("users.access.calcHint")}</p>
        </div>
        <div className="wb-card wb-card--control users-flag-grid">
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
      </section>

      <section className="card card--md card--stack users-access-section">
        <div className="users-section-head">
          <h3 className="users-section-title">{t("users.access.appTitle")}</h3>
          <p className="users-section-desc">{t("users.access.appHint")}</p>
        </div>
        <div className="wb-card wb-card--control users-flag-grid users-flag-grid--app">
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
      </section>

      <section className="card card--md card--stack users-access-section users-scripts-block users-scripts-block--catalog">
        <div className="users-section-head">
          <h3 className="users-section-title">{t("users.access.scriptsTitle")}</h3>
          <p className="users-section-desc">{t("users.access.scriptsHint")}</p>
        </div>
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
        <div className="field users-scripts-search">
          <label htmlFor="access-editor-script-search">{t("users.access.scriptsSearchLabel")}</label>
          <input
            id="access-editor-script-search"
            type="search"
            className="input"
            value={scriptQuery}
            disabled={busy || catalog.length === 0}
            placeholder={t("users.access.scriptsSearchPlaceholder")}
            onChange={(e) => setScriptQuery(e.target.value)}
            autoComplete="off"
          />
        </div>
        <div className="wb-card wb-card--control users-script-catalog">
          {catalog.length === 0 ? (
            <p className="muted u-m-0">{t("users.access.catalogEmpty")}</p>
          ) : visibleCatalog.length === 0 ? (
            <p className="muted u-m-0">{t("users.access.scriptsNoMatch")}</p>
          ) : (
            visibleCatalog.map((s) => (
              <label key={s.key} className="users-check-row">
                <input
                  type="checkbox"
                  checked={keyChecked(s.key)}
                  disabled={busy || scripts.all}
                  onChange={() => toggleKey(s.key)}
                />
                <span title={s.key}>
                  {scriptLabel(s)}
                  {s.memberOk ? ` (${t("users.access.memberOkBadge")})` : ""}
                </span>
              </label>
            ))
          )}
        </div>
      </section>
    </div>
  );
}
