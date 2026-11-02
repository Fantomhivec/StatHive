import type { CalculatorSnapshot } from './calculatorTypes'
import {
  DEFAULT_PROFIT_FORMULA,
  computeOrderCost,
  evaluateProfit,
  evaluateProfitFact,
  formatProfitRu,
  type ProfitRates,
} from './evaluateProfit'
import { formatCalcDisplay } from './formatCalcNumber'
import { t } from "../../i18n";
import {
  DEFAULT_PROFIT_FORMULA_PRETTY,
  formatFormulaPretty,
  normalizeFormula,
  spaceFormulaOperators,
} from './formulaTokens'

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function fmtPct(n: number): string {
  return `${fmt(n)}%`
}

/** Tip display: money as amounts; commissions/settings rates as percent (6%, not 0,06). */
function formatFormulaWithNumbers(
  formula: string,
  values: Record<string, string>,
  rates: ProfitRates,
): string {
  let expr = normalizeFormula(formula.trim() || DEFAULT_PROFIT_FORMULA_PRETTY)
  const reps: [string, string][] = [
    ['доп_расходы', fmtPct(rates.extraExpensesPercent)],
    ['цена_после_скидки', fmt(parseNum(values.priceAfterDiscount || values.price || values.ozPrice))],
    ['комиссия_fbo', fmtPct(parseNum(values.commissionFbo))],
    ['комиссия_fbs', fmtPct(parseNum(values.commissionFbs))],
    ['эквайринг', fmtPct(rates.acquiringPercent)],
    ['логистика', fmt(parseNum(values.logistics))],
    ['реклама', fmtPct(rates.advPercent)],
    ['налог', fmtPct(rates.taxPercent)],
    ['закуп', fmt(parseNum(values.purchase))],
  ]
  for (const [tag, text] of reps) {
    expr = expr.replace(new RegExp(escapeRegExp(tag), 'gi'), text)
  }
  return spaceFormulaOperators(expr)
}

export type CalcTip = {
  title: string
  lines: string[]
  /** `formula`: title = tokens, lines[0] = numbers (compact 2-line tip). */
  layout?: 'list' | 'formula'
}

function parseNum(raw: string | undefined): number {
  if (!raw?.trim()) return 0
  const s = raw.trim().replace(/%/g, '').replace(/\s/g, '').replace(',', '.')
  const n = Number(s)
  return Number.isFinite(n) ? n : 0
}

function fmt(n: number): string {
  return n.toLocaleString('ru-RU', { maximumFractionDigits: 2 })
}

/** Same rounding the card uses, so a tip total can equal the cell. */
function fmtNum(n: number): string {
  const raw = n
    .toFixed(2)
    .replace(/0+$/, '')
    .replace(/\.$/, '')
    .replace('.', ',')
  return formatCalcDisplay(raw) ?? raw
}

function closeEnough(stored: number, computed: number): boolean {
  return Math.abs(stored - computed) <= 0.05
}

/** Prefer the cell text when it already equals the formula. */
function textMatching(computed: number, storedRaw: string | undefined, suffix = ''): string {
  const stored = storedRaw?.trim() ?? ''
  if (stored && closeEnough(parseNum(stored), computed)) return stored
  return `${fmtNum(computed)}${suffix}`
}

function fmtSignedPct(pct: number): string {
  const raw = `${pct >= 0 ? '+' : ''}${pct.toFixed(1).replace('.', ',')}%`
  return formatCalcDisplay(raw) ?? raw
}

/** Свод хранит чистые заказы; без ordersGross сырые = чистые + отмены, итог = число на карточке. */
export function wbOrderParts(
  netRaw: string | undefined,
  cancelsRaw: string | undefined,
  grossRaw: string | undefined,
): { gross: number; cancels: number; total: number; totalText: string } {
  const net = parseNum(netRaw)
  const cancels = parseNum(cancelsRaw)
  const hasGross = Boolean(grossRaw?.trim())
  const gross = hasGross ? parseNum(grossRaw) : net + cancels
  const total = hasGross ? Math.max(0, gross - cancels) : net
  const totalText = netRaw?.trim() ? textMatching(total, netRaw) : fmtNum(total)
  return { gross, cancels, total, totalText }
}

