import { useCallback, useEffect, useRef, useState } from "react";
import { hostApi } from "../bridge/hostApi";
import { useTabsLayoutCaps } from "../hooks/useTabsLayoutCaps";
import { useAppDialogs } from "../components/useAppDialogs";
import { CalculatorCardBody, isProfitDirty } from "./calculator/CalculatorCardBody";
import {
  CALC_LOOKUP_INVALIDATE_EVENT,
} from "./calculator/calcLookupCache";
import type { CardResult } from "./calculator/calculatorTypes";
import {
  exportExcelAction,
  openExternalUrlAction,
} from "./calculator/calculatorTablesActions";
import {
  computeOrderCost,
  formatProfitRu,
  type ProfitEditKey,
} from "./calculator/evaluateProfit";
import { useCalculatorSnapshot } from "./calculator/useCalculatorSnapshot";
import "./CalculatorViewPage.css";
import "./WbCalculatorLayout.css";
import { useT } from "../i18n";

type Values = Record<string, string>;

const EMPTY: Values = {
  photo: "",
  dimLength: "",
  dimWidth: "",
  dimHeight: "",
  dimWeight: "",
  volumeLiters: "",
  tnved: "",
  docNumber: "",
  docStart: "",
  docEnd: "",
  docCheckStatus: "",
  cabinet: "",
  brand: "",
  vendorCode: "",
  ozOfferId: "",
  category: "",
  name: "",
  ozName: "",
  stockFbo: "",
  stockFbs: "",
  ozStock: "",
  ozFbsStock: "",
  rating: "",
  feedbacks: "",
  clicks: "",
  clicksPrev: "",
  clicksDelta: "",
  wishlist: "",
  cart: "",
  cartPrev: "",
  cartDelta: "",
  hasAds: "",
  orders: "",
  ordersPrev: "",
  ordersDelta: "",
  sales: "",
  salesPrev: "",
  clickToCart: "",
  clickToOrder: "",
  cartToOrder: "",
  price: "",
  ozPrice: "",
  discount: "",
  priceAfterDiscount: "",
  purchase: "",
  commissionFbo: "",
  commissionFbs: "",
  logistics: "",
  adExpenses: "",
  orderCost: "",
};

type EditKey = ProfitEditKey;

/** Session-only: article + last card; clears when the app process exits. */
type StoredLookup = {
  article: string;
  market: string;
  values: Values;
  baseline: Values;
  hint: string | null;
};

function isOzonMarketplace(mp: string | undefined | null): boolean {
  const m = (mp ?? "").toLowerCase();
  return m.includes("ozon") && m !== "both";
}

function storageKey(bundleId: string) {
  return `calc:lastLookup:${bundleId}`;
}

function sameArticle(a: string, b: string) {
  return a.localeCompare(b, undefined, { sensitivity: "accent" }) === 0;
}

