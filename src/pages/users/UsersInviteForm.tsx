import { FormEvent } from "react";
import AppComboBox from "../../components/AppComboBox";
import { useT } from "../../i18n";

export type UsersInviteFormProps = {
  busy: boolean;
  saConfigured: boolean;
  presetsEmpty: boolean;
  calcConfigsEmpty: boolean;
  name: string;
  login: string;
  password: string;
  email: string;
  invitePreset: string;
  inviteCalcConfig: string;
  presetOptions: { value: string; label: string }[];
  calcConfigOptions: { value: string; label: string }[];
  onNameChange: (value: string) => void;
  onLoginChange: (value: string) => void;
  onPasswordChange: (value: string) => void;
  onEmailChange: (value: string) => void;
  onInvitePresetChange: (value: string) => void;
  onInviteCalcConfigChange: (value: string) => void;
  onSubmit: (e: FormEvent) => void;
  /** Submit в footer диалога — без кнопки внутри формы. */
  hideSubmit?: boolean;
  formId?: string;
};

export default function UsersInviteForm({
  busy,
  saConfigured,
  presetsEmpty,
  calcConfigsEmpty,
  name,
  login,
  password,
  email,
  invitePreset,
  inviteCalcConfig,
  presetOptions,
  calcConfigOptions,
  onNameChange,
  onLoginChange,
  onPasswordChange,
  onEmailChange,
  onInvitePresetChange,
  onInviteCalcConfigChange,
  onSubmit,
  hideSubmit = false,
  formId = "users-invite-form",
}: UsersInviteFormProps) {
  const t = useT();
  const canSubmit =
    !busy &&
    saConfigured &&
    !presetsEmpty &&
    !!invitePreset &&
    !calcConfigsEmpty &&
    !!inviteCalcConfig;

  return (
    <div className="users-dialog-stack combo-clip-ok field-flush">
      {presetsEmpty ? <p className="muted u-m-0">{t("users.noPresetsInvite")}</p> : null}
      {!saConfigured ? <p className="muted u-m-0">{t("users.inviteNeedsSa")}</p> : null}
      <form id={formId} className="users-invite-form" onSubmit={onSubmit}>
        <div className="field">
          <label htmlFor="u-name">{t("common.name")}</label>
          <input
            id="u-name"
            value={name}
            onChange={(e) => onNameChange(e.target.value)}
            disabled={busy}
          />
        </div>
        <div className="field">
          <label htmlFor="u-login">{t("common.login")}</label>
          <input
            id="u-login"
            value={login}
            onChange={(e) => onLoginChange(e.target.value)}
            disabled={busy}
            required
            autoComplete="off"
          />
        </div>
        <div className="field">
          <label htmlFor="u-pass">{t("common.password")}</label>
          <input
            id="u-pass"
            type="password"
            value={password}
            onChange={(e) => onPasswordChange(e.target.value)}
            disabled={busy}
            required
            autoComplete="off"
          />
        </div>
        <div className="field">
          <label htmlFor="u-email">{t("users.googleEmail")}</label>
          <input
            id="u-email"
            type="text"
            inputMode="email"
            value={email}
            onChange={(e) => onEmailChange(e.target.value)}
            disabled={busy}
            required
            autoComplete="off"
          />
        </div>
        <div className="field">
          <label id="u-preset-label">{t("users.presetRights")}</label>
          <AppComboBox
            value={invitePreset}
            disabled={busy || presetsEmpty}
            placeholder={t("users.noPresetsOption")}
            options={presetOptions}
            onChange={onInvitePresetChange}
          />
        </div>
        <div className="field">
          <label id="u-calc-cfg-label">{t("users.calcConfig")}</label>
          <AppComboBox
            value={inviteCalcConfig}
            disabled={busy || calcConfigsEmpty}
            placeholder={t("users.noCalcConfigsOption")}
            options={calcConfigOptions}
            onChange={onInviteCalcConfigChange}
          />
        </div>
        {!hideSubmit ? (
          <div className="form-actions users-form-actions">
            <button className="btn btn-primary" type="submit" disabled={!canSubmit}>
              {t("users.inviteBtn")}
            </button>
          </div>
        ) : null}
      </form>
    </div>
  );
}

export function inviteFormCanSubmit(p: {
  busy: boolean;
  saConfigured: boolean;
  presetsEmpty: boolean;
  calcConfigsEmpty: boolean;
  invitePreset: string;
  inviteCalcConfig: string;
}): boolean {
  return (
    !p.busy &&
    p.saConfigured &&
    !p.presetsEmpty &&
    !!p.invitePreset &&
    !p.calcConfigsEmpty &&
    !!p.inviteCalcConfig
  );
}
