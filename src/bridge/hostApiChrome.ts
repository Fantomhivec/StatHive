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

/** Window / file dialogs / openUrl — split from hostApi for thinner page chunks. */
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { openUrl as pluginOpenUrl } from "@tauri-apps/plugin-opener";
import { t } from "../i18n";

function pad2(n: number) {
  return String(n).padStart(2, "0");
}

/** Имя файла выгрузки по умолчанию: calculator-yyyyMMdd-HHmm.xlsx */
export function defaultExcelFileName(now = new Date()): string {
  return `calculator-${now.getFullYear()}${pad2(now.getMonth() + 1)}${pad2(now.getDate())}-${pad2(now.getHours())}${pad2(now.getMinutes())}.xlsx`;
}

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Оконные операции — только через bridge (не `@tauri-apps/api/window` в UI). */
export const hostWindow = {
  isMaximized: async (): Promise<boolean> => {
    if (!inTauri()) return false;
    return getCurrentWindow().isMaximized();
  },
  minimize: async (): Promise<void> => {
    if (!inTauri()) return;
    await getCurrentWindow().minimize();
  },
  toggleMaximize: async (): Promise<void> => {
    if (!inTauri()) return;
    await getCurrentWindow().toggleMaximize();
  },
  close: async (): Promise<void> => {
    if (!inTauri()) return;
    await getCurrentWindow().close();
  },
  startDragging: async (): Promise<void> => {
    if (!inTauri()) return;
    await getCurrentWindow().startDragging();
  },
  onResized: async (handler: () => void): Promise<() => void> => {
    if (!inTauri()) return () => undefined;
    return getCurrentWindow().onResized(handler);
  },
};

/** Hosts allowed for external open (XSS → opener hardening). */
const OPEN_URL_HOST_SUFFIXES = [
  "discord.gg",
  "discord.com",
  "docs.google.com",
  "drive.google.com",
  "accounts.google.com",
  "github.com",
  "ozon.ru",
  "wildberries.ru",
];

function isAllowedOpenUrl(raw: string): boolean {
  try {
    const u = new URL(raw.trim());
    if (u.protocol !== "https:") return false;
    const host = u.hostname.toLowerCase();
    return OPEN_URL_HOST_SUFFIXES.some((s) => host === s || host.endsWith(`.${s}`));
  } catch {
    return false;
  }
}

/** Открыть URL во внешнем браузере (только allowlist https). */
export async function openUrl(url: string): Promise<void> {
  if (!isAllowedOpenUrl(url)) {
    throw new Error("URL не из разрешённого списка");
  }
  await pluginOpenUrl(url);
}

/** Диалоги проводника (.xlsx / .json / .env). */
export const hostDialogs = {
  openUrl,
  pickJsonFile: async (title = t("bridge.pickJsonImport")): Promise<string | null> => {
    const selected = await openDialog({
      multiple: false,
      title,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (typeof selected !== "string" || !selected.trim()) return null;
    return selected.trim();
  },
  /** Файл `*.env` (keys.env, cab.env, …); диалог стартует в папке exe. */
  pickEnvFile: async (title = t("bridge.pickEnvFile")): Promise<string | null> => {
    let defaultPath: string | undefined;
    try {
      defaultPath = await invoke<string>("env_exe_dir");
    } catch {
      defaultPath = undefined;
    }
    const selected = await openDialog({
      multiple: false,
      title,
      defaultPath: defaultPath?.trim() || undefined,
      filters: [{ name: ".env", extensions: ["env"] }],
    });
    if (typeof selected !== "string" || !selected.trim()) return null;
    return selected.trim();
  },
  saveEnvFile: async (
    title = t("bridge.saveEnvFile"),
    defaultPath = ".env",
  ): Promise<string | null> => {
    const selected = await saveDialog({
      title,
      defaultPath,
      filters: [{ name: ".env", extensions: ["env"] }],
    });
    if (typeof selected !== "string" || !selected.trim()) return null;
    return selected.trim();
  },
  saveJsonFile: async (
    title = t("bridge.saveJsonConfig"),
    defaultPath?: string,
  ): Promise<string | null> => {
    const selected = await saveDialog({
      title,
      defaultPath: defaultPath?.trim() || undefined,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (typeof selected !== "string" || !selected.trim()) return null;
    return selected.trim();
  },
  saveExcelFile: async (
    title = t("bridge.saveExcelFile"),
    defaultPath?: string,
  ): Promise<string | null> => {
    const selected = await saveDialog({
      title,
      defaultPath: defaultPath?.trim() || defaultExcelFileName(),
      filters: [{ name: "Excel", extensions: ["xlsx"] }],
    });
    if (typeof selected !== "string" || !selected.trim()) return null;
    return selected.trim();
  },
};
