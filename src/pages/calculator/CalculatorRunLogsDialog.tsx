import { useEffect, useRef, useState } from "react";
import { hostApi, onEvent } from "../../bridge/hostApi";
import AppCustomWindow from "../../components/AppCustomWindow";
import { useTabsLayoutCaps } from "../../hooks/useTabsLayoutCaps";
import { t } from "../../i18n";

type Props = {
  open: boolean;
  onClose: () => void;
  onError: (message: string) => void;
};

/** Собираем строки в буфер и сбрасываем раз за кадр — без «заморозки» React на пачке emit. */
function useLiveLogText(open: boolean) {
  const [text, setText] = useState("");
  const pendingRef = useRef<string[]>([]);
  const rafRef = useRef<number | null>(null);
  const textRef = useRef("");

  function flushPending() {
    rafRef.current = null;
    const chunk = pendingRef.current;
    if (chunk.length === 0) return;
    pendingRef.current = [];
    setText((prev) => {
      const next = prev ? `${prev}\n${chunk.join("\n")}` : chunk.join("\n");
      textRef.current = next;
      return next;
    });
  }

  function queueLine(line: string) {
    pendingRef.current.push(line);
    if (rafRef.current == null) {
      rafRef.current = window.requestAnimationFrame(flushPending);
    }
  }

  function replaceText(next: string) {
    pendingRef.current = [];
    if (rafRef.current != null) {
      window.cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    textRef.current = next;
    setText(next);
  }

  useEffect(() => {
    if (!open) return;
    return () => {
      if (rafRef.current != null) {
        window.cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
      pendingRef.current = [];
    };
  }, [open]);

  return { text, textRef, queueLine, replaceText };
}

export function CalculatorRunLogsDialog({ open, onClose, onError }: Props) {
  const [busy, setBusy] = useState(false);
  const preRef = useRef<HTMLPreElement>(null);
  const { text, textRef, queueLine, replaceText } = useLiveLogText(open);
  const canClearLogs = useTabsLayoutCaps().tablesCanMutate === true;

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setBusy(true);
    void hostApi.wbCalculator
      .getRunLogs()
      .then((res) => {
        if (!cancelled) replaceText(res.text || "");
      })
      .catch((e) => {
        if (!cancelled) {
          onError(e instanceof Error ? e.message : String(e));
          replaceText("");
        }
      })
      .finally(() => {
        if (!cancelled) setBusy(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load once when opened
  }, [open, onError]);

  const lastEventAtRef = useRef(0);
  useEffect(() => {
    if (!open) return;
    lastEventAtRef.current = Date.now();
    const offEntry = onEvent("wbCalculator.runLog", (payload) => {
      const line = (payload as { line?: string } | null)?.line;
      if (!line) return;
      lastEventAtRef.current = Date.now();
      queueLine(line);
    });
    const offClear = onEvent("wbCalculator.runLogCleared", () => {
      lastEventAtRef.current = Date.now();
      replaceText("");
    });
    return () => {
      offEntry();
      offClear();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- subscribe while open
  }, [open]);

  // Редкий догон: если emit задержался в IPC — подтянуть буфер с хоста.
  useEffect(() => {
    if (!open) return;
    const id = window.setInterval(() => {
      if (Date.now() - lastEventAtRef.current < 2500) return;
      void hostApi.wbCalculator.getRunLogs().then((res) => {
        const next = res.text || "";
        if (next !== textRef.current) replaceText(next);
      });
    }, 3000);
    return () => window.clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const el = preRef.current;
    if (!el) return;
    const dist = el.scrollHeight - el.scrollTop - el.clientHeight;
    if (dist < 48) el.scrollTop = el.scrollHeight;
  }, [open, text]);

  async function refresh() {
    setBusy(true);
    try {
      const res = await hostApi.wbCalculator.getRunLogs();
      replaceText(res.text || "");
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function clear() {
    setBusy(true);
    try {
      await hostApi.wbCalculator.clearRunLogs();
      replaceText("");
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function copy() {
    if (!text.trim()) return;
    try {
      await navigator.clipboard.writeText(text);
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    }
  }

  if (!open) return null;

  return (
    <AppCustomWindow
      title={t("calculator.runLogs.title")}
      onClose={onClose}
      width={900}
      bodyClassName="calc-dialog-body"
      footer={
        <div className="dialog-footer dialog-footer--pad-lg dialog-footer--surface calc-dialog-footer">
          <button type="button" className="btn btn-outline btn-sm" disabled={busy} onClick={() => void refresh()}>
            {t("common.refresh")}
          </button>
          <button
            type="button"
            className="btn btn-outline btn-sm"
            disabled={busy || !text.trim()}
            onClick={() => void copy()}
          >
            {t("common.copy")}
          </button>
          {canClearLogs ? (
            <button
              type="button"
              className="btn btn-outline btn-sm"
              disabled={busy}
              onClick={() => void clear()}
            >
              {t("common.clear")}
            </button>
          ) : null}
          <button type="button" className="btn btn-primary btn-sm" onClick={onClose}>
            {t("common.close")}
          </button>
        </div>
      }
    >
      <p className="calc-logs-hint muted">{t("calculator.runLogs.hint")}</p>
      <pre ref={preRef} className="mono-panel mono-panel--on-panel mono-panel--scroll-thin calc-run-log">
        {busy && !text ? t("common.loading") : text || t("calculator.runLogs.empty")}
      </pre>
    </AppCustomWindow>
  );
}
