import { useEffect, useState } from "react";
import { useT } from "../i18n";
import {
  ensureAppVersion,
  ensureUpdatesCheckQuiet,
  hostApi,
  onEvent,
  UpdateInfo,
} from "../bridge/hostApi";

/** Диалог обновления: загрузка в UI с прогрессом, без console updater. */
export default function UpdateAvailableDialog({ autoCheck = true }: { autoCheck?: boolean }) {
  const t = useT();
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [current, setCurrent] = useState("");

  useEffect(() => {
    void (async () => {
      const versionP = ensureAppVersion().then(setCurrent).catch(() => undefined);
      if (!autoCheck) {
        await versionP;
        return;
      }
      const checkP = ensureUpdatesCheckQuiet()
        .then((u) => {
          if (u.is_newer) setInfo(u);
        })
        .catch(() => undefined);
      await Promise.all([versionP, checkP]);
    })();
  }, [autoCheck]);

  useEffect(() => {
    return onEvent<{ percent?: number }>("updates.progress", (payload) => {
      const p = payload?.percent;
      if (typeof p === "number") setProgress(Math.max(0, Math.min(100, p)));
    });
  }, []);

  if (!info?.is_newer) return null;

  async function install() {
    setBusy(true);
    setErr(null);
    setStatus(t("updates.downloading"));
    setProgress(0);
    try {
      const res = await hostApi.updatesDownloadInstall();
      if (res.success) {
        setStatus(res.message || t("updates.downloadedRestart"));
        return;
      }
      setErr(res.message || t("profile.updateInstallFail"));
      setProgress(null);
    } catch (e) {
      const raw = e instanceof Error ? e.message : String(e);
      setErr(raw || t("updates.errorSeeLogs"));
      setStatus(null);
      setProgress(null);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="overlay overlay--win" role="dialog" aria-modal="true">
      <div className="dialog-panel dialog-panel--win update-dialog" style={{ width: 440 }}>
        <h2>{t("updates.availableTitle")}</h2>
        <p className="page-sub">
          {t("updates.versionLine", {
            current: current || t("common.emDash"),
            latest: info.version,
          })}
        </p>
        {info.body ? (
          <pre className="mono-panel mono-panel--on-bg update-dialog-body">
            {info.body.slice(0, 800)}
          </pre>
        ) : null}
        {status && !err ? <p className="page-sub">{status}</p> : null}
        {progress != null && (
          <div
            className="progress-track update-progress"
            role="progressbar"
            aria-valuenow={Math.round(progress)}
          >
            <div
              className="progress-fill update-progress-bar"
              style={{ width: `${progress}%` }}
            />
            <span className="update-progress-label">{progress.toFixed(0)}%</span>
          </div>
        )}
        {err && <div className="error">{err}</div>}
        <div className="form-actions update-dialog-actions">
          <button className="btn" type="button" disabled={busy} onClick={() => void install()}>
            {busy ? t("common.loading") : t("common.refresh")}
          </button>
          <button
            className="btn btn-ghost"
            type="button"
            disabled={busy}
            onClick={() => setInfo(null)}
          >
            {t("updates.later")}
          </button>
        </div>
      </div>
    </div>
  );
}
