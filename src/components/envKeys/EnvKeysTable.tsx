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

import type { EnvKeyRow } from "../../bridge/hostApi";
import { useT } from "../../i18n";

type Props = {
  rows: EnvKeyRow[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onEdit: (row: EnvKeyRow) => void;
  onDelete: (row: EnvKeyRow) => void;
};

/** Таблица ключей `.env` (кабинет / площадка / срок / действия). */
export default function EnvKeysTable({
  rows,
  selectedId,
  onSelect,
  onEdit,
  onDelete,
}: Props) {
  const t = useT();

  return (
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
                onClick={() => onSelect(r.id)}
              >
                <td>{r.cabinet}</td>
                <td>{r.marketplace}</td>
                <td>{r.expires_at || "—"}</td>
                <td>
                  <div
                    className="form-actions env-keys-actions"
                    onClick={(e) => e.stopPropagation()}
                  >
                    <button
                      type="button"
                      className="btn btn-ghost"
                      onClick={() => onEdit(r)}
                    >
                      {t("common.refresh")}
                    </button>
                    <button
                      type="button"
                      className="btn btn-ghost"
                      onClick={() => onDelete(r)}
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
  );
}
