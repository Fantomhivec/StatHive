import type { TemplateGroupDto } from "../../bridge/hostApi";
import AppCustomWindow from "../AppCustomWindow";
import "./templates-dialogs.css";
import { useT } from "../../i18n";

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
