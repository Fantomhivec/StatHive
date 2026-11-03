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

import type { CalculatorSnapshot } from './calculatorTypes'
import { fillOrdersMetricsTips } from './calcTips/ordersMetrics'
import { fillProfitTips } from './calcTips/profit'
import type { CalcTip, CalcTipsMap } from './calcTips/shared'

export type { CalcTip, CalcTipsMap }
export {
  conversionShown,
  orderCostShown,
  volumeShown,
  wbOrderParts,
} from './calcTips/shared'

export function buildCalcTips(
  values: Record<string, string>,
  snap: CalculatorSnapshot,
  marketplace: string | undefined,
): CalcTipsMap {
  const tips: CalcTipsMap = {}
  fillOrdersMetricsTips(tips, values, marketplace)
  fillProfitTips(tips, values, snap, marketplace)
  return tips
}
