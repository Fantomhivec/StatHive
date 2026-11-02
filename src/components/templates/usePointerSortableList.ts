import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type RefObject,
} from "react";

export type PointerSortableApi = {
  activeId: string | null;
  onItemPointerDown: (e: ReactPointerEvent, id: string) => void;
};

type Options = {
  ids: string[];
  /** Переместить fromId в toIndex (индекс в текущем `ids`). */
  onReorder: (fromId: string, toIndex: number) => void;
  itemSelector?: string;
  /**
   * Если задано — drag только с этого селектора.
   * По умолчанию: вся строка (ручка не нужна).
   */
  handleSelector?: string;
  /** Зоны, с которых не начинаем pending-drag (реальные контролы). */
  ignoreSelector?: string;
  floatClassName?: string;
  bodyDraggingClass?: string;
  /** Пиксели до активации drag (click vs drag). */
  activationDistance?: number;
};

const DEFAULT_ACTIVATION = 6;
const FLIP_MS = 200;

/**
 * Pointer-sortable: вся строка + фантом + FLIP при live reorder.
 *
 * Антипаттерны (не повторять):
 * - HTML5 `draggable` / `setDragImage` в WebView2
 * - `preventDefault` на pointerdown до порога (ломает label→checkbox)
 * - обязательная «ручка» DnD — отвергнуто: тянуть за любое место строки
 * - ignore `.toggle-track` без порога — ломает drag с левой части строки
 * - без `setPointerCapture` + activation distance
 */
export function usePointerSortableList(
  listRef: RefObject<HTMLElement | null>,
  options: Options,
): PointerSortableApi {
  const {
    ids,
    onReorder,
    itemSelector = "[data-sortable-id]",
    handleSelector,
    ignoreSelector = "input, button, a",
    floatClassName = "pick-drag-float tpl-picker-drag-float",
    bodyDraggingClass = "tpl-picker-dragging",
    activationDistance = DEFAULT_ACTIVATION,
  } = options;

  const [activeId, setActiveId] = useState<string | null>(null);
  const activeIdRef = useRef<string | null>(null);
  const floatRef = useRef<HTMLDivElement | null>(null);
  const offsetRef = useRef({ x: 0, y: 0 });
  const captureElRef = useRef<HTMLElement | null>(null);
  const pointerIdRef = useRef<number | null>(null);
  const idsRef = useRef(ids);
  const onReorderRef = useRef(onReorder);
  const firstRectsRef = useRef<Map<string, DOMRect> | null>(null);
  idsRef.current = ids;
  onReorderRef.current = onReorder;

  const cleanup = useCallback(() => {
    const el = captureElRef.current;
    const pid = pointerIdRef.current;
    if (el && pid != null) {
      try {
        if (el.hasPointerCapture?.(pid)) {
          el.releasePointerCapture(pid);
        }
      } catch {
        /* ignore */
      }
    }
    captureElRef.current = null;
    pointerIdRef.current = null;
    activeIdRef.current = null;
    setActiveId(null);
    floatRef.current?.remove();
    floatRef.current = null;
    firstRectsRef.current = null;
    document.body.classList.remove(bodyDraggingClass);
  }, [bodyDraggingClass]);

  useEffect(() => () => cleanup(), [cleanup]);

  /** FLIP после React-перестановки строк во время drag (до paint). */
  useLayoutEffect(() => {
    if (!activeId) return;
    const first = firstRectsRef.current;
    const list = listRef.current;
    if (!first || !list) return;
    firstRectsRef.current = null;
    playFlip(list, itemSelector, first, FLIP_MS);
  }, [ids, activeId, itemSelector, listRef]);

  const indexFromPointer = useCallback(
    (clientY: number): number => {
      const list = listRef.current;
      if (!list) return 0;
      const els = Array.from(list.querySelectorAll<HTMLElement>(itemSelector));
      if (els.length === 0) return 0;
      for (let i = 0; i < els.length; i++) {
        const rect = els[i].getBoundingClientRect();
        if (clientY < rect.top + rect.height / 2) {
          return i;
        }
      }
      return els.length - 1;
    },
    [itemSelector, listRef],
  );

  const onItemPointerDown = useCallback(
    (e: ReactPointerEvent, id: string) => {
      if (e.button !== 0) return;
      const t = e.target as HTMLElement | null;
      if (!t) return;

      if (handleSelector) {
        if (!t.closest(handleSelector)) return;
      } else if (t.closest(ignoreSelector)) {
        return;
      }

      const item = e.currentTarget as HTMLElement;
      const startX = e.clientX;
      const startY = e.clientY;
      const pointerId = e.pointerId;
      let activated = false;

      pointerIdRef.current = pointerId;
      captureElRef.current = item;
      try {
        item.setPointerCapture?.(pointerId);
      } catch {
        /* WebView without capture */
      }

      const rect0 = item.getBoundingClientRect();
      offsetRef.current = { x: startX - rect0.left, y: startY - rect0.top };

      const activate = (ev: PointerEvent) => {
        if (activated) return;
        activated = true;
        ev.preventDefault();

        const rect = item.getBoundingClientRect();
        offsetRef.current = {
          x: ev.clientX - rect.left,
          y: ev.clientY - rect.top,
        };

        const float = document.createElement("div");
        float.className = floatClassName;
        float.innerHTML = item.innerHTML;
        float.style.width = `${rect.width}px`;
        float.style.left = `${rect.left}px`;
        float.style.top = `${rect.top}px`;
        document.body.appendChild(float);
        floatRef.current = float;

        activeIdRef.current = id;
        setActiveId(id);
        document.body.classList.add(bodyDraggingClass);
      };

      const onMove = (ev: PointerEvent) => {
        if (pointerIdRef.current !== ev.pointerId) return;

        if (!activated) {
          const dx = ev.clientX - startX;
          const dy = ev.clientY - startY;
          if (Math.hypot(dx, dy) < activationDistance) return;
          activate(ev);
        }

        if (activeIdRef.current !== id) return;
        const f = floatRef.current;
        if (f) {
          f.style.left = `${ev.clientX - offsetRef.current.x}px`;
          f.style.top = `${ev.clientY - offsetRef.current.y}px`;
        }
        const from = idsRef.current.indexOf(id);
        if (from < 0) return;
        const to = indexFromPointer(ev.clientY);
        if (to === from) return;

        const list = listRef.current;
        if (list) {
          firstRectsRef.current = captureRects(list, itemSelector);
        }
        onReorderRef.current(id, to);
      };

      const onUp = (ev: PointerEvent) => {
        if (pointerIdRef.current != null && ev.pointerId !== pointerIdRef.current) {
          return;
        }
        item.removeEventListener("pointermove", onMove);
        item.removeEventListener("pointerup", onUp);
        item.removeEventListener("pointercancel", onUp);
        document.removeEventListener("pointermove", onMove);
        document.removeEventListener("pointerup", onUp);
        document.removeEventListener("pointercancel", onUp);
        if (activated) {
          const suppressClick = (ce: MouseEvent) => {
            ce.preventDefault();
            ce.stopPropagation();
          };
          item.addEventListener("click", suppressClick, { capture: true, once: true });
          window.setTimeout(() => {
            item.removeEventListener("click", suppressClick, true);
          }, 0);
        }
        cleanup();
      };

      item.addEventListener("pointermove", onMove);
      item.addEventListener("pointerup", onUp);
      item.addEventListener("pointercancel", onUp);
      document.addEventListener("pointermove", onMove);
      document.addEventListener("pointerup", onUp);
      document.addEventListener("pointercancel", onUp);
    },
    [
      activationDistance,
      bodyDraggingClass,
      cleanup,
      floatClassName,
      handleSelector,
      ignoreSelector,
      indexFromPointer,
      itemSelector,
      listRef,
    ],
  );

  return { activeId, onItemPointerDown };
}

