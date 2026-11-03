// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
    const onTeamClear = () => {
      revisionRef.current = 0;
      setLines([]);
      delayRef.current = POLL_MIN_MS;
      void refresh(true);
    };
    document.addEventListener("visibilitychange", onVis);
    window.addEventListener("stathive-logs-cleared", onTeamClear);
    return () => {
      cancelled = true;
      if (timerRef.current) clearTimeout(timerRef.current);
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("stathive-logs-cleared", onTeamClear);
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
