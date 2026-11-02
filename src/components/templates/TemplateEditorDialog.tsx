import { lazy, Suspense, useCallback, useEffect, useMemo, useState } from "react";
import {
  ensureTabsLayout,
  EnvKeyRow,
  hostApi,
  ScriptGroupDto,
  TemplateDto,
} from "../../bridge/hostApi";
import { normalizeFieldRows, type FieldRow } from "./templateNormalize";
import {
  visibilityForScript,
  visibilityForTemplate,
} from "./scriptVisibility";
import { normalizeTpl } from "./templateNormalize";
import type { SyncSettingsDto } from "../../bridge/hostApi.types";
import "./templates-dialogs.css";
import { useT } from "../../i18n";
import { buildTemplateSavePayload } from "./buildTemplateSavePayload";
import TemplateEditorBody from "./TemplateEditorBody";

const CabinetSelectorDialog = lazy(() => import("./CabinetSelectorDialog"));
const ExportSettingsDialog = lazy(() => import("./ExportSettingsDialog"));
const FieldSelectorDialog = lazy(() => import("./FieldSelectorDialog"));
const SubjectsFiltersDialog = lazy(() =>
  import("./ScriptExtraCards").then((m) => ({ default: m.SubjectsFiltersDialog })),
);

type Props = {
  templateId?: string | null;
  marketplace: "wb" | "ozon";
  /** Вкладка при открытии (текущая selected_tab для нового / контекст списка). */
  tabName: string;
  tabs: string[];
  onClose: () => void;
  onSaved: () => void;
};

