/** Сброс кэшей карточки/прогрева после обновления таблиц. */

const SESSION_PREFIX = "calc:lastLookup:";
export const CALC_LOOKUP_INVALIDATE_EVENT = "calc-lookup-invalidate";

type WarmFlag = { started: boolean };

/** Общий флаг с useCalculatorSnapshot (один модульный прогрев на сессию). */
const warmFlag: WarmFlag = { started: false };

export function isLookupWarmStarted(): boolean {
  return warmFlag.started;
}

export function markLookupWarmStarted(): void {
  warmFlag.started = true;
}

export function resetLookupWarmFlag(): void {
  warmFlag.started = false;
}

export function clearCalcLookupSessions(): void {
  try {
    const keys: string[] = [];
    for (let i = 0; i < sessionStorage.length; i++) {
      const k = sessionStorage.key(i);
      if (k?.startsWith(SESSION_PREFIX)) keys.push(k);
    }
    for (const k of keys) sessionStorage.removeItem(k);
  } catch {
    // private mode / quota
  }
}

/** После create/refresh таблиц: сессия карточки + повторный прогрев RAM. */
export function invalidateCalculatorLookupCaches(): void {
  clearCalcLookupSessions();
  resetLookupWarmFlag();
  try {
    window.dispatchEvent(new CustomEvent(CALC_LOOKUP_INVALIDATE_EVENT));
  } catch {
    // ignore
  }
}
