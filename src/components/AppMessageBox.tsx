import AppCustomWindow, { type DialogTone } from "./AppCustomWindow";
import { t } from "../i18n";

export type MessageBoxButton = {
  label: string;
  role?: "primary" | "ghost" | "danger";
  value: string;
};

export type AppMessageBoxProps = {
  title?: string;
  message: string;
  buttons?: MessageBoxButton[];
  tone?: DialogTone;
  onClose: (result: string | null) => void;
  zIndex?: number;
  width?: number;
};

const DEFAULT_OK: MessageBoxButton[] = [{ label: t("common.ok"), role: "primary", value: t("login.googleEmailFallback") }];

export function inferDialogTone(title?: string | null, tone?: string | null): DialogTone {
  const t = (tone || "").trim().toLowerCase();
  if (t === "success" || t === "error" || t === "warning" || t === "info" || t === "neutral") {
    return t;
  }
  const titleNorm = (title || "").trim().toLowerCase();
  if (titleNorm.startsWith("успех") || titleNorm.includes("успешно")) return "success";
  if (titleNorm.startsWith("ошибк") || titleNorm.includes("error")) return "error";
  if (
    titleNorm === "внимание" ||
    titleNorm === "предупреждение" ||
    titleNorm === "подтверждение" ||
    titleNorm.includes("удалить")
  ) {
    return "warning";
  }
  return "info";
}

export function confirmButtons(yesLabel = t("common.yes"), noLabel = t("common.no")): MessageBoxButton[] {
  return [
    { label: yesLabel, role: "primary", value: "yes" },
    { label: noLabel, role: "ghost", value: "no" },
  ];
}

export default function AppMessageBox({
  title = t("dialogs.message"),
  message,
  buttons = DEFAULT_OK,
  tone,
  onClose,
  zIndex = 1300,
  width = 450,
}: AppMessageBoxProps) {
  const resolvedTone = inferDialogTone(title, tone);

  return (
    <AppCustomWindow
      title={title}
      width={width}
      zIndex={zIndex}
      tone={resolvedTone}
      closeOnOverlay={false}
      onClose={() => onClose(null)}
      footer={
        <>
          {buttons.map((b) => (
            <button
              key={b.value}
              type="button"
              className={
                b.role === "danger"
                  ? "btn btn-danger"
                  : b.role === "ghost"
                    ? "btn btn-ghost"
                    : "btn"
              }
              onClick={() => onClose(b.value)}
            >
              {b.label}
            </button>
          ))}
        </>
      }
    >
      <p className="app-msg-text">{message}</p>
    </AppCustomWindow>
  );
}
