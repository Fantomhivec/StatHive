import { t } from "../../i18n";

/** Registry of profit-formula tokens: internal tag ↔ display label ↔ color. */

export type FormulaTokenDef = {
  tag: string
  display: string
  /** CSS color for chip background (with alpha via color-mix in CSS). */
  color: string
  group: 'settings' | 'content'
}

/** Longest tags/displays first for replace safety. */
export const FORMULA_TOKENS: FormulaTokenDef[] = [
  { tag: 'цена_после_скидки', display: t("calculator.card.priceAfterDiscount"), color: 'var(--formula-chip-price)', group: 'content' },
  { tag: 'комиссия_fbs', display: t("calculator.card.commissionFbs"), color: 'var(--formula-chip-fbs)', group: 'content' },
  { tag: 'комиссия_fbo', display: t("calculator.card.commissionFbo"), color: 'var(--formula-chip-fbo)', group: 'content' },
  { tag: 'доп_расходы', display: t("calculator.formula.tokens.extraExpenses"), color: 'var(--formula-chip-extra)', group: 'settings' },
  { tag: 'эквайринг', display: t("calculator.formula.tokens.acquiring"), color: 'var(--formula-chip-acquiring)', group: 'settings' },
  { tag: 'реклама', display: t("calculator.formula.tokens.ads"), color: 'var(--formula-chip-ads)', group: 'settings' },
  { tag: 'логистика', display: t("calculator.card.logistics"), color: 'var(--formula-chip-logistics)', group: 'content' },
  { tag: 'налог', display: t("calculator.formula.tokens.tax"), color: 'var(--formula-chip-tax)', group: 'settings' },
  { tag: 'закуп', display: t("calculator.card.purchase"), color: 'var(--formula-chip-purchase)', group: 'content' },
]

export const SETTINGS_TOKENS = FORMULA_TOKENS.filter((t) => t.group === 'settings').map((t) => ({
  label: t.display,
  tag: t.tag,
  display: t.display,
  color: t.color,
  insert: t.display,
}))

export const CONTENT_TOKENS = FORMULA_TOKENS.filter((t) => t.group === 'content').map((t) => ({
  label: t.display,
  tag: t.tag,
  display: t.display,
  color: t.color,
  insert: t.display,
}))

export const DEFAULT_PROFIT_FORMULA_PRETTY =
  t("calculator.formula.exampleHuman")

/**
 * Spaces around * + / = - so tokens/numbers do not run together.
 * Keeps ru-RU grouped numbers (e.g. `7 459,8`); no space right after `(` / before `)`.
 */
export function spaceFormulaOperators(expr: string): string {
  if (!expr?.trim()) return expr ?? ''
  const nums: string[] = []
  let s = expr.replace(/\d{1,3}(?:[\s\u00a0]\d{3})+(?:,\d+)?/g, (m) => {
    nums.push(m)
    return `\u0001${nums.length - 1}\u0001`
  })
  s = s.replace(/\s*([*+/=])\s*/g, ' $1 ')
  s = s.replace(/\s*-\s*/g, ' - ')
  s = s.replace(/\(\s+/g, '(')
  s = s.replace(/\s+\)/g, ')')
  s = s.replace(/=\s-\s+/g, '= -')
  s = s.replace(/\s+/g, ' ').trim()
  s = s.replace(/\u0001(\d+)\u0001/g, (_, i) => nums[Number(i)] ?? '')
  return s
}

/** Replace [Display] or bare Display → tag; leave legacy snake_case as-is. */
export function normalizeFormula(expr: string): string {
  let s = expr
  const byDisplayLen = [...FORMULA_TOKENS].sort((a, b) => b.display.length - a.display.length)
  for (const t of byDisplayLen) {
    // Optional brackets around display (legacy).
    const re = new RegExp(`\\[?${escapeRegExp(t.display)}\\]?`, 'gi')
    s = s.replace(re, t.tag)
  }
  return s
}

/** Human-readable formula for tips / UI (display names, no brackets). */
export function formatFormulaPretty(expr: string | undefined | null, _opts?: { brackets?: boolean }): string {
  let s = (expr ?? '').trim()
  if (!s) s = DEFAULT_PROFIT_FORMULA_PRETTY

  s = normalizeFormula(s)
  const byTagLen = [...FORMULA_TOKENS].sort((a, b) => b.tag.length - a.tag.length)
  for (const t of byTagLen) {
    const re = new RegExp(escapeRegExp(t.tag), 'gi')
    s = s.replace(re, t.display)
  }
  return spaceFormulaOperators(s)
}

export type FormulaSegment =
  | { kind: 'token'; display: string; color: string; tag: string }
  | { kind: 'text'; text: string }

/** Parse formula into colored token chips + plain text (ops/numbers). */
export function parseFormulaSegments(expr: string): FormulaSegment[] {
  const raw = expr || ''
  if (!raw.trim()) return []

  // Alternation: [Display] | Display | tag (longest first)
  const parts: string[] = []
  const sorted = [...FORMULA_TOKENS].sort(
    (a, b) => Math.max(b.display.length, b.tag.length) - Math.max(a.display.length, a.tag.length),
  )
  for (const t of sorted) {
    parts.push(`\\[${escapeRegExp(t.display)}\\]`)
    parts.push(escapeRegExp(t.display))
    parts.push(escapeRegExp(t.tag))
  }
  const combined = new RegExp(parts.join('|'), 'gi')

  const segments: FormulaSegment[] = []
  let last = 0
  let m: RegExpExecArray | null
  const source = raw
  while ((m = combined.exec(source)) != null) {
    if (m.index > last) {
      segments.push({ kind: 'text', text: source.slice(last, m.index) })
    }
    const matched = m[0]
    const bare = matched.replace(/^\[|\]$/g, '')
    const token =
      FORMULA_TOKENS.find(
        (t) =>
          bare.toLowerCase() === t.display.toLowerCase() ||
          matched.toLowerCase() === t.tag.toLowerCase(),
      ) ?? FORMULA_TOKENS[0]
    segments.push({ kind: 'token', display: token.display, color: token.color, tag: token.tag })
    last = m.index + matched.length
  }
  if (last < source.length) {
    segments.push({ kind: 'text', text: source.slice(last) })
  }
  return segments
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}