export default function TemplateEditorDialog({
  templateId,
  marketplace,
  tabName,
  tabs,
  onClose,
  onSaved,
}: Props) {
  const t = useT();
  const [busy, setBusy] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [excelOnly, setExcelOnly] = useState(false);
  const [scriptGroups, setScriptGroups] = useState<ScriptGroupDto[]>([]);
  const [cabinets, setCabinets] = useState<EnvKeyRow[]>([]);
  const [tpl, setTpl] = useState<TemplateDto | null>(null);
  const [editTabName, setEditTabName] = useState(tabName);
  const [sheetOptions, setSheetOptions] = useState<string[]>([]);
  const [fieldsOpen, setFieldsOpen] = useState(false);
  const [cabsOpen, setCabsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [subjectsFiltersOpen, setSubjectsFiltersOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      setBusy(true);
      setError(null);
      try {
        const [grouped, env, loaded, layout] = await Promise.all([
          hostApi.scriptsListGrouped(marketplace),
          hostApi
            .envListCabinetNames()
            .catch(() => ({ path: "", rows: [] as EnvKeyRow[] })),
          hostApi.templatesGet(templateId ?? undefined, undefined, marketplace, tabName),
          ensureTabsLayout().catch(() => null),
        ]);
        if (cancelled) return;
        const forceExcel = !!layout?.templatesExcelOnly;
        setExcelOnly(forceExcel);
        setScriptGroups(grouped.groups ?? []);
        const marketFilter = marketplace === "ozon" ? "OZON" : "WB";
        const cabs = env.rows.filter((r) => r.marketplace === marketFilter);
        setCabinets(cabs);
        const loadedTab = (loaded.template?.tab_name ?? "").trim();
        const initialTab =
          templateId && loadedTab
            ? loadedTab
            : tabName.trim() || loadedTab || tabs[0] || t("templates.defaultTab");
        const next = normalizeTpl(loaded.template, marketplace, initialTab);
        if (!next.cabinets?.length && cabs.length === 1) {
          next.cabinets = [cabs[0].cabinet];
        }
        if (forceExcel) {
          next.export_to_excel = true;
        }
        setEditTabName(initialTab);
        setTpl(next);
        setSheetOptions([]);
        // Подтянуть название Google-документа, если есть ID, а title пуст.
        const sid = (next.spreadsheet_id ?? "").trim();
        if (sid && !(next.spreadsheet_title ?? "").trim()) {
          void hostApi
            .googleListSheets(sid)
            .then(async (res) => {
              if (cancelled) return;
              const title = (res.title ?? "").trim();
              if (!title) return;
              setTpl((prev) => (prev ? { ...prev, spreadsheet_title: title } : prev));
              if (next.id) {
                try {
                  await hostApi.templatesSetSpreadsheetTitle(next.id, title);
                } catch {
                  /* ignore */
                }
              }
            })
            .catch(() => {
              /* ignore */
            });
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      } finally {
        if (!cancelled) setBusy(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [templateId, marketplace, tabName]);

  const visibility = useMemo(
    () =>
      visibilityForTemplate(tpl?.script_key ?? "", tpl?.use_custom_subject_ids),
    [tpl?.script_key, tpl?.use_custom_subject_ids],
  );

  function patch(partial: Partial<TemplateDto>) {
    setTpl((prev) => (prev ? { ...prev, ...partial } : prev));
  }

  async function onScriptChange(key: string) {
    if (!tpl || key === tpl.script_key) return;
    setBusy(true);
    try {
      const res = await hostApi.templatesListFields(key, []);
      const vis = visibilityForScript(key);
      const partial: Partial<TemplateDto> = {
        script_key: key,
        selected_fields: normalizeFieldRows(res.fields),
      };
      if (vis.showOzonIdMode && !(tpl.id_mode ?? "").trim()) {
        partial.id_mode = "sku";
      }
      if (vis.showOzonIdRangeByMode) {
        const mode = ((tpl.id_mode ?? partial.id_mode ?? "sku") as string)
          .trim()
          .toLowerCase();
        if (mode === "offer_id" && !(tpl.offer_id_source_range ?? "").trim()) {
          partial.offer_id_source_range = "B2:B";
          partial.id_source_range = "B2:B";
        } else if (mode === "product_id" && !(tpl.product_id_source_range ?? "").trim()) {
          partial.product_id_source_range = "B2:B";
          partial.id_source_range = "B2:B";
        } else if (!(tpl.sku_source_range ?? "").trim()) {
          partial.sku_source_range = "B2:B";
          partial.id_source_range = "B2:B";
        }
      }
      patch(partial);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  const save = useCallback(async () => {
    if (!tpl) return;
    setSaving(true);
    setError(null);
    try {
      await hostApi.templatesSave(buildTemplateSavePayload(tpl, marketplace, editTabName));
      onSaved();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  }, [tpl, marketplace, editTabName, onSaved, onClose]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (!saving && tpl) void save();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose, save, saving, tpl]);

  return (
    <div className="overlay overlay--win" role="presentation" onClick={onClose}>
      <div
        className="dialog-panel dialog-panel--win tpl-dialog tpl-dialog--wide"
        role="dialog"
        aria-labelledby="tpl-ed-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-header">
          <h2 id="tpl-ed-title">
            {templateId ? t("templates.editor.editTitle") : t("templates.editor.newTitle")}
          </h2>
          <button type="button" className="btn btn-ghost" onClick={onClose}>
            {t("common.close")}
          </button>
        </div>

        <div className="tpl-modal-body combo-clip-ok">
          {busy && !tpl ? <p className="page-sub">{t("common.loading")}</p> : null}
          {error ? <p className="error">{error}</p> : null}
          {tpl ? (
            <TemplateEditorBody
              tpl={tpl}
              saving={saving}
              excelOnly={excelOnly}
              scriptGroups={scriptGroups}
              visibility={visibility}
              sheetOptions={sheetOptions}
              tabs={tabs}
              tabName={editTabName}
              onTabNameChange={setEditTabName}
              onPatch={patch}
              onScriptChange={onScriptChange}
              onSheetOptionsChange={setSheetOptions}
              onMessage={setError}
              onOpenCabinets={() => setCabsOpen(true)}
              onOpenFields={() => setFieldsOpen(true)}
              onOpenExport={() => setExportOpen(true)}
              onOpenSubjectsFilters={() => setSubjectsFiltersOpen(true)}
            />
          ) : null}
        </div>

        <div className="dialog-footer tpl-modal-footer">
          <button type="button" className="btn btn-ghost" disabled={saving} onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={saving || !tpl}
            onClick={() => void save()}
          >
            {saving ? t("common.saving") : t("common.save")}
          </button>
        </div>
      </div>

      {fieldsOpen && tpl ? (
        <Suspense fallback={null}>
          <FieldSelectorDialog
            items={tpl.selected_fields as FieldRow[]}
            onCancel={() => setFieldsOpen(false)}
            onApply={(ordered) => {
              patch({ selected_fields: ordered });
              setFieldsOpen(false);
            }}
          />
        </Suspense>
      ) : null}

      {cabsOpen && tpl ? (
        <Suspense fallback={null}>
          <CabinetSelectorDialog
            options={cabinets.map((c) => ({ id: c.id, cabinet: c.cabinet }))}
            selected={tpl.cabinets ?? []}
            onCancel={() => setCabsOpen(false)}
            onApply={(cabs) => {
              patch({ cabinets: cabs });
              setCabsOpen(false);
            }}
          />
        </Suspense>
      ) : null}

      {exportOpen && tpl ? (
        <Suspense fallback={null}>
          <ExportSettingsDialog
            fields={(tpl.selected_fields ?? [])
              .filter((f) => f.selected)
              .map((f) => ({ name: f.name, title: f.title || f.name }))}
            settings={tpl.sync_settings}
            onCancel={() => setExportOpen(false)}
            onSave={(settings: SyncSettingsDto) => {
              patch({ sync_settings: settings });
              setExportOpen(false);
            }}
          />
        </Suspense>
      ) : null}

      {subjectsFiltersOpen && tpl ? (
        <Suspense fallback={null}>
          <SubjectsFiltersDialog
            filters={tpl.subjects_filters ?? { locale: "ru", name: "", limit: 30, offset: 0 }}
            onClose={() => setSubjectsFiltersOpen(false)}
            onApply={(subjects_filters) => {
              patch({ subjects_filters });
              setSubjectsFiltersOpen(false);
            }}
          />
        </Suspense>
      ) : null}
    </div>
  );
}
