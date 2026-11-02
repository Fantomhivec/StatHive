import { memo, useCallback, useMemo } from "react";
import type { CalculatorSnapshot } from "./calculatorTypes";
import { buildCalcTips, conversionShown, orderCostShown, volumeShown, wbOrderParts } from "./buildCalcTips";
import {
  EditableMetric,
  fmtDim,
  Metric,
  ProfitCell,
  Section,
} from "./calcViewWidgets";
import { t } from "../../i18n";
import {
  evaluateProfit,
  evaluateProfitFact,
  formatProfitPctRu,
  formatProfitRu,
  PROFIT_EDIT_KEYS,
  profitPercentOfSellPrice,
  type ProfitEditKey,
} from "./evaluateProfit";

type Values = Record<string, string>;

function firstLine(raw?: string): string {
  const s = (raw ?? "").trim();
  if (!s) return "";
  return s.split(/\r?\n/)[0]?.trim() ?? "";
}

function isDocCheckPassed(status?: string): boolean {
  return firstLine(status).toLowerCase() === t("calculator.card.keysCheckPassed");
}

export type CalculatorCardBodyProps = {
  values: Values;
  baseline: Values;
  shownArticle: string;
  isOzonCard: boolean;
  snap: CalculatorSnapshot;
  cardMarket: string;
  editDisabled: boolean;
  openTipId: string | null;
  setOpenTipId: (id: string | null) => void;
  onPatch: (key: ProfitEditKey, next: string) => void;
};