export function moveIdToIndex(ids: string[], fromId: string, toIndex: number): string[] {
  const from = ids.indexOf(fromId);
  if (from < 0) return ids;
  if (from === toIndex) return ids;
  const next = [...ids];
  next.splice(from, 1);
  const at = Math.max(0, Math.min(toIndex, next.length));
  next.splice(at, 0, fromId);
  return next;
}

function captureRects(
  list: HTMLElement,
  itemSelector: string,
): Map<string, DOMRect> {
  const map = new Map<string, DOMRect>();
  list.querySelectorAll<HTMLElement>(itemSelector).forEach((el) => {
    const id = el.getAttribute("data-sortable-id");
    if (id) map.set(id, el.getBoundingClientRect());
  });
  return map;
}

function playFlip(
  list: HTMLElement,
  itemSelector: string,
  first: Map<string, DOMRect>,
  durationMs: number,
) {
  const els = Array.from(list.querySelectorAll<HTMLElement>(itemSelector));
  for (const el of els) {
    const id = el.getAttribute("data-sortable-id");
    if (!id) continue;
    const prev = first.get(id);
    if (!prev) continue;
    const last = el.getBoundingClientRect();
    const dy = prev.top - last.top;
    if (Math.abs(dy) < 0.5) continue;
    el.style.transition = "none";
    el.style.transform = `translateY(${dy}px)`;
  }
  // Force reflow then animate to identity.
  void list.offsetHeight;
  for (const el of els) {
    if (!el.style.transform) continue;
    el.style.transition = `transform ${durationMs}ms ease`;
    el.style.transform = "";
  }
  window.setTimeout(() => {
    for (const el of els) {
      el.style.transition = "";
      el.style.transform = "";
    }
  }, durationMs + 40);
}
