import { useCallback, useEffect, useRef, useState } from "react";
import { hostApi, LogLine } from "../bridge/hostApi";
import { useT } from "../i18n";

const POLL_MIN_MS = 1500;
const POLL_MAX_MS = 12000;

export default function LogsPage() {
  const t = useT();
  const [lines, setLines] = useState<LogLine[]>([]);
  const delayRef = useRef(POLL_MIN_MS);
  const revisionRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const refresh = useCallback(async (force = false) => {
    const known = force ? 0 : revisionRef.current;
    const res = await hostApi.logsGet(known);
    revisionRef.current = res.revision;
    if (!force && res.unchanged) {
      delayRef.current = Math.min(POLL_MAX_MS, Math.round(delayRef.current * 1.5));
      return;
    }
    delayRef.current = POLL_MIN_MS;
    setLines(res.lines ?? []);
  }, []);

  useEffect(() => {
    let cancelled = false;
    const schedule = () => {
      if (cancelled) return;
      timerRef.current = setTimeout(() => {
        void (async () => {
          if (document.visibilityState !== "hidden") await refresh(false);
          schedule();
        })();
      }, delayRef.current);
    };
    void refresh(true).then(schedule);
    const onVis = () => {
      if (document.visibilityState !== "visible") return;
      delayRef.current = POLL_MIN_MS;
      void refresh(true);
    };
    document.addEventListener("visibilitychange", onVis);
    return () => {
      cancelled = true;
      if (timerRef.current) clearTimeout(timerRef.current);
      document.removeEventListener("visibilitychange", onVis);
    };
  }, [refresh]);

  return (
    <div>
      <h1 className="page-title">{t("logs.title")}</h1>
      <div className="toolbar">
        <button className="btn btn-ghost" type="button" onClick={() => void refresh(true)}>
          {t("common.refresh")}
        </button>
        <button
          className="btn btn-ghost"
          type="button"
          onClick={() => void hostApi.logsClear().then(() => refresh(true))}
        >
          {t("common.clear")}
        </button>
      </div>
      <div className="mono-panel mono-panel--on-panel logs">
        {lines.length === 0 && <div className="page-sub">{t("common.empty")}</div>}
        {lines.map((l, i) => (
          <div key={`${l.ts}-${i}`} className={l.level}>
            [{l.ts}] {l.level}: {l.message}
          </div>
        ))}
      </div>
    </div>
  );
}
