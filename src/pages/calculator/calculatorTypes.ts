import { t } from "../../i18n";

export type TableKey = { key: string; label: string; marketplace?: string }

export type TriggerRow = {
  tableKey: string
  displayName: string
  /** None | Once | Daily | Weekly | Interval */
  mode: string
  time: string
  intervalHours: number
  intervalMinutes: number
  nextExecution?: string
  scheduledDateTime?: string
  weeklyDay?: string
  weeklyTime?: string
}

export type ColumnBinding = {
  metricKey: string
  spreadsheetUrl: string
  sheetName: string
  columnHeader: string
  articleColumnHeader: string
}

/** gt = строго больше; gte = больше либо равно. */
export type KgtCmp = 'gt' | 'gte'

export type WbKgtRules = {
  maxOneSideCm: number
  oneSideCmp: KgtCmp
  maxSumSidesCm: number
  sumSidesCmp: KgtCmp
  maxWeightKg: number
  weightCmp: KgtCmp
}

export type OzKgtRules = {
  maxSumSides: number
  sumSidesCmp: KgtCmp
  maxWeight: number
  weightCmp: KgtCmp
  maxVolume: number
  volumeCmp: KgtCmp
  bonus: number
}

export const KGT_CMP_OPTIONS: { value: KgtCmp; label: string }[] = [
  { value: 'gt', label: t("calculator.kgtCmp.gt") },
  { value: 'gte', label: t("calculator.kgtCmp.gte") },
]

/** Как прежние константы СГТ в CardReader / SummaryBuilder (≥). */
export const DEFAULT_WB_KGT: WbKgtRules = {
  maxOneSideCm: 120,
  oneSideCmp: 'gte',
  maxSumSidesCm: 200,
  sumSidesCmp: 'gte',
  maxWeightKg: 25,
  weightCmp: 'gte',
}

/** Как прежние константы КГТ Ozon (вес>35, сумма>200, объём>500, +280). */
export const DEFAULT_OZ_KGT: OzKgtRules = {
  maxSumSides: 200,
  sumSidesCmp: 'gt',
  maxWeight: 35,
  weightCmp: 'gt',
  maxVolume: 500,
  volumeCmp: 'gt',
  bonus: 280,
}

export function normalizeKgtCmp(raw: string | undefined | null, fallback: KgtCmp): KgtCmp {
  return raw === 'gt' || raw === 'gte' ? raw : fallback
}

/** Подтянуть cmp из snapshot (старые профили без полей → defaults). */
export function coalesceWbKgt(raw: Partial<WbKgtRules> | null | undefined): WbKgtRules {
  const d = DEFAULT_WB_KGT
  return {
    maxOneSideCm: raw?.maxOneSideCm ?? d.maxOneSideCm,
    oneSideCmp: normalizeKgtCmp(raw?.oneSideCmp, d.oneSideCmp),
    maxSumSidesCm: raw?.maxSumSidesCm ?? d.maxSumSidesCm,
    sumSidesCmp: normalizeKgtCmp(raw?.sumSidesCmp, d.sumSidesCmp),
    maxWeightKg: raw?.maxWeightKg ?? d.maxWeightKg,
    weightCmp: normalizeKgtCmp(raw?.weightCmp, d.weightCmp),
  }
}

export function coalesceOzKgt(raw: Partial<OzKgtRules> | null | undefined): OzKgtRules {
  const d = DEFAULT_OZ_KGT
  return {
    maxSumSides: raw?.maxSumSides ?? d.maxSumSides,
    sumSidesCmp: normalizeKgtCmp(raw?.sumSidesCmp, d.sumSidesCmp),
    maxWeight: raw?.maxWeight ?? d.maxWeight,
    weightCmp: normalizeKgtCmp(raw?.weightCmp, d.weightCmp),
    maxVolume: raw?.maxVolume ?? d.maxVolume,
    volumeCmp: normalizeKgtCmp(raw?.volumeCmp, d.volumeCmp),
    bonus: raw?.bonus ?? d.bonus,
  }
}

export type CalculatorSnapshot = {
  bundleId: string
  calculatorName: string
  /** Фильтр блоков настроек + кабинетов: wb | ozon | both. */
  marketplace: string
  /** Активные таблицы по выбранным ключам: wb | ozon | both | none. */
  tablesMarketplace?: string
  triggersEnabled: boolean
  selectedCabinets: string[]
  availableCabinets: string[]
  /** Выбранные кабинеты без ключа в .env (после импорта конфига). */
  missingKeyCabinets?: string[]
  /** false у участника: ключей нет, только названия кабинетов из конфигурации. */
  usesLocalKeys?: boolean
  cabinetMarketplaces?: Record<string, string>
  cabinetChips?: Array<{ name: string; marketplace: string; hasKey: boolean }>
  tableLinks: Record<string, string>
  tableModes: Record<string, string>
  lastUpdated: Record<string, string>
  taxPercent: number
  acquiringPercent: number
  advPercent: number
  extraExpensesPercent: number
  ozonTaxPercent: number
  ozonAcquiringPercent: number
  ozonAdvPercent: number
  ozonExtraExpensesPercent: number
  profitFormula: string
  ozonProfitFormula: string
  wbKgt: WbKgtRules
  ozKgt: OzKgtRules
  columnBindings: ColumnBinding[]
  metrics: Array<{ key: string; label: string; marketplace: string; tableKey?: string }>
  triggers: TriggerRow[]
  tableKeys: TableKey[]
  templateIdsByTableKey: Record<string, string>
  createButtonLabel: string
}

export type CardResult = {
  ok: boolean
  message?: string
  article?: string
  marketplace?: string
  values?: Record<string, string>
  profit?: number | null
  profitFormatted?: string | null
}

export { SETTINGS_TOKENS, CONTENT_TOKENS } from './formulaTokens'
export const OPS = ['+', '-', '*', '/', '(', ')']
export const DIGITS = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']
export const MODE_OPTS = [
  { value: 'None', label: t("calculator.triggerMode.none") },
  { value: 'Once', label: t("calculator.triggerMode.once") },
  { value: 'Daily', label: t("calculator.triggerMode.daily") },
  { value: 'Weekly', label: t("calculator.triggerMode.weekly") },
  { value: 'Interval', label: t("calculator.triggerMode.interval") },
]
export const TIME_OPTS = Array.from({ length: 24 }, (_, i) => {
  const t = `${String(i).padStart(2, '0')}:00`
  return { value: t, label: t }
})
export const MARKETPLACE_OPTS = [
  { value: 'wb', label: t("calculator.marketplace.wb") },
  { value: 'ozon', label: t("calculator.marketplace.ozon") },
  { value: 'both', label: t("calculator.marketplace.both") },
]
export const TABLE_MODE_OPTS = [
  { value: 'auto', label: t("calculator.tableMode.auto") },
  { value: 'manual', label: t("calculator.tableMode.manual") },
]

/** Case-insensitive dict lookup (catalog keys like Array vs camelCase array). */
export function dictGet(dict: Record<string, string> | undefined | null, key: string): string | undefined {
  if (!dict) return undefined
  if (Object.prototype.hasOwnProperty.call(dict, key)) return dict[key]
  const lower = key.toLowerCase()
  for (const [k, v] of Object.entries(dict)) {
    if (k.toLowerCase() === lower) return v
  }
  return undefined
}
