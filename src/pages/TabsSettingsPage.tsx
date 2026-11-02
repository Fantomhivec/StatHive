import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ensureTabsLayout, hostApi, type TabLayoutItem } from "../bridge/hostApi";
import AppCustomWindow from "../components/AppCustomWindow";
import AppMessageBox from "../components/AppMessageBox";
import "./TabsSettingsPage.css";
import { useT } from "../i18n";

export default function TabsSettingsPage() {
  const t = useT();
  const navigate = useNavigate();
  const [items, setItems] = useState<TabLayoutItem[]>([]);
  const [snapshot, setSnapshot] = useState<TabLayoutItem[]>([]);
  const [calculatorMode, setCalculatorMode] = useState(false);
  const [snapshotMode, setSnapshotMode] = useState(false);
  const [canCalculatorMode, setCanCalculatorMode] = useState(false);
  const [forceCalculatorMode, setForceCalculatorMode] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    void ensureTabsLayout()
      .then((res) => {
        setItems(res.items);
        setSnapshot(res.items.map((x) => ({ ...x })));
        setCanCalculatorMode(!!res.canCalculatorMode);
        setForceCalculatorMode(!!res.forceCalculatorMode);
        setCalculatorMode(!!res.calculatorMode && !!res.canCalculatorMode);
        setSnapshotMode(!!res.calculatorMode && !!res.canCalculatorMode);
        if (res.forceCalculatorMode) {
          navigate("/calculator", { replace: true });
        }
      })
      .catch((e: Error) => setError(e.message));
  }, [navigate]);

  function close() {
    navigate(-1);
  }

  function move(index: number, delta: number) {
    setItems((prev) => {
      const next = [...prev];
      const j = index + delta;
      if (j < 0 || j >= next.length) return prev;
      [next[index], next[j]] = [next[j], next[index]];
      return next.map((it, order) => ({ ...it, order }));
    });
  }

  function toggleVisible(tabKey: string, checked: boolean) {
    setItems((prev) => {
      const next = prev.map((x) => (x.tabKey === tabKey ? { ...x, isVisible: checked } : x));
      if (checked) return next;
      if (next.some((x) => x.isVisible)) return next;
      if (next.length > 0) {
        next[0] = { ...next[0], isVisible: true };
        setMsg(t("tabsSettings.hideAllTabsError"));
      }
      return next;
    });
  }

  return (
    <>
      <AppCustomWindow
        title={t("host.tabsSettings")}
        maximized
        closeOnOverlay={false}
        onClose={close}
        bodyClassName="tabs-settings-body-wrap"
      >
        <div className="tabs-settings-inner">
          <div className="plate plate--pad tabs-settings-body">
            {canCalculatorMode && !forceCalculatorMode && (
              <div className="card card--md card--pad tabs-settings-mode-row">
                <div>
                  <div className="tabs-settings-mode-title">{t("tabsSettings.calculatorModeTitle")}</div>
                  <p className="tabs-settings-hint u-m-0">{t("tabsSettings.calculatorModeHint")}</p>
                </div>
                <label className="toggle-switch" title={t("tabsSettings.calculatorModeTitle")}>
                  <input
                    type="checkbox"
                    checked={calculatorMode}
                    onChange={(e) => setCalculatorMode(e.target.checked)}
                  />
                  <span className="toggle-track" />
                </label>
              </div>
            )}

            {!calculatorMode && (
              <>
                <p className="tabs-settings-hint">{t("tabsSettings.orderHint")}</p>
                {error && <p className="error">{error}</p>}

                <div className="plate tabs-settings-list">
                  {items.map((it, idx) => (
                    <div key={it.tabKey} className="card card--md card--pad-sm tabs-settings-row">
                      <div className="tabs-settings-arrows">
                        <button
                          type="button"
                          className="reorder-arrow"
                          title={t("tabsSettings.moveUpTitle")}
                          onClick={() => move(idx, -1)}
                        >
                          ▲
                        </button>
                        <button
                          type="button"
                          className="reorder-arrow"
                          title={t("tabsSettings.moveDownTitle")}
                          onClick={() => move(idx, 1)}
                        >
                          ▼
                        </button>
                      </div>
                      <div className="tabs-settings-name">{it.title}</div>
                      <label className="toggle-switch" title={t("tabsSettings.visibilityTitle")}>
                        <input
                          type="checkbox"
                          checked={it.isVisible}
                          onChange={(e) => toggleVisible(it.tabKey, e.target.checked)}
                        />
                        <span className="toggle-track" />
                      </label>
                    </div>
                  ))}
                </div>
              </>
            )}

            {calculatorMode && (
              <p className="tabs-settings-hint">{t("tabsSettings.calculatorModeOnHint")}</p>
            )}
          </div>

          <footer className="dialog-footer form-actions form-actions--end tabs-settings-footer">
            <button
              className="btn btn-primary btn-minw--lg"
              disabled={busy}
              onClick={async () => {
                setBusy(true);
                setError(null);
                try {
                  // Не передаём false при !can — иначе затираем stored calculator_mode в tabs.layout.json.
                  const res = await hostApi.saveTabsLayout(
                    items,
                    canCalculatorMode ? calculatorMode : undefined,
                  );
                  setItems(res.items);
                  setSnapshot(res.items.map((x) => ({ ...x })));
                  setCanCalculatorMode(!!res.canCalculatorMode);
                  setCalculatorMode(!!res.calculatorMode && !!res.canCalculatorMode);
                  setSnapshotMode(!!res.calculatorMode && !!res.canCalculatorMode);
                  if (res.calculatorMode) navigate("/calculator", { replace: true });
                  else close();
                } catch (e) {
                  setError(e instanceof Error ? e.message : String(e));
                } finally {
                  setBusy(false);
                }
              }}
            >
              {t("common.save")}
            </button>
            <button
              className="btn btn-outline btn-minw--md"
              disabled={busy}
              onClick={() => {
                setItems(snapshot.map((x) => ({ ...x })));
                setCalculatorMode(snapshotMode);
                setError(null);
                close();
              }}
            >
              {t("common.cancel")}
            </button>
          </footer>
        </div>
      </AppCustomWindow>

      {msg && (
        <AppMessageBox title={t("dialogs.tabs")} message={msg} onClose={() => setMsg(null)} zIndex={1200} />
      )}
    </>
  );
}
