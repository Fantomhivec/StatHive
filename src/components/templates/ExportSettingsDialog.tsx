import { useMemo, useState } from "react";
import AppComboBox from "../AppComboBox";
import AppCustomWindow from "../AppCustomWindow";
import type { SyncSettingsDto } from "../../bridge/hostApi.types";
import "./templates-dialogs.css";
import { useT } from "../../i18n";

type FieldOpt = { name: string; title: string };

type Props = {
  fields: FieldOpt[];
  settings: SyncSettingsDto | null | undefined;
  onCancel: () => void;
  onSave: (settings: SyncSettingsDto) => void;
};

const EMPTY = "";

export default function ExportSettingsDialog({ fields, settings, onCancel, onSave }: Props) {
  const t = useT();
  const [enable, setEnable] = useState(!!settings?.enable_change_sync);
  const [key1, setKey1] = useState(settings?.key_field_1 ?? EMPTY);
  const [key2, setKey2] = useState(settings?.key_field_2 ?? EMPTY);
  const [noDelete, setNoDelete] = useState(!!settings?.do_not_delete_old_rows);
  const [error, setError] = useState<string | null>(null);

  const options = useMemo(
    () => [
      { value: EMPTY, label: t("templates.exportSync.notSelected") },
      ...fields
        .slice()
        .sort((a, b) => (a.title || a.name).localeCompare(b.title || b.name, "ru"))
        .map((f) => ({ value: f.name, label: f.title || f.name })),
    ],
    [fields],
  );

  const hasFields = fields.length > 0;
  const keysEnabled = enable && hasFields;

  function save() {
    if (enable) {
      if (!hasFields) {
        setError(t("templates.exportSync.errNoFields"));
        return;
      }
      if (!key1 && !key2) {
        setError(t("templates.exportSync.errPickField"));
        return;
      }
      if (key1 && key2 && key1.toLowerCase() === key2.toLowerCase()) {
        setError(t("templates.exportSync.errKeysSame"));
        return;
      }
    }
    onSave({
      enable_change_sync: enable && hasFields,
      key_field_1: key1,
      key_field_2: key2,
      do_not_delete_old_rows: enable && hasFields && noDelete,
    });
  }

  return (
    <AppCustomWindow
      title={t("templates.exportSync.title")}
      width={520}
      zIndex={1250}
      onClose={onCancel}
      bodyClassName="tpl-export-body combo-clip-ok"
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={onCancel}>
            {t("common.cancel")}
          </button>
          <button type="button" className="btn btn-primary" onClick={save}>
            {t("common.save")}
          </button>
        </>
      }
    >
      <h3 className="tpl-trigger-edit-title">{t("templates.exportSync.heading")}</h3>
      {error ? <p className="error">{error}</p> : null}

      <div className="tpl-export-stack">
        <label className="toggle-switch tpl-toggle-row">
          <input type="checkbox" checked={enable} onChange={(e) => setEnable(e.target.checked)} />
          <span className="toggle-track" />
          <span className="tpl-toggle-label">
            {t("templates.exportSync.enable")}
          </span>
        </label>

        <p className="page-sub tpl-export-hint">{t("templates.exportSync.enableHint")}</p>
        <p className="page-sub tpl-export-hint">{t("templates.exportSync.triggersHint")}</p>

        {!hasFields ? (
          <p className="error">
            {t("templates.exportSync.noExportFields")}
          </p>
        ) : null}

        <div className="card card--md card--stack tpl-export-card field-flush">
          <h3 className="tpl-export-card-title">{t("templates.exportSync.compareFieldsTitle")}</h3>
          <p className="page-sub tpl-export-card-hint">
            {t("templates.exportSync.compareFieldsHint")}
          </p>

          <div className="field">
            <label>{t("templates.exportSync.key1")}</label>
            <AppComboBox
              options={options}
              value={key1}
              onChange={setKey1}
              disabled={!keysEnabled}
            />
          </div>

          <div className="field">
            <label>{t("templates.exportSync.key2")}</label>
            <div className="tpl-export-key2-row">
              <div className="tpl-export-key2-combo">
                <AppComboBox
                  options={options}
                  value={key2}
                  onChange={setKey2}
                  disabled={!keysEnabled}
                />
              </div>
              <button
                type="button"
                className="btn btn-ghost btn-sm"
                disabled={!keysEnabled}
                onClick={() => setKey2(EMPTY)}
              >
                {t("common.clear")}
              </button>
            </div>
          </div>

          <label className="toggle-switch tpl-toggle-row tpl-export-delete-toggle">
            <input
              type="checkbox"
              checked={noDelete}
              disabled={!keysEnabled}
              onChange={(e) => setNoDelete(e.target.checked)}
            />
            <span className="toggle-track" />
            <span className="tpl-toggle-label">
              {t("templates.exportSync.keepMissingRows")}
            </span>
          </label>
        </div>
      </div>
    </AppCustomWindow>
  );
}
