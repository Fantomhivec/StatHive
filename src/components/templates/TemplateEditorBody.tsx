import { useMemo } from "react";
import type { ScriptGroupDto, TemplateDto } from "../../bridge/hostApi";
import AppComboBox, { type AppComboGroup } from "../AppComboBox";
import OzonIdModeCard from "./OzonIdModeCard";
import ScriptExtraCards from "./ScriptExtraCards";
import SpreadsheetIdField from "./SpreadsheetIdField";
import {
  hasUnifiedSources,
  type ScriptVisibility,
} from "./scriptVisibility";
import TemplateDateRangeCard from "./TemplateDateRangeCard";
import TemplateEditorWriteCards from "./TemplateEditorWriteCards";
import UnifiedSourcesCard from "./UnifiedSourcesCard";
import { useT } from "../../i18n";

export type TemplateEditorBodyProps = {
  tpl: TemplateDto;
  saving: boolean;
  excelOnly: boolean;
  scriptGroups: ScriptGroupDto[];
  visibility: ScriptVisibility;
  sheetOptions: string[];
  tabs: string[];
  tabName: string;
  onTabNameChange: (tab: string) => void;
  onPatch: (partial: Partial<TemplateDto>) => void;
  onScriptChange: (key: string) => void;
  onSheetOptionsChange: (options: string[]) => void;
  onMessage: (msg: string | null) => void;
  onOpenCabinets: () => void;
  onOpenFields: () => void;
  onOpenExport: () => void;
  onOpenSubjectsFilters: () => void;
};

