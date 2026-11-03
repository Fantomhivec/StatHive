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

import type { TemplateGroupDto } from "../../../bridge/hostApi";
import AppCustomWindow from "../../AppCustomWindow";
import { useT } from "../../../i18n";
import "../templates-dialogs.css";

type Props = {
  groups: TemplateGroupDto[];
  onCancel: () => void;
  onSelect: (groupId: string) => void;
};

export default function GroupPickDialog({ groups, onCancel, onSelect }: Props) {
  const t = useT();
  return (
    <AppCustomWindow
      title={t("templates.card.addToGroup")}
      onClose={onCancel}
      width={420}
      closeOnOverlay
      footer={
        <button type="button" className="btn btn-ghost" onClick={onCancel}>
          {t("common.cancel")}
        </button>
      }
    >
      {groups.length === 0 ? (
        <p className="page-sub">{t("templates.groupPick.createGroupFirst")}</p>
      ) : (
        <div className="tpl-pick-list">
          {groups.map((g) => (
            <button
              key={g.id}
              type="button"
              className="pick-item"
              onClick={() => onSelect(g.id)}
            >
              {g.name}
              <span className="page-sub"> ({g.template_ids?.length ?? 0})</span>
            </button>
          ))}
        </div>
      )}
    </AppCustomWindow>
  );
}
