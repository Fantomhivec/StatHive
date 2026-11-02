import { type ReactNode, useEffect } from "react";
import "./AppCustomWindow.css";

export type DialogTone = "neutral" | "info" | "success" | "warning" | "error";

type Props = {
  title: string;
  children: ReactNode;
  onClose: () => void;
  width?: number | string;
  zIndex?: number;
  tone?: DialogTone;
  closeOnOverlay?: boolean;
  showClose?: boolean;
  bodyClassName?: string;
  footer?: ReactNode;
  /** Почти на весь экран (настройка вкладок и т.п.). */
  maximized?: boolean;
};

/** Тематическое окно — без native alert/prompt. Esc закрывает (как Modal). */
export default function AppCustomWindow({
  title,
  children,
  onClose,
  width = 440,
  zIndex = 1200,
  tone = "neutral",
  closeOnOverlay = false,
  showClose = true,
  bodyClassName,
  footer,
  maximized = false,
}: Props) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  return (
    <div
      className={
        "overlay overlay--win" + (maximized ? " app-win-backdrop--max" : "")
      }
      style={{ zIndex }}
      role="presentation"
      onClick={closeOnOverlay ? onClose : undefined}
    >
      <div
        className={
          `dialog-panel dialog-panel--win app-win--${tone}` +
          (maximized ? " app-win--max" : "")
        }
        style={
          maximized
            ? undefined
            : { width: typeof width === "number" ? `min(${width}px, 100%)` : width }
        }
        role="dialog"
        aria-modal="true"
        aria-labelledby="app-win-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="app-win-header">
          <h2 id="app-win-title">{title}</h2>
          {showClose ? (
            <button type="button" className="btn btn-ghost btn-modal-close" onClick={onClose}>
              ×
            </button>
          ) : null}
        </div>
        <div className={"app-win-body" + (bodyClassName ? ` ${bodyClassName}` : "")}>{children}</div>
        {footer ? <div className="dialog-footer app-win-footer">{footer}</div> : null}
      </div>
    </div>
  );
}
