/** Thin event listen — shell/auth without wbCalculator. */
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Подписка на host-события (updates.progress, wbCalculator.runLog и т.п.). */
export function onEvent<T = unknown>(
  event: string,
  handler: (payload: T) => void,
): () => void {
  let un: UnlistenFn | undefined;
  let cancelled = false;
  void listen<T>(event, (e) => {
    if (!cancelled) handler(e.payload);
  }).then((fn) => {
    if (cancelled) {
      fn();
      return;
    }
    un = fn;
  });
  return () => {
    cancelled = true;
    un?.();
  };
}
