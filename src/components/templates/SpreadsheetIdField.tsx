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

import { useState } from "react";
import { hostApi, openUrl } from "../../bridge/hostApi";
import AppComboBox from "../AppComboBox";
import { t } from "../../i18n";
import { sheetsUrl } from "../../pages/users/usersPageCache";
import "./templates-dialogs.css";

type Props = {
  spreadsheetId: string;
  sheetName: string;
  sheetOptions: string[];
  /** Кэш названия документа (показывается над полем ID). */
  spreadsheetTitle?: string;
  disabled?: boolean;
  spreadsheetIdLabel?: string;
  sheetNameLabel?: string;
  /** Префикс id полей (когда на форме два блока Spreadsheet). */
  idPrefix?: string;
  /** Только ID/URL (калькулятор: ссылка таблицы без выбора листа). */
  hideSheet?: boolean;
  onSpreadsheetIdChange: (id: string) => void;
  onSheetNameChange: (name: string) => void;
  onSheetOptionsChange: (sheets: string[]) => void;
  onSpreadsheetTitleChange?: (title: string) => void;
  onMessage?: (msg: string | null) => void;
};

/** Унифицированный блок: ID/URL таблицы + «Открыть» + лист + «Загрузить листы». */
export default function SpreadsheetIdField({
  spreadsheetId,
  sheetName,
  sheetOptions,
  spreadsheetTitle = "",
  disabled,
  spreadsheetIdLabel = t("templates.spreadsheet.tableIdOrUrl"),
  sheetNameLabel = t("common.sheet"),
  idPrefix = "tpl",
  hideSheet = false,
  onSpreadsheetIdChange,
  onSheetNameChange,
  onSheetOptionsChange,
  onSpreadsheetTitleChange,
  onMessage,
}: Props) {
  const [busy, setBusy] = useState(false);
  const [openBusy, setOpenBusy] = useState(false);
  const sidId = `${idPrefix}-ssid`;
  const snameId = `${idPrefix}-sname`;
  const titleShown = spreadsheetTitle.trim();

  async function normalizeOnBlur() {
    const raw = spreadsheetId.trim();
    if (!raw) return;
    try {
      const res = await hostApi.spreadsheetNormalize(raw);
      if (res.changed) onSpreadsheetIdChange(res.spreadsheet_id);
    } catch {
      /* ignore */
    }
  }

  async function openSpreadsheet() {
    const raw = spreadsheetId.trim();
    if (!raw) return;
    setOpenBusy(true);
    onMessage?.(null);
    try {
      const res = await hostApi.spreadsheetNormalize(raw);
      const id = (res.spreadsheet_id || raw).trim();
      if (res.changed) onSpreadsheetIdChange(id);
      if (!id) {
        onMessage?.(t("templates.spreadsheet.specifyTableFirst"));
        return;
      }
      await openUrl(sheetsUrl(id));
    } catch (err) {
      onMessage?.(err instanceof Error ? err.message : String(err));
    } finally {
      setOpenBusy(false);
    }
  }

  async function loadSheets() {
    setBusy(true);
    onMessage?.(null);
    try {
      const id = spreadsheetId.trim();
      if (!id) {
        onMessage?.(t("templates.spreadsheet.specifyTableFirst"));
        return;
      }
      // googleListSheets уже нормализует ID/URL (как spreadsheetNormalize).
      const res = await hostApi.googleListSheets(id);
      onSpreadsheetIdChange(res.spreadsheet_id);
      onSheetOptionsChange(res.sheets);
      if (res.title != null && onSpreadsheetTitleChange) {
        onSpreadsheetTitleChange(res.title);
      }
      if (res.sheets.length && !res.sheets.includes(sheetName)) {
        onSheetNameChange(res.sheets[0]);
      }
      if (res.sheets.length === 0) {
        onMessage?.(t("templates.spreadsheet.noSheets"));
      }
    } catch (err) {
      onMessage?.(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const loadedSheetOptions =
    sheetOptions.length > 0 ? sheetOptions.map((n) => ({ value: n, label: n })) : [];
  const locked = disabled || busy || openBusy;

  return (
    <>
      <div className="field">
        <label htmlFor={sidId}>
          {spreadsheetIdLabel}
          {titleShown ? <span className="tpl-gs-title"> — {titleShown}</span> : null}
        </label>
        <div className="tpl-ssid-row">
          <input
            id={sidId}
            className="tpl-ssid-input"
            value={spreadsheetId}
            disabled={locked}
            onChange={(e) => {
              onSpreadsheetIdChange(e.target.value);
              onSpreadsheetTitleChange?.("");
            }}
            onBlur={() => void normalizeOnBlur()}
            placeholder={t("templates.spreadsheet.idPlaceholder")}
            autoComplete="off"
          />
          <button
            type="button"
            className="btn btn-outline btn-sm"
            disabled={locked || !spreadsheetId.trim()}
            title={t("users.openSheet")}
            onClick={() => void openSpreadsheet()}
          >
            {openBusy ? t("common.loading") : t("common.open")}
          </button>
        </div>
      </div>
      {!hideSheet && (
        <>
          <div className="field">
            <label htmlFor={snameId}>{sheetNameLabel}</label>
            {loadedSheetOptions.length > 0 ? (
              <AppComboBox
                editable
                value={sheetName}
                disabled={locked}
                options={loadedSheetOptions}
                onChange={onSheetNameChange}
                placeholder={t("templates.spreadsheet.sheetPlaceholder")}
              />
            ) : (
              <input
                id={snameId}
                value={sheetName}
                disabled={locked}
                onChange={(e) => onSheetNameChange(e.target.value)}
                placeholder={t("templates.spreadsheet.sheetPlaceholder")}
                autoComplete="off"
              />
            )}
          </div>
          <div className="form-actions">
            <button
              className="btn btn-outline"
              type="button"
              disabled={locked || !spreadsheetId.trim()}
              onClick={() => void loadSheets()}
            >
              {busy ? t("common.loading") : t("templates.spreadsheet.loadSheets")}
            </button>
          </div>
        </>
      )}
    </>
  );
}
