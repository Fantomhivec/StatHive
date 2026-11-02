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

import AppCustomWindow from "../../components/AppCustomWindow";
import { useT } from "../../i18n";

type Props = {
  open: boolean;
  busy: boolean;
  importJson: string;
  onImportJsonChange: (value: string) => void;
  onPaste: () => void;
  onImportFile: () => void;
  onClose: () => void;
};

export function SettingsImportDialog({
  open,
  busy,
  importJson,
  onImportJsonChange,
  onPaste,
  onImportFile,
  onClose,
}: Props) {
  const t = useT();
  if (!open) return null;

  return (
    <AppCustomWindow
      title={t("calculator.settings.importDialogTitle")}
      onClose={onClose}
      width={720}
      bodyClassName="calc-dialog-body"
    >
      <p className="calc-import-hint muted">{t("calculator.settings.importHint")}</p>
      <textarea
        className="input input-mono mono-panel--scroll-thin calc-import-textarea"
        value={importJson}
        onChange={(e) => onImportJsonChange(e.target.value)}
        rows={16}
        spellCheck={false}
        placeholder="{ … }"
        aria-label={t("calculator.settings.importJsonAria")}
      />
      <div className="dialog-footer dialog-footer--pad-lg dialog-footer--surface calc-dialog-footer">
        <button
          type="button"
          className="btn btn-primary"
          disabled={busy || !importJson.trim()}
          onClick={onPaste}
        >
          {t("calculator.settings.paste")}
        </button>
        <button
          type="button"
          className="btn btn-outline"
          disabled={busy}
          onClick={onImportFile}
        >
          {t("common.browseFile")}
        </button>
        <button type="button" className="btn btn-outline" onClick={onClose}>
          {t("common.cancel")}
        </button>
      </div>
    </AppCustomWindow>
  );
}
