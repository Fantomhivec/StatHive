import { useEffect, useRef, useState } from "react";
import AppCustomWindow from "./AppCustomWindow";
import { useT } from "../i18n";


type Props = {
  title: string;
  label: string;
  initial?: string;
  /** Кнопка справа от поля (например «Обзор…» → проводник). */
  browseLabel?: string;
  onBrowse?: () => Promise<string | null>;
  onClose: (value: string | null) => void;
  zIndex?: number;
};

/** Тематический ввод текста вместо window.prompt (без «localhost»). */
export default function AppPromptDialog({
  title,
  label,
  initial = "",
  browseLabel,
  onBrowse,
  onClose,
  zIndex = 1300,
}: Props) {
  const t = useT();
  const [value, setValue] = useState(initial);
  const [browsing, setBrowsing] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  function submit() {
    onClose(value);
  }

  async function browse() {
    if (!onBrowse || browsing) return;
    setBrowsing(true);
    try {
      const next = await onBrowse();
      if (next != null && next !== "") setValue(next);
    } finally {
      setBrowsing(false);
      inputRef.current?.focus();
    }
  }

  const showBrowse = Boolean(browseLabel && onBrowse);

  return (
    <AppCustomWindow
      title={title}
      width={520}
      zIndex={zIndex}
      tone="info"
      closeOnOverlay={false}
      onClose={() => onClose(null)}
      footer={
        <>
          <button type="button" className="btn btn-ghost" onClick={() => onClose(null)}>
            {t("common.cancel")}
          </button>
          <button type="button" className="btn" onClick={submit} disabled={browsing}>
            {t("common.ok")}
          </button>
        </>
      }
    >
      <label className="app-prompt-label" htmlFor="app-prompt-input">
        {label}
      </label>
      <div className={showBrowse ? "app-prompt-row" : undefined}>
        <input
          id="app-prompt-input"
          ref={inputRef}
          className="input app-prompt-input"
          value={value}
          disabled={browsing}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submit();
            if (e.key === "Escape") onClose(null);
          }}
        />
        {showBrowse ? (
          <button
            type="button"
            className="btn btn-outline app-prompt-browse"
            disabled={browsing}
            title={t("common.openExplorer")}
            onClick={() => void browse()}
          >
            {browsing ? "…" : browseLabel}
          </button>
        ) : null}
      </div>
    </AppCustomWindow>
  );
}
