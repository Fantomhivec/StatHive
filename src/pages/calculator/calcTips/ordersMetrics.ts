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

import { t } from "../../../i18n";
import { formatCalcDisplay } from '../formatCalcNumber'
import type { CalcTip, CalcTipsMap } from './shared'
import {
  closeEnough,
  conversionShown,
  disp,
  fmtLogisticsNum,
  fmtNum,
  fmtSignedPct,
  isOzonMarketplace,
  orderCostShown,
  ozonLogisticsParts,
  parseNum,
  textMatching,
  volumeShown,
  wbOrderParts,
} from './shared'

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

/** Orders, deltas, conversions, volume, logistics, order cost. */
export function fillOrdersMetricsTips(
  tips: CalcTipsMap,
  values: Record<string, string>,
  marketplace: string | undefined,
): void {
  const isOzon = isOzonMarketplace(marketplace)

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
}
