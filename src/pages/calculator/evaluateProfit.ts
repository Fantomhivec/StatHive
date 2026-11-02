/** Mirrors CalculatorCardReader.EvaluateProfit (default formula + token replace). */

import { DEFAULT_PROFIT_FORMULA_PRETTY, normalizeFormula } from './formulaTokens'

export const PROFIT_EDIT_KEYS = [
  'priceAfterDiscount',
  'purchase',
  'commissionFbo',
  'commissionFbs',
  'logistics',
  'adExpenses',
] as const

export type ProfitEditKey = (typeof PROFIT_EDIT_KEYS)[number]

export type ProfitRates = {
  taxPercent: number
  acquiringPercent: number
  advPercent: number
  extraExpensesPercent: number
}

/** Canonical default (bracket form). evaluateProfit normalizes before compute. */
export const DEFAULT_PROFIT_FORMULA = DEFAULT_PROFIT_FORMULA_PRETTY

function parseNumber(raw: string | undefined): number {
  if (!raw?.trim()) return 0
  const s = raw.trim().replace(/%/g, '').replace(/\s/g, '').replace(',', '.')
  const n = Number(s)
  return Number.isFinite(n) ? n : 0
}

function metric(values: Record<string, string>, ...keys: string[]): number {
  for (const k of keys) {
    if (values[k]?.trim()) return parseNumber(values[k])
  }
  return 0
}

function replaceToken(expr: string, token: string, value: number): string {
  return expr.replace(new RegExp(token.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi'), String(value))
}

export function formatProfitRu(n: number | null | undefined): string | null {
  if (n == null || !Number.isFinite(n)) return null
  return n.toLocaleString('ru-RU', { maximumFractionDigits: 2, minimumFractionDigits: 0 })
}

/** Цена продажи: WB после скидки / Ozon после акций (один ключ в карточке). */
export function sellPrice(values: Record<string, string>): number {
  return metric(values, 'priceAfterDiscount', 'price', 'ozPrice')
}

/** Прибыль в % от цены продажи. */
export function profitPercentOfSellPrice(
  profit: number | null | undefined,
  values: Record<string, string>,
): number | null {
  if (profit == null || !Number.isFinite(profit)) return null
  const price = sellPrice(values)
  if (!(price > 0)) return null
  return (profit / price) * 100
}

export function formatProfitPctRu(n: number | null | undefined): string | null {
  if (n == null || !Number.isFinite(n)) return null
  return `${n.toLocaleString('ru-RU', { maximumFractionDigits: 2, minimumFractionDigits: 0 })} %`
}

export function evaluateProfit(
  formula: string | undefined | null,
  values: Record<string, string>,
  rates: ProfitRates,
): number | null {
  let expr = normalizeFormula(formula?.trim() || DEFAULT_PROFIT_FORMULA)

  expr = replaceToken(expr, 'налог', rates.taxPercent / 100)
  expr = replaceToken(expr, 'эквайринг', rates.acquiringPercent / 100)
  expr = replaceToken(expr, 'реклама', rates.advPercent / 100)
  expr = replaceToken(expr, 'доп_расходы', rates.extraExpensesPercent / 100)
  expr = replaceToken(expr, 'цена_после_скидки', metric(values, 'priceAfterDiscount', 'price', 'ozPrice'))
  expr = replaceToken(expr, 'комиссия_fbo', metric(values, 'commissionFbo') / 100)
  expr = replaceToken(expr, 'комиссия_fbs', metric(values, 'commissionFbs') / 100)
  expr = replaceToken(expr, 'закуп', metric(values, 'purchase'))
  expr = replaceToken(expr, 'логистика', metric(values, 'logistics'))

  const cleaned = expr.replace(/[^0-9+\-*/().,\s]/g, '').replace(/,/g, '.').trim()
  if (!cleaned) return null
  try {
    // Expression is sanitized to digits and arithmetic ops only (same idea as DataTable.Compute).
    // eslint-disable-next-line no-new-func -- local calculator, not user HTML
    const result = Function(`"use strict"; return (${cleaned})`)() as unknown
    return typeof result === 'number' && Number.isFinite(result) ? result : null
  } catch {
    return null
  }
}

/** Mirror CalculatorCardReader.ComputeOrderCost: A/O; if O=0 → A if A>0 else 0. */
export function computeOrderCost(adExpenses: number, orders: number): number {
  if (orders > 0) return adExpenses / orders
  return adExpenses > 0 ? adExpenses : 0
}

/**
 * Прибыль факт = profit + priceAfterDiscount × (advPercent/100) − orderCost.
 * null only if profit is null; orderCost=0 still yields a number.
 */
export function evaluateProfitFact(
  profit: number | null | undefined,
  values: Record<string, string>,
  advPercent: number,
): number | null {
  if (profit == null || !Number.isFinite(profit)) return null
  const price = sellPrice(values)
  const adExpenses = metric(values, 'adExpenses')
  const orders = metric(values, 'orders')
  const orderCost = computeOrderCost(adExpenses, orders)
  return profit + price * (advPercent / 100) - orderCost
}