/** Объём на карточке: формула Д×Ш×В/1000, если она не совпала с ячейкой свода. */
export function volumeShown(values: Record<string, string>): string {
  const l = parseNum(values.dimLength)
  const w = parseNum(values.dimWidth)
  const h = parseNum(values.dimHeight)
  const stored = values.volumeLiters?.trim() ?? ''
  if (!(l > 0 && w > 0 && h > 0)) return stored
  return textMatching(Math.max((l * w * h) / 1000, 0.001), stored)
}

/** Конверсия: числитель / знаменатель × 100, иначе текст ячейки. */
export function conversionShown(
  values: Record<string, string>,
  numKey: string,
  denKey: string,
  outKey: string,
): string {
  const den = parseNum(values[denKey])
  const stored = values[outKey]?.trim() ?? ''
  if (!(Math.abs(den) > 1e-9)) return stored
  return textMatching((parseNum(values[numKey]) / den) * 100, stored, '%')
}

/** Стоимость заказа от чистых заказов (уже минус отмены). */
export function orderCostShown(values: Record<string, string>, ordersNet?: number): string {
  const orders = ordersNet ?? wbOrderParts(values.orders, values.cancels, values.ordersGross).total
  const oc = computeOrderCost(parseNum(values.adExpenses), orders)
  const stored = values.orderCost?.trim() ?? ''
  if (!stored && oc === 0 && !values.adExpenses?.trim() && !values.orders?.trim()) return ''
  return textMatching(oc, stored)
}

function disp(raw: string | undefined, fallback = '0'): string {
  const t = raw?.trim()
  if (!t) return fallback
  return formatCalcDisplay(t) ?? t
}

/** Ozon: полная логистика на краях = магистраль min|max + последняя миля + КГТ. */
function ozonLogisticsParts(values: Record<string, string>) {
  const min = parseNum(values.logisticsMin)
  const max = parseNum(values.logisticsMax)
  const avg = (min > 0 || max > 0) ? (min + max) / 2 : parseNum(values.logisticsAvg)
  let lastMile = parseNum(values.lastMile)
  let kgtBonus = parseNum(values.kgtBonus)
  const total =
    values.logistics?.trim()
      ? parseNum(values.logistics)
      : avg + lastMile + kgtBonus
  // Если колонки не пришли в values, восстановить из итого − среднее (как в своде).
  if (!values.lastMile?.trim() && values.logistics?.trim()) {
    lastMile = Math.max(0, total - avg - kgtBonus)
  }
  if (!values.kgtBonus?.trim() && values.logistics?.trim() && values.lastMile?.trim()) {
    kgtBonus = Math.max(0, total - avg - lastMile)
  }
  return {
    min,
    max,
    avg,
    lastMile,
    kgtBonus,
    total,
    logLo: min + lastMile + kgtBonus,
    logHi: max + lastMile + kgtBonus,
  }
}

function fmtLogisticsNum(n: number): string {
  return n.toLocaleString('ru-RU', { maximumFractionDigits: 2 })
}

function profitRates(snap: CalculatorSnapshot, marketplace: string | undefined): ProfitRates & { formula: string } {
  const isOzon =
    (marketplace ?? '').toLowerCase().includes('ozon') &&
    (marketplace ?? '').toLowerCase() !== 'both'
  return {
    taxPercent: isOzon ? snap.ozonTaxPercent : snap.taxPercent,
    acquiringPercent: isOzon ? snap.ozonAcquiringPercent : snap.acquiringPercent,
    advPercent: isOzon ? snap.ozonAdvPercent : snap.advPercent,
    extraExpensesPercent: isOzon ? snap.ozonExtraExpensesPercent : snap.extraExpensesPercent,
    formula:
      (isOzon ? snap.ozonProfitFormula : snap.profitFormula)?.trim() ||
      DEFAULT_PROFIT_FORMULA_PRETTY ||
      DEFAULT_PROFIT_FORMULA,
  }
}

