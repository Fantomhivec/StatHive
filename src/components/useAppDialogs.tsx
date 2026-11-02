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

import { useCallback, useState } from "react";
import AppMessageBox, { confirmButtons, type MessageBoxButton } from "./AppMessageBox";
import AppPromptDialog from "./AppPromptDialog";
import type { DialogTone } from "./AppCustomWindow";
import { t } from "../i18n";

type MsgState = {
  kind: "message";
  title: string;
  message: string;
  tone?: DialogTone;
  buttons: MessageBoxButton[];
  resolve: (value: string | null) => void;
};

type PromptState = {
  kind: "prompt";
  title: string;
  label: string;
  initial: string;
  browseLabel?: string;
  onBrowse?: () => Promise<string | null>;
  resolve: (value: string | null) => void;
};

type DialogState = MsgState | PromptState | null;

export type PromptOptions = {
  browseLabel?: string;
  onBrowse?: () => Promise<string | null>;
};

/**
 * Тематические alert / confirm / prompt вместо window.* (без «localhost:…»).
 * Рендерьте `{dialogs}` рядом с разметкой страницы.
 */
export function useAppDialogs() {
  const [dlg, setDlg] = useState<DialogState>(null);

  const alert = useCallback((message: string, title = t("dialogs.message"), tone?: DialogTone) => {
    return new Promise<void>((resolve) => {
      setDlg({
        kind: "message",
        title,
        message,
        tone,
        buttons: [{ label: t("common.ok"), role: "primary", value: t("login.googleEmailFallback") }],
        resolve: () => {
          setDlg(null);
          resolve();
        },
      });
    });
  }, []);

  const confirm = useCallback((message: string, title = t("dialogs.confirm")) => {
    return new Promise<boolean>((resolve) => {
      setDlg({
        kind: "message",
        title,
        message,
        tone: "warning",
        buttons: confirmButtons(t("common.yes"), t("common.no")),
        resolve: (v) => {
          setDlg(null);
          resolve(v === "yes");
        },
      });
    });
  }, []);

  const prompt = useCallback(
    (label: string, title = t("dialogs.prompt"), initial = "", options?: PromptOptions) => {
      return new Promise<string | null>((resolve) => {
        setDlg({
          kind: "prompt",
          title,
          label,
          initial,
          browseLabel: options?.browseLabel,
          onBrowse: options?.onBrowse,
          resolve: (v) => {
            setDlg(null);
            resolve(v);
          },
        });
      });
    },
    [],
  );

  const dialogs =
    dlg?.kind === "message" ? (
      <AppMessageBox
        title={dlg.title}
        message={dlg.message}
        tone={dlg.tone}
        buttons={dlg.buttons}
        onClose={(v) => dlg.resolve(v)}
      />
    ) : dlg?.kind === "prompt" ? (
      <AppPromptDialog
        title={dlg.title}
        label={dlg.label}
        initial={dlg.initial}
        browseLabel={dlg.browseLabel}
        onBrowse={dlg.onBrowse}
        onClose={(v) => dlg.resolve(v)}
      />
    ) : null;

  return { alert, confirm, prompt, dialogs };
}
