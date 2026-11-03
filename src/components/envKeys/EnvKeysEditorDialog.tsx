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

import { useCallback, useEffect, useState } from "react";
import { EnvKeyRow, hostApi } from "../../bridge/hostApi";
import { hostDialogs } from "../../bridge/hostApiChrome";
import { useT } from "../../i18n";
import Modal from "../Modal";
import { useAppDialogs } from "../useAppDialogs";
import { EnvKeysAddDialog, EnvKeysEditDialog, isOzon } from "./EnvKeysFormDialogs";
import EnvKeysTable from "./EnvKeysTable";
import "../EnvKeysEditorDialog.css";

type Props = {
  onClose: () => void;
  onSaved?: () => void;
};

/** Редактор `.env` — Wildberries и Ozon. */
export default function EnvKeysEditorDialog({ onClose, onSaved }: Props) {
  const t = useT();
  const [path, setPath] = useState("");
  const [rows, setRows] = useState<EnvKeyRow[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [editRow, setEditRow] = useState<EnvKeyRow | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const { confirm, dialogs } = useAppDialogs();

  const selected = rows.find((r) => r.id === selectedId);
  const selectedIsWb = selected ? !isOzon(selected.marketplace) : false;

  useEffect(() => {
    void hostApi
      .envCreateDefault()
      .then((res) => {
        setPath(res.path || "");
        setRows(res.rows ?? []);
      })
      .catch((e: Error) => setError(e.message));
  }, []);

  const save = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      await hostApi.envSaveRows(rows);
      onSaved?.();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, [rows, onSaved, onClose]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (!busy && !addOpen && !editRow) void save();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [busy, addOpen, editRow, save]);

  async function exportEnv() {
    setBusy(true);
    setError(null);
    try {
      const dest = await hostDialogs.saveEnvFile();
      if (!dest) return;
      await hostApi.envExportTo(dest);
      setInfo(t("keys.envEditor.exportOk"));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function importEnv() {
    if (!(await confirm(t("keys.envEditor.importConfirm"), t("dialogs.confirm")))) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const src = await hostDialogs.pickEnvFile(t("bridge.pickEnvImport"));
      if (!src) return;
      const snap = await hostApi.envImportFrom(src);
      setPath(snap.path || path);
      setRows(snap.rows ?? []);
      setInfo(t("keys.envEditor.importOk", { count: snap.rows?.length ?? 0 }));
      onSaved?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete(r: EnvKeyRow) {
    if (
      !(await confirm(
        t("keys.envEditor.deleteConfirm", { cabinet: r.cabinet }),
        t("dialogs.confirm"),
      ))
    ) {
      return;
    }
    setRows((prev) => prev.filter((x) => x.id !== r.id));
    if (selectedId === r.id) setSelectedId(null);
  }

  return (
    <>
      <Modal
        title={t("keys.envEditor.title")}
        onClose={onClose}
        width={900}
        footer={
          <>
            <button
              type="button"
              className="btn btn-ghost"
              disabled={busy || !selectedId || !selectedIsWb}
              title={
                selected && !selectedIsWb
                  ? t("keys.envEditor.apiInfoWbOnly")
                  : undefined
              }
              onClick={() => {
                const sel = rows.find((r) => r.id === selectedId);
                if (!sel) {
                  setInfo(t("keys.envEditor.selectKeyFirst"));
                  return;
                }
                if (isOzon(sel.marketplace)) {
                  setInfo(t("keys.envEditor.apiInfoWbOnly"));
                  return;
                }
                void hostApi
                  .envWbTokenInfo(sel.cabinet, sel.marketplace, sel.value)
                  .then((res) => setInfo(res.message))
                  .catch((e) => setInfo(e instanceof Error ? e.message : String(e)));
              }}
            >
              {t("keys.envEditor.apiInfoBtn")}
            </button>
            <button
              type="button"
              className="btn btn-ghost"
              disabled={busy}
              onClick={() => setAddOpen(true)}
            >
              {t("keys.envEditor.addKey")}
            </button>
            <button
              type="button"
              className="btn btn-outline"
              disabled={busy}
              onClick={() => void exportEnv()}
            >
              {t("keys.envEditor.exportBtn")}
            </button>
            <button
              type="button"
              className="btn btn-outline"
              disabled={busy}
              onClick={() => void importEnv()}
            >
              {t("keys.envEditor.importBtn")}
            </button>
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy}
              onClick={() => void save()}
            >
              {t("common.save")}
            </button>
            <button type="button" className="btn btn-ghost" disabled={busy} onClick={onClose}>
              {t("common.cancel")}
            </button>
          </>
        }
      >
        <div className="env-keys-root">
          <p className="env-keys-hint">
            {t("keys.envEditor.hint")}
            {path ? (
              <span className="env-keys-hint-path">
                {t("keys.envEditor.pathPrefix")} {path}
              </span>
            ) : null}
          </p>
          {error ? <p className="error">{error}</p> : null}
          {info ? (
            <pre className="mono-panel mono-panel--on-bg env-keys-info">{info}</pre>
          ) : null}
          <EnvKeysTable
            rows={rows}
            selectedId={selectedId}
            onSelect={setSelectedId}
            onEdit={(r) => {
              setEditRow(r);
              setError(null);
            }}
            onDelete={(r) => void handleDelete(r)}
          />
        </div>
      </Modal>

      {addOpen ? (
        <EnvKeysAddDialog
          onClose={() => setAddOpen(false)}
          rows={rows}
          onAdd={(row) => {
            setRows((prev) => [...prev, row]);
            setSelectedId(row.id);
            setAddOpen(false);
          }}
        />
      ) : null}

      {editRow ? (
        <EnvKeysEditDialog
          row={editRow}
          onClose={() => setEditRow(null)}
          onValidationError={setError}
          onApply={({ value, ozon_seller_id }) => {
            setRows((prev) =>
              prev.map((r) =>
                r.id === editRow.id
                  ? { ...r, value, ozon_seller_id, expires_at: "" }
                  : r,
              ),
            );
            setEditRow(null);
            setError(null);
          }}
        />
      ) : null}

      {dialogs}
    </>
  );
}