function dynamicsTip(curKey: string, prevKey: string, values: Record<string, string>): CalcTip | undefined {
  const cur = values[curKey]
  const prev = values[prevKey]
  if (!cur?.trim() && !prev?.trim()) return undefined
  const c = parseNum(cur)
  const p = parseNum(prev)
  const lines = [
    t("calculator.tips.currentPeriod", { value: disp(cur) }),
    t("calculator.tips.prevPeriod", { value: disp(prev) }),
  ]
  if (Math.abs(p) < 1e-9) {
    lines.push(t("calculator.tips.totalPrevZero"))
  } else {
    const pct = ((c - p) / p) * 100
    const stored = values[`${curKey}Delta`]
    const shown = stored?.trim() && closeEnough(parseNum(stored), pct)
      ? (formatCalcDisplay(stored) ?? fmtSignedPct(pct))
      : fmtSignedPct(pct)
    lines.push(t("calculator.tips.total", { value: shown }))
  }
  return {
    title: t("calculator.tips.deltaTitle"),
    lines,
  }
}

export type CalcTipsMap = {
  orders?: CalcTip
  ordersPrev?: CalcTip
  clicksDelta?: CalcTip
  cartDelta?: CalcTip
  ordersDelta?: CalcTip
  clickToCart?: CalcTip
  clickToOrder?: CalcTip
  cartToOrder?: CalcTip
  volume?: CalcTip
  logistics?: CalcTip
  orderCost?: CalcTip
  profitPlanned?: CalcTip
  profitFact?: CalcTip
}

function conversionTip(
  title: string,
  numLabel: string,
  denLabel: string,
  numKey: string,
  denKey: string,
  outKey: string,
  values: Record<string, string>,
): CalcTip | undefined {
  if (!values[outKey]?.trim() && !values[numKey]?.trim() && !values[denKey]?.trim()) return undefined
  const total = conversionShown(values, numKey, denKey, outKey)
  return {
    title,
    lines: [
      `${numLabel}: ${disp(values[numKey])}`,
      `${denLabel}: ${disp(values[denKey])}`,
      t("calculator.tips.total", { value: disp(total) }),
    ],
  }
}

