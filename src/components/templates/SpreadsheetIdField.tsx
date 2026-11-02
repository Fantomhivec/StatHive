import { useState } from "react";
import { hostApi } from "../../bridge/hostApi";
import AppComboBox from "../AppComboBox";
import { t } from "../../i18n";

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

/** Унифицированный блок: ID/URL таблицы + лист + «Загрузить листы». */
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

  return (
    <>
      <div className="field">
        <label htmlFor={sidId}>
          {spreadsheetIdLabel}
          {titleShown ? <span className="tpl-gs-title"> — {titleShown}</span> : null}
        </label>
        <input
          id={sidId}
          value={spreadsheetId}
          disabled={disabled || busy}
          onChange={(e) => {
            onSpreadsheetIdChange(e.target.value);
            onSpreadsheetTitleChange?.("");
          }}
          onBlur={() => void normalizeOnBlur()}
          placeholder={t("templates.spreadsheet.idPlaceholder")}
          autoComplete="off"
        />
      </div>
      {!hideSheet && (
        <>
          <div className="field">
            <label htmlFor={snameId}>{sheetNameLabel}</label>
            {loadedSheetOptions.length > 0 ? (
              <AppComboBox
                editable
                value={sheetName}
                disabled={disabled || busy}
                options={loadedSheetOptions}
                onChange={onSheetNameChange}
                placeholder={t("templates.spreadsheet.sheetPlaceholder")}
              />
            ) : (
              <input
                id={snameId}
                value={sheetName}
                disabled={disabled || busy}
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
              disabled={disabled || busy || !spreadsheetId.trim()}
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
