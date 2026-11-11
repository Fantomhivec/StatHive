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
      <section className="card card--md card--stack users-section-card users-presets-panel">
        <div className="users-presets-toolbar">
          <div className="users-presets-toolbar-meta">
            <h2 className="users-section-title">{t("users.presetsTitle")}</h2>
            <p className="users-section-desc">{t("users.presetsHintShort")}</p>
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
              <li key={p.id} className="wb-card wb-card--control users-presets-row users-presets-row--bar">
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
