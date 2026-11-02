import { lazy, Suspense } from "react";
import type { AccessPresetDto, ScriptCatalogItem } from "../../bridge/hostApi";
import AppCustomWindow from "../../components/AppCustomWindow";
import { useT } from "../../i18n";

const UsersAccessEditor = lazy(() => import("../UsersAccessEditor"));

type Props = {
  open: boolean;
  busy: boolean;
  /** Пустой id = создание. */
  preset: AccessPresetDto;
  catalog: ScriptCatalogItem[];
  onChange: (preset: AccessPresetDto) => void;
  onSave: () => void;
  onClose: () => void;
};

export default function UsersPresetDialog({
  open,
  busy,
  preset,
  catalog,
  onChange,
  onSave,
  onClose,
}: Props) {
  const t = useT();
  if (!open) return null;

  const isEdit = !!preset.id.trim();
  const canSave = !busy && !!preset.title.trim();

  return (
    <AppCustomWindow
      title={isEdit ? t("users.editPresetTitle", { title: preset.title || preset.id }) : t("users.newPreset")}
      onClose={onClose}
      width={640}
      bodyClassName="users-preset-dialog-body"
      footer={
        <>
          <button type="button" className="btn btn-ghost" disabled={busy} onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={!canSave}
            onClick={() => void onSave()}
          >
            {isEdit ? t("common.saveChanges") : t("users.createPreset")}
          </button>
        </>
      }
    >
      <div className="users-dialog-stack field-flush">
        {isEdit ? (
          <p className="muted u-m-0 users-preset-id-line">
            {t("users.presetIdLabel", { id: preset.id })}
          </p>
        ) : null}
        <Suspense fallback={<p className="muted">{t("common.loading")}</p>}>
          <UsersAccessEditor
            {...preset}
            catalog={catalog}
            autoId
            busy={busy}
            onChange={onChange}
          />
        </Suspense>
      </div>
    </AppCustomWindow>
  );
}
