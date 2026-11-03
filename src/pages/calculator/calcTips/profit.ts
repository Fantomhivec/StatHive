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
  computeOrderCost,
  evaluateProfit,
  evaluateProfitFact,
  formatProfitRu,
} from '../evaluateProfit'
import { formatFormulaPretty } from '../formulaTokens'
import { t } from "../../../i18n";
import type { CalcTipsMap } from './shared'
import {
  emDashFromCommon,
  fmt,
  fmtPct,
  formatFormulaWithNumbers,
  isOzonMarketplace,
  ozonLogisticsParts,
  parseNum,
  profitRates,
} from './shared'

/** Planned / fact profit formula tips. */
export function fillProfitTips(
  tips: CalcTipsMap,
  values: Record<string, string>,
  snap: CalculatorSnapshot,
  marketplace: string | undefined,
): void {
  const rates = profitRates(snap, marketplace)
  const isOzon = isOzonMarketplace(marketplace)
  const planned = evaluateProfit(rates.formula, values, rates)
  if (planned == null) return

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
  if (fact == null) return

  const em = emDashFromCommon()
  const ad = parseNum(values.adExpenses)
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
