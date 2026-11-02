import { describe, expect, it } from 'vitest'
import { formatCalcDisplay, formatVolumeLiters, unformatCalcNumber } from './formatCalcNumber'

describe('formatCalcDisplay', () => {
  it('adds thousand separators (ru-RU)', () => {
    expect(formatCalcDisplay('1234567')).toMatch(/1[\s\u00a0]234[\s\u00a0]567/)
    expect(formatCalcDisplay('1500,5')).toMatch(/1[\s\u00a0]500,5/)
  })

  it('keeps non-numeric labels', () => {
    expect(formatCalcDisplay('да')).toBe('да')
    expect(formatCalcDisplay('ABC-12')).toBe('ABC-12')
  })

  it('formats percent deltas', () => {
    expect(formatCalcDisplay('+3,8%')).toBe('+3,8%')
    expect(formatCalcDisplay('1250%')).toMatch(/\+?1[\s\u00a0]250%/)
  })
})

describe('formatVolumeLiters', () => {
  it('caps at 2 fraction digits', () => {
    expect(formatVolumeLiters(1.23456)).toBe('1,23')
    expect(formatVolumeLiters('0.001')).toBe('0')
    expect(formatVolumeLiters(12.5)).toBe('12,5')
  })
})

describe('unformatCalcNumber', () => {
  it('strips spaces', () => {
    expect(unformatCalcNumber('1 234,56')).toBe('1234,56')
  })
})
