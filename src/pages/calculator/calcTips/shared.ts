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

import type { CalculatorSnapshot } from '../calculatorTypes'
import {
  DEFAULT_PROFIT_FORMULA,
  computeOrderCost,
  type ProfitRates,
} from '../evaluateProfit'
import { formatCalcDisplay } from '../formatCalcNumber'
import { t } from "../../../i18n";
import {
  DEFAULT_PROFIT_FORMULA_PRETTY,
  normalizeFormula,
  spaceFormulaOperators,
} from '../formulaTokens'

export type CalcTip = {
  title: string
  lines: string[]
  /** `formula`: title = tokens, lines[0] = numbers (compact 2-line tip). */
  layout?: 'list' | 'formula'
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

export function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

export function parseNum(raw: string | undefined): number {
  if (!raw?.trim()) return 0
  const s = raw.trim().replace(/%/g, '').replace(/\s/g, '').replace(',', '.')
  const n = Number(s)
  return Number.isFinite(n) ? n : 0
}

export function fmt(n: number): string {
  return n.toLocaleString('ru-RU', { maximumFractionDigits: 2 })
}

export function fmtPct(n: number): string {
  return `${fmt(n)}%`
}

/** Same rounding the card uses, so a tip total can equal the cell. */
export function fmtNum(n: number): string {
  const raw = n
    .toFixed(2)
    .replace(/0+$/, '')
    .replace(/\.$/, '')
    .replace('.', ',')
  return formatCalcDisplay(raw) ?? raw
}

export function closeEnough(stored: number, computed: number): boolean {
  return Math.abs(stored - computed) <= 0.05
}

/** Prefer the cell text when it already equals the formula. */
export function textMatching(computed: number, storedRaw: string | undefined, suffix = ''): string {
  const stored = storedRaw?.trim() ?? ''
  if (stored && closeEnough(parseNum(stored), computed)) return stored
  return `${fmtNum(computed)}${suffix}`
}

export function fmtSignedPct(pct: number): string {
  const raw = `${pct >= 0 ? '+' : ''}${pct.toFixed(1).replace('.', ',')}%`
  return formatCalcDisplay(raw) ?? raw
}

export function disp(raw: string | undefined, fallback = '0'): string {
  const text = raw?.trim()
  if (!text) return fallback
  return formatCalcDisplay(text) ?? text
}

export function emDashFromCommon(): string {
  return t("common.emDash")
}

export function isOzonMarketplace(marketplace: string | undefined): boolean {
  const m = (marketplace ?? '').toLowerCase()
  return m.includes('ozon') && m !== 'both'
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

/** Ozon: полная логистика на краях = магистраль min|max + последняя миля + КГТ. */
export function ozonLogisticsParts(values: Record<string, string>) {
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

export function fmtLogisticsNum(n: number): string {
  return n.toLocaleString('ru-RU', { maximumFractionDigits: 2 })
}

export function profitRates(
  snap: CalculatorSnapshot,
  marketplace: string | undefined,
): ProfitRates & { formula: string } {
  const isOzon = isOzonMarketplace(marketplace)
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

/** Tip display: money as amounts; commissions/settings rates as percent (6%, not 0,06). */
export function formatFormulaWithNumbers(
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
