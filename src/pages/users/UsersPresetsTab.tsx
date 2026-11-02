import { useState } from "react";
import type { AccessPresetDto, ScriptCatalogItem } from "../../bridge/hostApi";
import { useT } from "../../i18n";
import { emptyPreset } from "./usersPageCache";
import UsersPresetDialog from "./UsersPresetDialog";

export type UsersPresetsTabProps = {
  busy: boolean;
  presets: AccessPresetDto[];
  catalog: ScriptCatalogItem[];
  onSavePreset: (preset: AccessPresetDto, onSuccess?: () => void) => void;
  onDeletePreset: (presetId: string) => void;
};

export default function UsersPresetsTab({
  busy,
  presets,
  catalog,
  onSavePreset,
  onDeletePreset,
}: UsersPresetsTabProps) {
  const t = useT();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [draft, setDraft] = useState<AccessPresetDto>(emptyPreset);

  function openCreate() {
    setDraft(emptyPreset());
    setDialogOpen(true);
  }

  function openEdit(p: AccessPresetDto) {
    setDraft({
      ...p,
      calcAccess: [...(p.calcAccess ?? [])],
      appAccess: [...(p.appAccess ?? [])],
      scripts: { ...p.scripts, keys: [...p.scripts.keys] },
    });
    setDialogOpen(true);
  }

  return (
    <>
      <section className="card card--md card--stack users-presets-panel">
        <div className="users-presets-toolbar">
          <div className="users-presets-toolbar-meta">
            <h2 className="panel-title u-m-0">{t("users.presetsTitle")}</h2>
            <p className="muted u-m-0">{t("users.presetsHintShort")}</p>
          </div>
          <button
            type="button"
            className="btn btn-primary"
            disabled={busy}
            onClick={openCreate}
          >
            {t("users.presetAdd")}
          </button>
        </div>

        <ul className="users-presets-list">
          {presets.length === 0 ? (
            <li className="muted users-presets-empty">{t("users.noPresetsList")}</li>
          ) : (
            presets.map((p) => (
              <li key={p.id} className="users-presets-row users-presets-row--bar">
                <div className="users-presets-row-meta">
                  <span className="users-presets-title">{p.title || p.id}</span>
                  <span className="muted users-presets-id">
                    {t("users.presetIdLabel", { id: p.id })}
                  </span>
                </div>
                <div className="users-presets-row-actions">
                  <button
                    type="button"
                    className="btn btn-outline btn-sm"
                    disabled={busy}
                    onClick={() => openEdit(p)}
                  >
                    {t("users.presetEdit")}
                  </button>
                  <button
                    type="button"
                    className="btn btn-danger btn-sm"
                    disabled={busy}
                    onClick={() => void onDeletePreset(p.id)}
                  >
                    {t("common.delete")}
                  </button>
                </div>
              </li>
            ))
          )}
        </ul>
      </section>

      <UsersPresetDialog
        open={dialogOpen}
        busy={busy}
        preset={draft}
        catalog={catalog}
        onChange={setDraft}
        onClose={() => setDialogOpen(false)}
        onSave={() => onSavePreset(draft, () => setDialogOpen(false))}
      />
    </>
  );
}