export default function TemplateEditorBody({
  tpl,
  saving,
  excelOnly,
  scriptGroups,
  visibility,
  sheetOptions,
  tabs,
  tabName,
  onTabNameChange,
  onPatch,
  onScriptChange,
  onSheetOptionsChange,
  onMessage,
  onOpenCabinets,
  onOpenFields,
  onOpenExport,
  onOpenSubjectsFilters,
}: TemplateEditorBodyProps) {
  const t = useT();

  const comboGroups: AppComboGroup[] = useMemo(
    () =>
      scriptGroups.map((g) => ({
        label: g.name,
        options: g.scripts.map((s) => ({
          value: s.key,
          label: s.title,
          secondary: s.api_method || undefined,
        })),
      })),
    [scriptGroups],
  );

  const selectedCount = (tpl.selected_fields ?? []).filter((f) => f.selected).length;
  const cabCount = tpl.cabinets?.length ?? 0;

  const showExtras =
    visibility.showCardsMoveTrashHint ||
    visibility.showCardsUpdateModes ||
    visibility.showTnvedCustomIds ||
    visibility.showObjectCharcsCustomIds ||
    visibility.showPromotionsAllPromo ||
    visibility.showSubjectsFilters ||
    visibility.showOzonIdMode ||
    hasUnifiedSources(visibility) ||
    !visibility.hideCabinets ||
    !visibility.hideFields;

  return (
    <>
      {!visibility.hideExportToExcelCard ? (
        <div className="tpl-dialog-card tpl-excel-card">
          <label className="toggle-switch tpl-toggle-row">
            <input
              type="checkbox"
              checked={excelOnly ? true : !!tpl.export_to_excel}
              disabled={saving || excelOnly}
              onChange={(e) => {
                if (excelOnly) return;
                onPatch({ export_to_excel: e.target.checked });
              }}
            />
            <span className="toggle-track" />
            <span className="tpl-toggle-label">{t("templates.editor.createExcelFile")}</span>
          </label>
          {excelOnly ? (
            <p className="muted u-mt-8">{t("templates.editor.excelOnlyLocked")}</p>
          ) : null}
        </div>
      ) : null}

      <div className="tpl-dialog-card">
        <h3 className="tpl-dialog-card-header">{t("templates.editor.mainSettings")}</h3>
        <div className="field">
          <label htmlFor="tpl-name">{t("common.name")}</label>
          <input
            id="tpl-name"
            value={tpl.name}
            disabled={saving}
            onChange={(e) => onPatch({ name: e.target.value })}
          />
        </div>
        <div className="field">
          <label>{t("templates.editor.tab")}</label>
          <AppComboBox
            options={tabs.map((name) => ({ value: name, label: name }))}
            value={tabName}
            disabled={saving || tabs.length === 0}
            onChange={onTabNameChange}
          />
        </div>
        <div className="field tpl-field-script">
          <label>{t("templates.editor.script")}</label>
          {comboGroups.length === 0 ? (
            <p className="page-sub">{t("templates.editor.noScripts")}</p>
          ) : (
            <AppComboBox
              className={
                "tpl-script-combo app-combo--emphasis" +
                (!(tpl.script_key ?? "").trim() ? " app-combo--emphasis-empty" : "")
              }
              groups={comboGroups}
              value={tpl.script_key}
              disabled={saving}
              onChange={(key) => void onScriptChange(key)}
              placeholder={t("templates.editor.pickScript")}
            />
          )}
        </div>
        {!visibility.sheetAsSource && !tpl.export_to_excel ? (
          <>
            <p className="tpl-section-title">{t("templates.editor.writeTarget")}</p>
            <SpreadsheetIdField
              spreadsheetId={tpl.spreadsheet_id}
              sheetName={tpl.sheet_name}
              spreadsheetTitle={tpl.spreadsheet_title}
              sheetOptions={sheetOptions}
              disabled={saving}
              onSpreadsheetIdChange={(id) => {
                onPatch({ spreadsheet_id: id });
                onSheetOptionsChange([]);
              }}
              onSheetNameChange={(name) => onPatch({ sheet_name: name })}
              onSheetOptionsChange={onSheetOptionsChange}
              onSpreadsheetTitleChange={(title) => onPatch({ spreadsheet_title: title })}
              onMessage={onMessage}
            />
          </>
        ) : null}
        {visibility.sheetAsSource ? (
          <>
            <p className="tpl-section-title">{t("templates.editor.dataSource")}</p>
            <SpreadsheetIdField
              spreadsheetId={tpl.spreadsheet_id}
              sheetName={tpl.sheet_name}
              spreadsheetTitle={tpl.spreadsheet_title}
              sheetOptions={sheetOptions}
              disabled={saving}
              onSpreadsheetIdChange={(id) => {
                onPatch({ spreadsheet_id: id });
                onSheetOptionsChange([]);
              }}
              onSheetNameChange={(name) => onPatch({ sheet_name: name })}
              onSheetOptionsChange={onSheetOptionsChange}
              onSpreadsheetTitleChange={(title) => onPatch({ spreadsheet_title: title })}
              onMessage={onMessage}
            />
          </>
        ) : null}
      </div>

      {visibility.requiresDateRange || visibility.requiresSingleDate ? (
        <TemplateDateRangeCard
          tpl={tpl}
          period={visibility.requiresDateRange}
          disabled={saving}
          onPatch={onPatch}
        />
      ) : null}

      {showExtras ? (
        <section className="tpl-ed-section">
          <h3 className="tpl-ed-section-title">{t("templates.editor.extra")}</h3>
          <div className="tpl-ed-section-stack">
            <ScriptExtraCards
              visibility={visibility}
              tpl={tpl}
              disabled={saving}
              onPatch={onPatch}
              onOpenSubjectsFilters={onOpenSubjectsFilters}
            />

            {visibility.showOzonIdMode ? (
              <OzonIdModeCard tpl={tpl} disabled={saving} onPatch={onPatch} />
            ) : null}

            <UnifiedSourcesCard
              visibility={visibility}
              tpl={tpl}
              disabled={saving}
              onPatch={onPatch}
              onMessage={onMessage}
            />

            {!visibility.hideCabinets || !visibility.hideFields ? (
              <div className="form-actions">
                {!visibility.hideCabinets ? (
                  <button
                    type="button"
                    className="btn btn-outline"
                    disabled={saving}
                    onClick={onOpenCabinets}
                  >
                    {t("templates.editor.cabinetsCount", { count: cabCount })}
                  </button>
                ) : null}
                {!visibility.hideFields ? (
                  <button
                    type="button"
                    className="btn btn-outline"
                    disabled={saving}
                    onClick={onOpenFields}
                  >
                    {t("templates.editor.fieldsCount", { count: selectedCount })}
                  </button>
                ) : null}
              </div>
            ) : null}
          </div>
        </section>
      ) : null}

      <TemplateEditorWriteCards
        tpl={tpl}
        visibility={visibility}
        saving={saving}
        onPatch={onPatch}
        onOpenExport={onOpenExport}
      />
    </>
  );
}
