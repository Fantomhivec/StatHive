import { type ReactNode, useEffect } from "react";
import "./Modal.css";
import { useT } from "../i18n";

type Props = {
  title: string;
  onClose: () => void;
  children: ReactNode;
  width?: number | string;
  footer?: ReactNode;
};

export default function Modal({ title, onClose, children, width = 720, footer }: Props) {
  const t = useT();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  return (
    <div
      className="overlay overlay--modal"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="dialog-panel dialog-panel--modal"
        style={{ width: typeof width === "number" ? `${width}px` : width }}
        role="dialog"
        aria-modal="true"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <header className="app-modal-titlebar">
          <div className="app-modal-title">{title}</div>
          <button type="button" className="btn-modal-close" title={t("common.close")} onClick={onClose}>
            ✕
          </button>
        </header>
        <div className="app-modal-body">{children}</div>
        {footer ? (
          <div className="dialog-footer dialog-footer--pad-sm dialog-footer--surface app-modal-footer">
            {footer}
          </div>
        ) : null}
      </div>
    </div>
  );
}