export default function CalculatorViewPage() {
  const t = useT();
  const { snap } = useCalculatorSnapshot("view");
  const { alert, dialogs } = useAppDialogs();
  const [article, setArticle] = useState("");
  const [market, setMarket] = useState("");
  const [values, setValues] = useState<Values>({ ...EMPTY });
  const [baseline, setBaseline] = useState<Values>({ ...EMPTY });
  const [busy, setBusy] = useState(false);
  const [hint, setHint] = useState<string | null>(
    t("calculator.view.initialHint"),
  );
  const [openTipId, setOpenTipId] = useState<string | null>(null);
  const restoredRef = useRef(false);
  const loadedArticleRef = useRef("");
  const hasCardRef = useRef(false);
  const [hasCard, setHasCard] = useState(false);
  /** Артикул на карточке — только после lookup/restore (набор в input не трогает сетку). */
  const [shownArticle, setShownArticle] = useState("");
  const layoutCaps = useTabsLayoutCaps();
  const canExcel = layoutCaps.calcAccess?.excel === true;

  const tablesMp = (snap?.tablesMarketplace || snap?.marketplace || "wb").toLowerCase();
  const cardMarket = (market || tablesMp || "wb").toLowerCase();
  const isOzonCard = isOzonMarketplace(cardMarket);
  const articlePlaceholder =
    tablesMp === "both"
      ? t("calculator.view.articlePlaceholderBoth")
      : tablesMp === "ozon"
        ? t("calculator.view.articlePlaceholderOzon")
        : t("calculator.view.articlePlaceholderWb");

  const dirty = isProfitDirty(values, baseline);

  const persistTimerRef = useRef<number | null>(null);
  const pendingPersistRef = useRef<StoredLookup | null>(null);
  const snapBundleRef = useRef(snap?.bundleId);
  snapBundleRef.current = snap?.bundleId;

  function flushPersistSession() {
    if (persistTimerRef.current != null) {
      window.clearTimeout(persistTimerRef.current);
      persistTimerRef.current = null;
    }
    const payload = pendingPersistRef.current;
    const bundleId = snapBundleRef.current;
    if (!payload || !bundleId) return;
    try {
      sessionStorage.setItem(storageKey(bundleId), JSON.stringify(payload));
    } catch {
      // ignore quota / private mode
    }
  }

  function persistSession(payload: StoredLookup, immediate = false) {
    if (!snapBundleRef.current) return;
    pendingPersistRef.current = payload;
    if (immediate) {
      flushPersistSession();
      return;
    }
    if (persistTimerRef.current != null) {
      window.clearTimeout(persistTimerRef.current);
    }
    persistTimerRef.current = window.setTimeout(flushPersistSession, 300);
  }

  useEffect(() => {
    return () => {
      flushPersistSession();
    };
  }, []);

  function readSession(): StoredLookup | null {
    if (!snap) return null;
    try {
      const raw = sessionStorage.getItem(storageKey(snap.bundleId));
      if (!raw) return null;
      const parsed = JSON.parse(raw) as Partial<StoredLookup>;
      if (!parsed?.article?.trim() || !parsed.values) return null;
      return {
        article: parsed.article.trim(),
        market: (parsed.market ?? "").trim(),
        values: { ...EMPTY, ...parsed.values },
        baseline: { ...EMPTY, ...(parsed.baseline ?? parsed.values) },
        hint: parsed.hint ?? null,
      };
    } catch {
      return null;
    }
  }

  function restoreFromStored(stored: StoredLookup) {
    setArticle(stored.article);
    if (stored.market) setMarket(stored.market);
    setValues(stored.values);
    setBaseline(stored.baseline);
    setHint(stored.hint);
    loadedArticleRef.current = stored.article.trim();
    setShownArticle(stored.article.trim());
    hasCardRef.current = true;
    setHasCard(true);
  }

  function applyLookup(
    articleKey: string,
    next: Values,
    nextHint: string | null,
    nextMarket: string,
  ) {
    const baselineNext = { ...next };
    const mp = nextMarket || market || "wb";
    setMarket(mp);
    setValues(next);
    setBaseline(baselineNext);
    setHint(nextHint);
    loadedArticleRef.current = articleKey.trim();
    setShownArticle(articleKey.trim());
    hasCardRef.current = true;
    setHasCard(true);
    persistSession(
      {
        article: articleKey.trim(),
        market: mp,
        values: next,
        baseline: baselineNext,
        hint: nextHint,
      },
      true,
    );
  }

  const patch = useCallback(
    (key: EditKey, next: string) => {
      setValues((prev) => {
        const merged = { ...prev, [key]: next };
        if (key === "adExpenses") {
          const ad = Number(String(next).replace(/\s/g, "").replace(",", ".")) || 0;
          const orders =
            Number(String(prev.orders ?? "").replace(/\s/g, "").replace(",", ".")) || 0;
          const oc = computeOrderCost(ad, orders);
          merged.orderCost = formatProfitRu(oc) ?? "0";
        }
        persistSession({
          article: (loadedArticleRef.current || "").trim(),
          market: market || "wb",
          values: merged,
          baseline,
          hint,
        });
        return merged;
      });
    },
    [baseline, hint, market],
  );

  /**
   * Площадка для lookup: явный override → режим таблиц wb|ozon →
   * уже известная площадка того же артикула (без повторного прогрева индекса) →
   * иначе auto (индекс на бэкенде).
   */
  function resolveLookupMp(nextMarket?: string): string | undefined {
    if (nextMarket?.trim()) return nextMarket.trim().toLowerCase();
    if (tablesMp === "ozon" || tablesMp === "wb") return tablesMp;
    const known = (market || "").trim().toLowerCase();
    const a = (article || "").trim();
    if (
      (known === "wb" || known === "ozon") &&
      a &&
      sameArticle(a, loadedArticleRef.current)
    ) {
      return known;
    }
    return undefined;
  }

  useEffect(() => {
    const onInvalidate = () => {
      loadedArticleRef.current = "";
      hasCardRef.current = false;
      setHasCard(false);
      setShownArticle("");
      setValues({ ...EMPTY });
      setBaseline({ ...EMPTY });
      setHint(t("calculator.view.tablesUpdatedHint"));
    };
    window.addEventListener(CALC_LOOKUP_INVALIDATE_EVENT, onInvalidate);
    return () => window.removeEventListener(CALC_LOOKUP_INVALIDATE_EVENT, onInvalidate);
  }, []);

  /** Network reload from GS/summary. force=true — сброс RAM и перечитка Sheets. */
  async function lookup(nextArticle?: string, nextMarket?: string, force = false) {
    const a = (nextArticle ?? article).trim();
    if (!a || !snap) return;
    const mp = resolveLookupMp(nextMarket);
    setBusy(true);
    setHint(null);
    try {
      const res = (await hostApi.wbCalculator.lookupCard(a, mp, force)) as CardResult;
      if (res.marketplace) setMarket(res.marketplace);
      const next = { ...EMPTY, ...(res.values ?? {}) };
      const nextHint = res.ok
        ? res.message || null
        : res.message || t("calculator.view.articleNotFoundFallback");
      applyLookup(a, next, nextHint, res.marketplace || mp || "wb");
      setHasCard(true);
    } catch (e) {
      // Ошибка сети не сбрасывает уже загруженную карточку.
      setHint(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  /** «Показать»: новый артикул → сеть; тот же что в сессии → из кэша без GS. */
  async function showArticle() {
    const a = article.trim();
    if (!a) return;
    if (sameArticle(a, loadedArticleRef.current) && hasCardRef.current) return;
    const stored = readSession();
    if (stored && sameArticle(a, stored.article)) {
      restoreFromStored(stored);
      return;
    }
    await lookup(a);
  }

  function resetToDefaults() {
    const restored = { ...baseline };
    setValues(restored);
    persistSession({
      article: (article || loadedArticleRef.current).trim(),
      market: market || "wb",
      values: restored,
      baseline,
      hint,
    });
  }

  useEffect(() => {
    if (!snap || restoredRef.current) return;
    restoredRef.current = true;
    const stored = readSession();
    if (stored) {
      restoreFromStored(stored);
      return;
    }
    try {
      const raw = sessionStorage.getItem(storageKey(snap.bundleId));
      if (!raw) return;
      const parsed = JSON.parse(raw) as { article?: string; market?: string };
      if (parsed?.article?.trim()) setArticle(parsed.article);
      if (parsed?.market?.trim()) setMarket(parsed.market);
    } catch {
      // ignore
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- restore once per mount when snap ready
  }, [snap]);

  async function exportExcel() {
    setBusy(true);
    try {
      await exportExcelAction({ alert });
    } catch (e) {
      await alert(e instanceof Error ? e.message : String(e), t("common.error"));
    } finally {
      setBusy(false);
    }
  }

  const editDisabled = !hasCard || busy;

  if (!snap) {
    return (
      <div className="page wb-calc">
        <p className="muted">{hint || t("common.loading")}</p>
        {dialogs}
      </div>
    );
  }

  return (
    <div className="page wb-calc">
      <div className="wb-calc-header">
        <button
          type="button"
          className="btn btn-outline btn-md calc-refresh-data"
          disabled={!article.trim() || busy}
          title={t("calculator.view.refreshDataTitle")}
          onClick={() => void lookup(undefined, undefined, true)}
        >
          {t("calculator.view.refreshData")}
        </button>
        <div className="wb-calc-header-actions calc-lookup-bar">
          {hasCard && (
            <>
              <button
                type="button"
                className="btn btn-outline btn-md"
                disabled={!article.trim() || busy}
                title={
                  isOzonCard
                    ? t("calculator.view.openOzonTitle")
                    : t("calculator.view.openWbTitle")
                }
                onClick={() => {
                  const id = article.trim();
                  if (!id) return;
                  const url = isOzonCard
                    ? `https://www.ozon.ru/product/${encodeURIComponent(id)}/`
                    : `https://www.wildberries.ru/catalog/${encodeURIComponent(id)}/detail.aspx`;
                  void openExternalUrlAction(url).catch((e) =>
                    setHint(e instanceof Error ? e.message : String(e)),
                  );
                }}
              >
                {t("common.open")}
              </button>
              <span className="badge calc-market-badge" title={t("calculator.view.marketBadgeTitle")}>
                {isOzonCard ? t("calculator.marketplace.ozon") : "WB"}
              </span>
            </>
          )}
          <input
            className="input calc-article-input"
            placeholder={articlePlaceholder}
            value={article}
            disabled={busy}
            onChange={(e) => setArticle(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void showArticle();
            }}
          />
          <button
            type="button"
            className="btn btn-primary btn-md"
            disabled={!article.trim() || busy}
            onClick={() => void showArticle()}
          >
            {t("calculator.view.show")}
          </button>
          <button
            type="button"
            className="btn btn-outline btn-md"
            disabled={!hasCard || !dirty || busy}
            title={t("calculator.view.resetDefaultsTitle")}
            onClick={resetToDefaults}
          >
            {t("calculator.view.resetDefaults")}
          </button>
          {canExcel ? (
            <button
              type="button"
              className="btn btn-outline btn-md"
              disabled={busy}
              onClick={() => void exportExcel()}
            >
              Excel
            </button>
          ) : null}
        </div>
      </div>

      {hint ? <p className="plate plate--pad calc-banner">{hint}</p> : null}

      <CalculatorCardBody
        values={values}
        baseline={baseline}
        shownArticle={shownArticle}
        isOzonCard={isOzonCard}
        snap={snap}
        cardMarket={cardMarket}
        editDisabled={editDisabled}
        openTipId={openTipId}
        setOpenTipId={setOpenTipId}
        onPatch={patch}
      />
      {dialogs}
    </div>
  );
}