function CalculatorCardBodyInner({
  values,
  baseline,
  shownArticle,
  isOzonCard,
  snap,
  cardMarket,
  editDisabled,
  openTipId,
  setOpenTipId,
  onPatch,
}: CalculatorCardBodyProps) {
  const v = values;

  const volume = useMemo(() => volumeShown(values), [values]);
  const wbOrders = useMemo(() => {
    if (isOzonCard) return values;
    return {
      ...values,
      orders: values.orders?.trim()
        ? wbOrderParts(values.orders, values.cancels, values.ordersGross).totalText
        : (values.orders ?? ""),
      ordersPrev: values.ordersPrev?.trim()
        ? wbOrderParts(values.ordersPrev, values.cancelsPrev, values.ordersPrevGross).totalText
        : (values.ordersPrev ?? ""),
    };
  }, [values, isOzonCard]);

  const rates = useMemo(() => {
    if (isOzonCard) {
      return {
        taxPercent: snap.ozonTaxPercent ?? 0,
        acquiringPercent: snap.ozonAcquiringPercent ?? 0,
        advPercent: snap.ozonAdvPercent ?? 0,
        extraExpensesPercent: snap.ozonExtraExpensesPercent ?? 0,
      };
    }
    return {
      taxPercent: snap.taxPercent ?? 0,
      acquiringPercent: snap.acquiringPercent ?? 0,
      advPercent: snap.advPercent ?? 0,
      extraExpensesPercent: snap.extraExpensesPercent ?? 0,
    };
  }, [snap, isOzonCard]);

  const profitFormula = isOzonCard ? snap.ozonProfitFormula : snap.profitFormula;

  const profit = useMemo(
    () => evaluateProfit(profitFormula, values, rates),
    [profitFormula, values, rates],
  );
  const profitFact = useMemo(
    () => evaluateProfitFact(profit, values, rates.advPercent),
    [profit, values, rates.advPercent],
  );
  const margin = useMemo(() => profitPercentOfSellPrice(profit, values), [profit, values]);
  const marginFact = useMemo(
    () => profitPercentOfSellPrice(profitFact, values),
    [profitFact, values],
  );

  /** Полный buildCalcTips только пока открыт tip — иначе на каждый ввод в метриках. */
  const tips = useMemo(() => {
    if (!openTipId) return null;
    return buildCalcTips(values, snap, cardMarket);
  }, [openTipId, values, snap, cardMarket]);

  const emptyTip = useMemo(() => ({ title: "", lines: [] as string[] }), []);
  const tipOf = useCallback(
    (key: keyof NonNullable<typeof tips>) => (tips?.[key] ?? emptyTip),
    [tips, emptyTip],
  );

  const tipProps = useMemo(
    () => ({ openTipId, setOpenTipId }),
    [openTipId, setOpenTipId],
  );

  const fieldDirty = useCallback(
    (key: ProfitEditKey) => (values[key] ?? "") !== (baseline[key] ?? ""),
    [values, baseline],
  );

  return (
    <section className="calc-card-grid">
      <aside className="calc-card-aside">
        <div className="card card--md card--pad calc-card-photo">
          {v.photo ? (
            <img src={v.photo} alt="" className="calc-photo-img" referrerPolicy="no-referrer" />
          ) : (
            <div className="calc-photo-placeholder">{t("calculator.card.noPhoto")}</div>
          )}
        </div>

        <Section title={t("calculator.card.params")} tone="params">
          <div className="calc-params-grid">
            <Metric label={t("calculator.card.length")} value={fmtDim(v.dimLength, t("calculator.card.unitCm"))} />
            <Metric label={t("calculator.card.width")} value={fmtDim(v.dimWidth, t("calculator.card.unitCm"))} />
            <Metric label={t("calculator.card.height")} value={fmtDim(v.dimHeight, t("calculator.card.unitCm"))} />
            <Metric label={t("calculator.card.weight")} value={fmtDim(v.dimWeight, t("calculator.card.unitKg"))} />
            <Metric
              label={t("calculator.card.volume")}
              value={fmtDim(volume, t("calculator.card.unitL"))}
              tipId="volume"
              tip={tipOf("volume")}
              {...tipProps}
            />
          </div>
          <div className="plate plate--tint calc-aside-plate">
            <div className="calc-aside-plate-label">{t("calculator.card.tnved")}</div>
            <div className="calc-aside-plate-value">{v.tnved?.trim() || "—"}</div>
          </div>
        </Section>

        {!isOzonCard &&
          (v.docNumber || v.docStart || v.docEnd || v.docCheckStatus) && (
            <Section
              title={t("calculator.card.documents")}
              tone="docs"
              success={isDocCheckPassed(v.docCheckStatus)}
            >
              <div className="calc-docs-values">
                <div className="plate plate--tint calc-docs-value">{firstLine(v.docNumber) || "—"}</div>
                <div className="plate plate--tint calc-docs-value">{firstLine(v.docStart) || "—"}</div>
                <div className="plate plate--tint calc-docs-value">{firstLine(v.docEnd) || "—"}</div>
                <div className="plate plate--tint calc-docs-value">
                  {firstLine(v.docCheckStatus) || "—"}
                </div>
              </div>
            </Section>
          )}
      </aside>

      <div className="calc-card-main">
        <Section title={t("calculator.card.mainInfo")} tone="id">
          <div className="calc-card-top">
            <Metric label={t("calculator.card.article")} value={shownArticle || undefined} numeric={false} />
            <Metric label={t("common.cabinet")} value={v.cabinet} numeric={false} />
            <Metric label={t("calculator.card.brand")} value={v.brand} numeric={false} />
            <Metric
              label={t("calculator.card.vendorCode")}
              value={v.vendorCode || v.ozOfferId}
              numeric={false}
            />
          </div>
          <div className="calc-metrics-row calc-metrics-row--category-name">
            <Metric label={t("calculator.card.category")} value={v.category} numeric={false} />
            <div className="plate calc-name-block">
              <div className="calc-metric-label">{t("calculator.card.productName")}</div>
              <div className="calc-name-value">{v.name || v.ozName || "—"}</div>
            </div>
          </div>
        </Section>

        <Section title={t("calculator.card.stockRating")} tone="stock">
          <div className="calc-metrics-row">
            <Metric label={t("calculator.card.stockFbo")} value={v.stockFbo || v.ozStock} />
            <Metric label={t("calculator.card.stockFbs")} value={v.stockFbs || v.ozFbsStock} />
            <Metric label={t("calculator.card.rating")} value={v.rating} />
            <Metric label={t("calculator.card.feedbacks")} value={v.feedbacks} />
          </div>
        </Section>

        {isOzonCard ? (
          <Section title={t("calculator.card.orders")} tone="funnel">
            <div className="calc-metrics-row">
              <Metric
                label={t("calculator.card.orders")}
                value={v.orders}
                tipId="orders"
                tip={tipOf("orders")}
                {...tipProps}
              />
              <Metric
                label={t("calculator.card.ordersPrev")}
                value={v.ordersPrev}
                tipId="ordersPrev"
                tip={tipOf("ordersPrev")}
                {...tipProps}
              />
              <Metric label={t("calculator.card.ordersDelta")} value={v.ordersDelta} />
            </div>
          </Section>
        ) : (
          <Section title={t("calculator.card.funnel")} tone="funnel">
            <div className="calc-metrics-row">
              <Metric label={t("calculator.card.clicks")} value={v.clicks} />
              <Metric label={t("calculator.card.clicksPrev")} value={v.clicksPrev} />
              <Metric
                label={t("calculator.card.clicksDelta")}
                value={v.clicksDelta}
                tipId="clicksDelta"
                tip={tipOf("clicksDelta")}
                {...tipProps}
              />
              <Metric
                label={t("calculator.card.clickToCart")}
                value={conversionShown(v, "cart", "clicks", "clickToCart")}
                tipId="clickToCart"
                tip={tipOf("clickToCart")}
                {...tipProps}
              />
            </div>
            <div className="calc-metrics-row">
              <Metric label={t("calculator.card.cart")} value={v.cart} />
              <Metric label={t("calculator.card.cartPrev")} value={v.cartPrev} />
              <Metric
                label={t("calculator.card.cartDelta")}
                value={v.cartDelta}
                tipId="cartDelta"
                tip={tipOf("cartDelta")}
                {...tipProps}
              />
              <Metric
                label={t("calculator.card.cartToOrder")}
                value={conversionShown(wbOrders, "orders", "cart", "cartToOrder")}
                tipId="cartToOrder"
                tip={tipOf("cartToOrder")}
                {...tipProps}
              />
            </div>
            <div className="calc-metrics-row">
              <Metric
                label={t("calculator.card.orders")}
                value={wbOrders.orders}
                tipId="orders"
                tip={tipOf("orders")}
                {...tipProps}
              />
              <Metric
                label={t("calculator.card.ordersPrev")}
                value={wbOrders.ordersPrev}
                tipId="ordersPrev"
                tip={tipOf("ordersPrev")}
                {...tipProps}
              />
              <Metric
                label={t("calculator.card.ordersDelta")}
                value={v.ordersDelta}
                tipId="ordersDelta"
                tip={tipOf("ordersDelta")}
                {...tipProps}
              />
              <Metric
                label={t("calculator.card.clickToOrder")}
                value={conversionShown(wbOrders, "orders", "clicks", "clickToOrder")}
                tipId="clickToOrder"
                tip={tipOf("clickToOrder")}
                {...tipProps}
              />
            </div>
            <div className="calc-metrics-row">
              <Metric label={t("calculator.card.sales")} value={v.sales} />
              <Metric label={t("calculator.card.salesPrev")} value={v.salesPrev} />
              <Metric label={t("calculator.card.wishlist")} value={v.wishlist} maxFractionDigits={0} />
              <Metric label={t("calculator.card.hasAds")} value={v.hasAds} />
            </div>
          </Section>
        )}

        <Section title={t("calculator.card.prices")} tone="price">
          <div className="calc-metrics-row">
            <Metric
              label={isOzonCard ? t("calculator.card.priceAfterDiscount") : t("calculator.card.price")}
              value={v.price || v.ozPrice}
            />
            <Metric label={t("calculator.card.discount")} value={v.discount} suffix="%" />
            <EditableMetric
              label={isOzonCard ? t("calculator.card.priceAfterPromo") : t("calculator.card.priceAfterDiscount")}
              value={v.priceAfterDiscount}
              dirty={fieldDirty("priceAfterDiscount")}
              disabled={editDisabled}
              onChange={(n) => onPatch("priceAfterDiscount", n)}
            />
            <EditableMetric
              label={t("calculator.card.purchase")}
              value={v.purchase}
              dirty={fieldDirty("purchase")}
              disabled={editDisabled}
              onChange={(n) => onPatch("purchase", n)}
            />
          </div>
        </Section>

        <Section title={t("calculator.card.costs")} tone="cost">
          <div
            className={`calc-metrics-row${isOzonCard ? "" : " calc-metrics-row--costs5"}`}
          >
            <EditableMetric
              label={t("calculator.card.commissionFbo")}
              value={v.commissionFbo}
              dirty={fieldDirty("commissionFbo")}
              disabled={editDisabled}
              suffix="%"
              onChange={(n) => onPatch("commissionFbo", n)}
            />
            <EditableMetric
              label={t("calculator.card.commissionFbs")}
              value={v.commissionFbs}
              dirty={fieldDirty("commissionFbs")}
              disabled={editDisabled}
              suffix="%"
              onChange={(n) => onPatch("commissionFbs", n)}
            />
            <EditableMetric
              label={t("calculator.card.logistics")}
              value={v.logistics}
              dirty={fieldDirty("logistics")}
              disabled={editDisabled}
              onChange={(n) => onPatch("logistics", n)}
              tipId="logistics"
              tip={tipOf("logistics")}
              {...tipProps}
            />
            {!isOzonCard ? (
              <EditableMetric
                label={t("calculator.card.adExpenses2w")}
                value={v.adExpenses}
                dirty={fieldDirty("adExpenses")}
                disabled={editDisabled}
                onChange={(n) => onPatch("adExpenses", n)}
              />
            ) : null}
            {!isOzonCard ? (
              <Metric
                label={t("calculator.card.orderCost")}
                value={orderCostShown(wbOrders)}
                tipId="orderCost"
                tip={tipOf("orderCost")}
                {...tipProps}
              />
            ) : null}
          </div>
        </Section>

        <div className="section-tint calc-profit-row calc-profit-row--quad">
          <ProfitCell
            title={t("calculator.card.profitRub")}
            value={formatProfitRu(profit) ?? "—"}
            tipId="profitPlanned"
            tip={tipOf("profitPlanned")}
            {...tipProps}
          />
          <ProfitCell title={t("calculator.card.profitPct")} value={formatProfitPctRu(margin) ?? "—"} />
          <ProfitCell
            title={t("calculator.card.profitFactRub")}
            value={formatProfitRu(profitFact) ?? "—"}
            tipId="profitFact"
            tip={tipOf("profitFact")}
            {...tipProps}
          />
          <ProfitCell title={t("calculator.card.profitFactPct")} value={formatProfitPctRu(marginFact) ?? "—"} />
        </div>
      </div>
    </section>
  );
}

export const CalculatorCardBody = memo(CalculatorCardBodyInner);

/** Для кнопки «Вернуть по умолчанию» в шапке — без пересчёта всей сетки. */
export function isProfitDirty(values: Values, baseline: Values): boolean {
  return PROFIT_EDIT_KEYS.some((k) => (values[k] ?? "") !== (baseline[k] ?? ""));
}
