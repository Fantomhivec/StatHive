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
