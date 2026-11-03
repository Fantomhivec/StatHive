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

import { useMemo, useState } from "react";
import type { EnvKeyRow } from "../../bridge/hostApi";
import { useT } from "../../i18n";
import AppComboBox from "../AppComboBox";
import Modal from "../Modal";

export type Marketplace = "WB" | "OZON";

export function isOzon(mp: string): boolean {
  return mp.trim().toUpperCase() === "OZON";
}

export function envKeyFor(mp: Marketplace, cabinet: string): string {
  return mp === "OZON" ? `OZON_KEY_${cabinet}` : `API_KEY_${cabinet}`;
}

type AddProps = {
  onClose: () => void;
  rows: EnvKeyRow[];
  onAdd: (row: EnvKeyRow) => void;
};

/** Диалог добавления ключа WB / Ozon. */
export function EnvKeysAddDialog({ onClose, rows, onAdd }: AddProps) {
  const t = useT();
  const [addMp, setAddMp] = useState<Marketplace>("WB");
  const [addCabinet, setAddCabinet] = useState("");
  const [addToken, setAddToken] = useState("");
  const [addClientId, setAddClientId] = useState("");
  const [addError, setAddError] = useState<string | null>(null);

  const mpOptions = useMemo(
    () => [
      { value: "WB", label: t("keys.envEditor.marketplaceWb") },
      { value: "OZON", label: t("keys.envEditor.marketplaceOzon") },
    ],
    [t],
  );

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
    onAdd({
      id: envKey,
      cabinet: cab,
      marketplace: addMp,
      expires_at: "",
      env_key: envKey,
      value: token,
      ozon_seller_id: addMp === "OZON" ? clientId : "",
    });
  }

  return (
    <Modal
      title={t("keys.envEditor.addDialogTitle")}
      onClose={onClose}
      width={480}
      footer={
        <>
          <button type="button" className="btn btn-primary" onClick={addKey}>
            {t("common.ok")}
          </button>
          <button type="button" className="btn btn-ghost" onClick={onClose}>
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
  );
}

type EditProps = {
  row: EnvKeyRow;
  onClose: () => void;
  onApply: (patch: { value: string; ozon_seller_id: string }) => void;
  onValidationError: (message: string) => void;
};

/** Диалог обновления токена / Client ID. */
export function EnvKeysEditDialog({
  row,
  onClose,
  onApply,
  onValidationError,
}: EditProps) {
  const t = useT();
  const [editValue, setEditValue] = useState(row.value);
  const [editClientId, setEditClientId] = useState(row.ozon_seller_id || "");
  const ozon = isOzon(row.marketplace);

  function applyEdit() {
    const token = editValue.trim();
    if (!token) {
      onValidationError(t("keys.envEditor.tokenEmpty"));
      return;
    }
    const clientId = editClientId.trim();
    if (ozon && !clientId) {
      onValidationError(t("keys.envEditor.clientIdEmpty"));
      return;
    }
    onApply({ value: token, ozon_seller_id: ozon ? clientId : "" });
  }

  return (
    <Modal
      title={t("keys.envEditor.editDialogTitle", {
        cabinet: row.cabinet,
        marketplace: row.marketplace,
      })}
      onClose={onClose}
      width={480}
      footer={
        <>
          <button type="button" className="btn btn-primary" onClick={applyEdit}>
            {t("common.ok")}
          </button>
          <button type="button" className="btn btn-ghost" onClick={onClose}>
            {t("common.cancel")}
          </button>
        </>
      }
    >
      <div className="env-keys-form combo-clip-ok field-flush">
        <div className="field">
          <label htmlFor="edit-tok">
            {ozon ? t("keys.envEditor.apiKeyOzon") : t("keys.envEditor.newToken")}
          </label>
          <input
            id="edit-tok"
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            autoFocus
          />
        </div>
        {ozon ? (
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
  );
}
