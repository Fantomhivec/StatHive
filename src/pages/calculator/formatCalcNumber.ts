/** Format calculator card numbers: ru-RU thousand separators. */

function parseLooseNumber(raw: string): number | null {
  const s = raw.trim().replace(/\s/g, '').replace(/\u00a0/g, '').replace('%', '').replace(',', '.')
  if (!s || !/^-?\d+(\.\d+)?$/.test(s)) return null
  const n = Number(s)
  return Number.isFinite(n) ? n : null
}

/**
 * Format a cell value for display. Non-numeric / mixed labels left as-is
 * (except leading number + optional % / delta sign).
 */
export function formatCalcDisplay(
  raw: string | undefined | null,
  opts?: { maxFractionDigits?: number },
): string | undefined {
  if (raw == null) return undefined
  const trimmed = String(raw).trim()
  if (!trimmed) return undefined

  const maxFrac = opts?.maxFractionDigits ?? 2

  // Pure number (optional leading +/−)
  const pure = parseLooseNumber(trimmed)
  if (pure != null && !trimmed.includes('%')) {
    return pure.toLocaleString('ru-RU', {
      maximumFractionDigits: maxFrac,
      minimumFractionDigits: 0,
    })
  }

  // +3,8% / -12% / 80%
  const pct = trimmed.match(/^([+\-]?\d+(?:[.,]\d+)?)\s*%$/)
  if (pct) {
    const n = parseLooseNumber(pct[1])
    if (n == null) return trimmed
    const hadPlus = pct[1].trimStart().startsWith('+')
    const body = n.toLocaleString('ru-RU', {
      maximumFractionDigits: maxFrac,
      minimumFractionDigits: 0,
    })
    if (hadPlus && n >= 0 && !body.startsWith('+')) return `+${body}%`
    return `${body}%`
  }

  return trimmed
}

/** Strip thousand separators for editable inputs / storage. */
export function unformatCalcNumber(raw: string): string {
  return raw.replace(/\s/g, '').replace(/\u00a0/g, '')
}

export function formatVolumeLiters(raw: string | number | undefined | null): string | undefined {
  if (raw == null || raw === '') return undefined
  const n = typeof raw === 'number' ? raw : parseLooseNumber(String(raw))
  if (n == null) return undefined
  return n.toLocaleString('ru-RU', {
    maximumFractionDigits: 2,
    minimumFractionDigits: 0,
  })
}
