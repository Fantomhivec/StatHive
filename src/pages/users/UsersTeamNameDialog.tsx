import { FormEvent } from "react";
import AppCustomWindow from "../../components/AppCustomWindow";
import { useT } from "../../i18n";
import { shortTeamName } from "../../shared/workspaceTitle";

const FORM_ID = "users-team-name-dialog-form";

type Props = {
  open: boolean;
  busy: boolean;
  workspaceId: string | null;
  workspaceTitle: string;
  teamNameDraft: string;
  onTeamNameDraftChange: (value: string) => void;
  onSubmit: (e: FormEvent) => void;
  onClose: () => void;
};

export default function UsersTeamNameDialog({
  open,
  busy,
  workspaceId,
  workspaceTitle,
  teamNameDraft,
  onTeamNameDraftChange,
  onSubmit,
  onClose,
}: Props) {
  const t = useT();
  if (!open) return null;

  const short = shortTeamName(workspaceTitle);

  return (
    <AppCustomWindow
      title={t("users.teamNameTitle")}
      onClose={onClose}
      width={420}
      bodyClassName="users-dialog-body"
      footer={
        <>
          <button type="button" className="btn btn-ghost" disabled={busy} onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button
            className="btn btn-primary"
            type="submit"
            form={FORM_ID}
            disabled={busy || !workspaceId || !teamNameDraft.trim()}
          >
            {t("users.teamNameSave")}
          </button>
        </>
      }
    >
      <form id={FORM_ID} className="users-dialog-stack field-flush" onSubmit={onSubmit}>
        <p className="muted u-m-0">{t("users.teamNameHint")}</p>
        {short ? (
          <p className="ok-text u-m-0">{t("users.teamNameCurrent", { name: short })}</p>
        ) : null}
        <div className="field">
          <label htmlFor="u-team-name-dlg">{t("users.teamNameLabel")}</label>
          <input
            id="u-team-name-dlg"
            className="input"
            value={teamNameDraft}
            onChange={(e) => onTeamNameDraftChange(e.target.value)}
            disabled={busy || !workspaceId}
            placeholder="StatHive"
            autoComplete="off"
            autoFocus
          />
        </div>
      </form>
    </AppCustomWindow>
  );
}
