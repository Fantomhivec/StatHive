import { useCallback, useState } from "react";

/** Shared busy/error/ok for page actions — no UX change, less copy-paste. */
export function useBusyState() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);

  const withBusy = useCallback(async (fn: () => Promise<void>) => {
    setError(null);
    setOk(null);
    setBusy(true);
    try {
      await fn();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  return { busy, setBusy, error, setError, ok, setOk, withBusy };
}
