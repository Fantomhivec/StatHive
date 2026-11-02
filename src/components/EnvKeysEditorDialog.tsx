import { useCallback, useEffect, useMemo, useState } from "react";
import { EnvKeyRow, hostApi } from "../bridge/hostApi";
import { hostDialogs } from "../bridge/hostApiChrome";
import AppComboBox from "./AppComboBox";
import Modal from "./Modal";
import { useAppDialogs } from "./useAppDialogs";
import "./EnvKeysEditorDialog.css";
import { useT } from "../i18n";

type Props = {
  onClose: () => void;
  onSaved?: () => void;
};

type Marketplace = "WB" | "OZON";

function isOzon(mp: string): boolean {
  return mp.trim().toUpperCase() === "OZON";
}

function envKeyFor(mp: Marketplace, cabinet: string): string {
  return mp === "OZON" ? `OZON_KEY_${cabinet}` : `API_KEY_${cabinet}`;
}

/** Редактор `.env` — Wildberries и Ozon. */
export default function EnvKeysEditorDialog({ onClose, onSaved }: Props) {
  const t = useT();
  const [path, setPath] = useState("");
  const [rows, setRows] = useState<EnvKeyRow[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [addMp, setAddMp] = useState<Marketplace>("WB");
  const [addCabinet, setAddCabinet] = useState("");
  const [addToken, setAddToken] = useState("");
  const [addClientId, setAddClientId] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [editRow, setEditRow] = useState<EnvKeyRow | null>(null);
  const [editValue, setEditValue] = useState("");
  const [editClientId, setEditClientId] = useState("");
  const [info, setInfo] = useState<string | null>(null);
  const { confirm, dialogs } = useAppDialogs();

  const mpOptions = useMemo(
    () => [
      { value: "WB", label: t("keys.envEditor.marketplaceWb") },
      { value: "OZON", label: t("keys.envEditor.marketplaceOzon") },
    ],
    [t],
  );

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

  function resetAddForm() {
    setAddMp("WB");
    setAddCabinet("");
    setAddToken("");
    setAddClientId("");
    setAddError(null);
  }

  function addKey() {
    const cab = addCabinet.trim();
    const token = addToken.trim();
    const clientId = addClientId.trim();
    if (!cab) {
      setAddError(t("keys.envEditor.cabinetEmpty"));
      return;
    }
    if (!token) {
      setAddError(t("keys.envEditor.tokenEmpty"));
      return;
    }
    if (addMp === "OZON" && !clientId) {
      setAddError(t("keys.envEditor.clientIdEmpty"));
      return;
    }
    const envKey = envKeyFor(addMp, cab);
    if (rows.some((r) => r.env_key.toLowerCase() === envKey.toLowerCase())) {
      setAddError(t("keys.envEditor.duplicateKey"));
      return;
    }
    setRows((prev) => [
      ...prev,
      {
        id: envKey,
        cabinet: cab,
        marketplace: addMp,
        expires_at: "",
        env_key: envKey,
        value: token,
        ozon_seller_id: addMp === "OZON" ? clientId : "",
      },
    ]);
    setSelectedId(envKey);
    setAddOpen(false);
    resetAddForm();
  }

  function applyEdit() {
    if (!editRow) return;
    const token = editValue.trim();
    if (!token) {
      setError(t("keys.envEditor.tokenEmpty"));
      return;
    }
    const ozon = isOzon(editRow.marketplace);
    const clientId = editClientId.trim();
    if (ozon && !clientId) {
      setError(t("keys.envEditor.clientIdEmpty"));
      return;
    }
    setRows((prev) =>
      prev.map((r) =>
        r.id === editRow.id
          ? {
              ...r,
              value: token,
              ozon_seller_id: ozon ? clientId : "",
              expires_at: "",
            }
          : r,
      ),
    );
    setEditRow(null);
    setError(null);
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
              onClick={() => {
                resetAddForm();
                setAddOpen(true);
              }}
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
          <div className="mono-panel env-keys-scroll">
            <table className="data-table env-keys-table">
              <thead>
                <tr>
                  <th>{t("common.cabinet")}</th>
                  <th>{t("common.marketplace")}</th>
                  <th>{t("keys.envEditor.expiry")}</th>
                  <th>{t("common.actions")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.length === 0 ? (
                  <tr>
                    <td colSpan={4} className="env-keys-empty">
                      {t("keys.envEditor.emptyTable")}
                    </td>
                  </tr>
                ) : (
                  rows.map((r) => (
                    <tr
                      key={r.id}
                      className={selectedId === r.id ? "selected" : undefined}
                      onClick={() => setSelectedId(r.id)}
                    >
                      <td>{r.cabinet}</td>
                      <td>{r.marketplace}</td>
                      <td>{r.expires_at || "—"}</td>
                      <td>
                        <div className="form-actions env-keys-actions" onClick={(e) => e.stopPropagation()}>
                          <button
                            type="button"
                            className="btn btn-ghost"
                            onClick={() => {
                              setEditRow(r);
                              setEditValue(r.value);
                              setEditClientId(r.ozon_seller_id || "");
                              setError(null);
                            }}
                          >
                            {t("common.refresh")}
                          </button>
                          <button
                            type="button"
                            className="btn btn-ghost"
                            onClick={() => {
                              void (async () => {
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
                              })();
                            }}
                          >
                            {t("common.delete")}
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      </Modal>

      {addOpen && (
        <Modal
          title={t("keys.envEditor.addDialogTitle")}
          onClose={() => {
            setAddOpen(false);
            resetAddForm();
          }}
          width={480}
          footer={
            <>
              <button type="button" className="btn btn-primary" onClick={addKey}>
                {t("common.ok")}
              </button>
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => {
                  setAddOpen(false);
                  resetAddForm();
                }}
              >
                {t("common.cancel")}
              </button>
            </>
          }
        >
          <div className="env-keys-form combo-clip-ok field-flush">
            <p className="env-keys-hint">
              {addMp === "OZON"
                ? t("keys.envEditor.platformHintOzon")
                : t("keys.envEditor.platformHintWb")}
            </p>
            <div className="field">
              <label id="add-mp-label">{t("keys.envEditor.marketplace")}</label>
              <AppComboBox
                value={addMp}
                options={mpOptions}
                onChange={(v) => setAddMp(v === "OZON" ? "OZON" : "WB")}
              />
            </div>
            <div className="field">
              <label htmlFor="add-cab">{t("keys.envEditor.cabinetName")}</label>
              <input
                id="add-cab"
                value={addCabinet}
                onChange={(e) => setAddCabinet(e.target.value)}
                autoFocus
              />
            </div>
            <div className="field">
              <label htmlFor="add-tok">
                {addMp === "OZON"
                  ? t("keys.envEditor.apiKeyOzon")
                  : t("keys.envEditor.apiKey")}
              </label>
              <input
                id="add-tok"
                value={addToken}
                onChange={(e) => setAddToken(e.target.value)}
              />
            </div>
            {addMp === "OZON" ? (
              <div className="field">
                <label htmlFor="add-cid">{t("keys.envEditor.clientId")}</label>
                <input
                  id="add-cid"
                  value={addClientId}
                  onChange={(e) => setAddClientId(e.target.value)}
                />
              </div>
            ) : null}
            {addError ? <p className="error">{addError}</p> : null}
          </div>
        </Modal>
      )}

      {editRow && (
        <Modal
          title={t("keys.envEditor.editDialogTitle", {
            cabinet: editRow.cabinet,
            marketplace: editRow.marketplace,
          })}
          onClose={() => setEditRow(null)}
          width={480}
          footer={
            <>
              <button type="button" className="btn btn-primary" onClick={applyEdit}>
                {t("common.ok")}
              </button>
              <button type="button" className="btn btn-ghost" onClick={() => setEditRow(null)}>
                {t("common.cancel")}
              </button>
            </>
          }
        >
          <div className="env-keys-form combo-clip-ok field-flush">
            <div className="field">
              <label htmlFor="edit-tok">
                {isOzon(editRow.marketplace)
                  ? t("keys.envEditor.apiKeyOzon")
                  : t("keys.envEditor.newToken")}
              </label>
              <input
                id="edit-tok"
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                autoFocus
              />
            </div>
            {isOzon(editRow.marketplace) ? (
              <div className="field">
                <label htmlFor="edit-cid">{t("keys.envEditor.clientId")}</label>
                <input
                  id="edit-cid"
                  value={editClientId}
                  onChange={(e) => setEditClientId(e.target.value)}
                />
              </div>
            ) : null}
          </div>
        </Modal>
      )}

      {dialogs}
    </>
  );
}