export function buildCalcTips(
  values: Record<string, string>,
  snap: CalculatorSnapshot,
  marketplace: string | undefined,
): CalcTipsMap {
  const tips: CalcTipsMap = {}
  const rates = profitRates(snap, marketplace)
  const isOzon =
    (marketplace ?? '').toLowerCase().includes('ozon') &&
    (marketplace ?? '').toLowerCase() !== 'both'

  if (isOzon) {
    if (values.orders?.trim() || values.ordersPrev?.trim()) {
      tips.orders = {
        title: t("calculator.tips.ordersOzon2w"),
        lines: [
          t("calculator.tips.ozonOrdersSumLine"),
          t("calculator.tips.ozonOrdersNoCancelled"),
          t("calculator.tips.total", { value: disp(values.orders) }),
        ],
      }
      tips.ordersPrev = {
        title: t("calculator.tips.ordersOzon2wPrev"),
        lines: [
          t("calculator.tips.ozonOrdersSumLine"),
          t("calculator.tips.ozonOrdersNoCancelled"),
          t("calculator.tips.total", { value: disp(values.ordersPrev) }),
        ],
      }
    }
  } else {
    const cur = wbOrderParts(values.orders, values.cancels, values.ordersGross)
    if (values.ordersGross?.trim() || values.cancels?.trim() || values.orders?.trim()) {
      tips.orders = {
        title: t("calculator.tips.ordersMinusCancels"),
        lines: [
          t("calculator.tips.grossOrders", { value: fmtNum(cur.gross) }),
          t("calculator.tips.cancels", { value: fmtNum(cur.cancels) }),
          t("calculator.tips.total", { value: disp(cur.totalText) }),
        ],
      }
    }

    const prev = wbOrderParts(values.ordersPrev, values.cancelsPrev, values.ordersPrevGross)
    if (values.ordersPrevGross?.trim() || values.cancelsPrev?.trim() || values.ordersPrev?.trim()) {
      tips.ordersPrev = {
        title: t("calculator.tips.ordersMinusCancelsPrev"),
        lines: [
          t("calculator.tips.grossOrders", { value: fmtNum(prev.gross) }),
          t("calculator.tips.cancels", { value: fmtNum(prev.cancels) }),
          t("calculator.tips.total", { value: disp(prev.totalText) }),
        ],
      }
    }
  }

  const tipValues = isOzon
    ? values
    : {
        ...values,
        orders:
          wbOrderParts(values.orders, values.cancels, values.ordersGross).totalText ||
          values.orders ||
          '',
        ordersPrev:
          wbOrderParts(values.ordersPrev, values.cancelsPrev, values.ordersPrevGross).totalText ||
          values.ordersPrev ||
          '',
      }

  tips.clicksDelta = dynamicsTip('clicks', 'clicksPrev', values)
  tips.cartDelta = dynamicsTip('cart', 'cartPrev', values)
  tips.ordersDelta = dynamicsTip('orders', 'ordersPrev', tipValues)
  tips.clickToCart = conversionTip(
    t("calculator.tips.cartOverClicks"),
    t("calculator.card.cart"),
    t("calculator.card.clicks"),
    'cart',
    'clicks',
    'clickToCart',
    values,
  )
  tips.clickToOrder = conversionTip(
    t("calculator.tips.ordersOverClicks"),
    t("calculator.card.orders"),
    t("calculator.card.clicks"),
    'orders',
    'clicks',
    'clickToOrder',
    tipValues,
  )
  tips.cartToOrder = conversionTip(
    t("calculator.tips.ordersOverCart"),
    t("calculator.card.orders"),
    t("calculator.card.cart"),
    'orders',
    'cart',
    'cartToOrder',
    tipValues,
  )

  const l = parseNum(values.dimLength)
  const w = parseNum(values.dimWidth)
  const h = parseNum(values.dimHeight)
  if (l > 0 && w > 0 && h > 0) {
    tips.volume = {
      title: t("calculator.tips.volumeTitle"),
      lines: [
        t("calculator.tips.dimLine", { label: t("calculator.card.length"), value: disp(values.dimLength) }),
        t("calculator.tips.dimLine", { label: t("calculator.card.width"), value: disp(values.dimWidth) }),
        t("calculator.tips.dimLine", { label: t("calculator.card.height"), value: disp(values.dimHeight) }),
        t("calculator.tips.total", { value: `${disp(volumeShown(values))} ${t("calculator.card.unitL")}` }),
      ],
    }
  }

  if (isOzon && (values.logistics?.trim() || values.logisticsMin?.trim() || values.logisticsMax?.trim()
    || values.lastMile?.trim() || values.kgtBonus?.trim())) {
    const p = ozonLogisticsParts(values)
    tips.logistics = {
      layout: 'formula',
      title: t("calculator.tips.logisticsOzonTitle"),
      lines: [
        t("calculator.tips.logisticsLineMin", { value: fmtLogisticsNum(p.min) }),
        t("calculator.tips.logisticsLineMax", { value: fmtLogisticsNum(p.max) }),
        t("calculator.tips.logisticsLineAvg", { value: fmtLogisticsNum(p.avg) }),
        t("calculator.tips.logisticsLineLastMile", { value: fmtLogisticsNum(p.lastMile) }),
        t("calculator.tips.logisticsLineKgt", { value: fmtLogisticsNum(p.kgtBonus) }),
        t("calculator.tips.logisticsLineTotal", {
          value: disp(textMatching(p.avg + p.lastMile + p.kgtBonus, values.logistics)),
        }),
      ],
    }
  } else if (values.logistics?.trim() || values.logisticsBase?.trim()) {
    const lines: string[] = []
    const part = (key: string, raw: string | undefined) => {
      const v = raw?.trim()
      if (v && v !== '—' && v !== '-') lines.push(t(key, { value: disp(v) }))
    }
    part("calculator.tips.sgt", values.isSgt)
    part("calculator.tips.volumeV", values.volumeLiters)
    part("calculator.tips.warehouse", values.logisticsWarehouse)
    part("calculator.tips.fbsBase1l", values.logisticsBase)
    part("calculator.tips.extraLiter", values.logisticsLiter)
    if (values.logistics?.trim()) lines.push(t("calculator.tips.total", { value: disp(values.logistics) }))
    tips.logistics = {
      title: t("calculator.tips.logisticsWbTitle"),
      lines,
    }
  }

  const ad = parseNum(values.adExpenses)
  const clean = isOzon
    ? { total: parseNum(values.orders), totalText: values.orders ?? '' }
    : wbOrderParts(values.orders, values.cancels, values.ordersGross)
  const ordersNet = clean.total
  if (values.adExpenses?.trim() || values.orders?.trim() || values.orderCost?.trim()) {
    const rule =
      ordersNet > 0
        ? t("calculator.tips.orderCostFormula")
        : ad > 0
          ? t("calculator.tips.orderCostZeroExpenses")
          : t("calculator.tips.orderCostZero")
    tips.orderCost = {
      title: rule,
      lines: [
        t("calculator.tips.adExpensesLine", { value: disp(values.adExpenses) }),
        t("calculator.tips.cleanOrdersLine", { value: disp(clean.totalText) }),
        t("calculator.tips.total", { value: disp(orderCostShown(values, ordersNet)) }),
      ],
    }
  }

  const planned = evaluateProfit(rates.formula, values, rates)
  if (planned != null) {
    const price = parseNum(values.priceAfterDiscount || values.price || values.ozPrice)
    const numericExpr = formatFormulaWithNumbers(rates.formula, values, rates)
    const plannedLines = [`${numericExpr} = ${formatProfitRu(planned) ?? emDashFromCommon()}`]
    if (isOzon && (values.logisticsMin?.trim() || values.logisticsMax?.trim())) {
      const parts = ozonLogisticsParts(values)
      const withMin = { ...values, logistics: String(parts.logLo) }
      const withMax = { ...values, logistics: String(parts.logHi) }
      const pMin = evaluateProfit(rates.formula, withMin, rates)
      const pMax = evaluateProfit(rates.formula, withMax, rates)
      if (pMin != null && pMax != null) {
        const lo = Math.min(pMin, pMax)
        const hi = Math.max(pMin, pMax)
        plannedLines.push(
          t("calculator.tips.profitRange", {
            lo: formatProfitRu(lo) ?? emDashFromCommon(),
            hi: formatProfitRu(hi) ?? emDashFromCommon(),
          }),
        )
      }
    }
    tips.profitPlanned = {
      layout: 'formula',
      title: formatFormulaPretty(rates.formula, { brackets: false }),
      lines: plannedLines,
    }

    const fact = evaluateProfitFact(planned, values, rates.advPercent)
    if (fact != null) {
      const em = emDashFromCommon()
      const factCost = computeOrderCost(ad, parseNum(values.orders))
      const factLines = [
        `${formatProfitRu(planned) ?? em} + ${fmt(price)} × ${fmtPct(rates.advPercent)} − ${fmt(factCost)} = ${formatProfitRu(fact) ?? em}`,
      ]
      if (isOzon && (values.logisticsMin?.trim() || values.logisticsMax?.trim())) {
        const parts = ozonLogisticsParts(values)
        const withMin = { ...values, logistics: String(parts.logLo) }
        const withMax = { ...values, logistics: String(parts.logHi) }
        const plMin = evaluateProfit(rates.formula, withMin, rates)
        const plMax = evaluateProfit(rates.formula, withMax, rates)
        const fMin = evaluateProfitFact(plMin, values, rates.advPercent)
        const fMax = evaluateProfitFact(plMax, values, rates.advPercent)
        if (fMin != null && fMax != null) {
          const lo = Math.min(fMin, fMax)
          const hi = Math.max(fMin, fMax)
          factLines.push(
            t("calculator.tips.profitRange", {
              lo: formatProfitRu(lo) ?? em,
              hi: formatProfitRu(hi) ?? em,
            }),
          )
        }
      }
      tips.profitFact = {
        layout: 'formula',
        title: t("calculator.tips.profitFactTitle"),
        lines: factLines,
      }
    }
  }
  return tips
}
function emDashFromCommon(): string {
  return t("common.emDash")
}
