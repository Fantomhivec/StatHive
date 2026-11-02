import { FormEvent } from "react";
import AppCustomWindow from "../../components/AppCustomWindow";
import { useT } from "../../i18n";
import UsersInviteForm, {
  inviteFormCanSubmit,
  type UsersInviteFormProps,
} from "./UsersInviteForm";

const FORM_ID = "users-invite-dialog-form";

type Props = Omit<UsersInviteFormProps, "hideSubmit" | "formId" | "onSubmit"> & {
  open: boolean;
  onClose: () => void;
  onSubmit: (e: FormEvent) => void;
};

export default function UsersInviteDialog({
  open,
  onClose,
  onSubmit,
  busy,
  saConfigured,
  presetsEmpty,
  calcConfigsEmpty,
  invitePreset,
  inviteCalcConfig,
  ...formProps
}: Props) {
  const t = useT();
  if (!open) return null;

  const canSubmit = inviteFormCanSubmit({
    busy,
    saConfigured,
    presetsEmpty,
    calcConfigsEmpty,
    invitePreset,
    inviteCalcConfig,
  });

  return (
    <AppCustomWindow
      title={t("users.inviteTitle")}
      onClose={onClose}
      width={480}
      bodyClassName="users-dialog-body combo-clip-ok"
      footer={
        <>
          <button type="button" className="btn btn-ghost" disabled={busy} onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button
            className="btn btn-primary"
            type="submit"
            form={FORM_ID}
            disabled={!canSubmit}
          >
            {t("users.inviteBtn")}
          </button>
        </>
      }
    >
      <UsersInviteForm
        {...formProps}
        busy={busy}
        saConfigured={saConfigured}
        presetsEmpty={presetsEmpty}
        calcConfigsEmpty={calcConfigsEmpty}
        invitePreset={invitePreset}
        inviteCalcConfig={inviteCalcConfig}
        hideSubmit
        formId={FORM_ID}
        onSubmit={onSubmit}
      />
    </AppCustomWindow>
  );
}
