import { useLayoutEffect, useRef, type RefObject } from "react";

const FLIP_MS = 220;
const FLIP_EASING = "cubic-bezier(0.22, 1, 0.36, 1)";

/**
 * Animates elements marked with `data-flip-id` when their layout position
 * changes between renders (FLIP: First, Last, Invert, Play).
 *
 * Cross-lane moves temporarily lift lane overflow clipping so the card can
 * travel horizontally without being cropped.
 */
export function useFlipLayout(
  containerRef: RefObject<HTMLElement | null>,
  layoutKey: string,
  enabled = true,
): void {
  const prevRectsRef = useRef(new Map<string, DOMRect>());
  const prevKeyRef = useRef<string | null>(null);
  const overflowTimerRef = useRef<number | null>(null);

  useLayoutEffect(() => {
    const root = containerRef.current;
    if (!root) return;

    const nodes = Array.from(root.querySelectorAll<HTMLElement>("[data-flip-id]"));
    const nextRects = new Map<string, DOMRect>();

    for (const node of nodes) {
      const id = node.dataset.flipId;
      if (!id) continue;
      nextRects.set(id, node.getBoundingClientRect());
    }

    const keyChanged = prevKeyRef.current !== null && prevKeyRef.current !== layoutKey;
    const reduceMotion =
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const shouldAnimate = enabled && keyChanged && !reduceMotion;

    if (shouldAnimate) {
      let needsOverflowEscape = false;
      const plays: Array<{ node: HTMLElement; dx: number; dy: number }> = [];

      for (const node of nodes) {
        const id = node.dataset.flipId;
        if (!id) continue;

        const last = nextRects.get(id);
        const first = prevRectsRef.current.get(id);
        if (!first || !last) continue;

        const dx = first.left - last.left;
        const dy = first.top - last.top;
        if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;

        if (Math.abs(dx) > 2) needsOverflowEscape = true;
        plays.push({ node, dx, dy });
      }

      if (needsOverflowEscape) {
        if (overflowTimerRef.current !== null) {
          window.clearTimeout(overflowTimerRef.current);
        }
        const scrolls = root.querySelectorAll<HTMLElement>("[data-lane-scroll]");
        for (const el of scrolls) {
          el.style.overflow = "visible";
        }
        overflowTimerRef.current = window.setTimeout(() => {
          for (const el of scrolls) {
            el.style.overflow = "";
          }
          overflowTimerRef.current = null;
        }, FLIP_MS + 40);
      }

      for (const { node, dx, dy } of plays) {
        node.getAnimations().forEach((animation) => animation.cancel());
        node.style.zIndex = "20";
        const animation = node.animate(
          [
            { transform: `translate(${dx}px, ${dy}px)` },
            { transform: "translate(0px, 0px)" },
          ],
          { duration: FLIP_MS, easing: FLIP_EASING, fill: "none" },
        );
        void animation.finished.then(
          () => {
            node.style.zIndex = "";
          },
          () => {
            node.style.zIndex = "";
          },
        );
      }
    }

    prevRectsRef.current = nextRects;
    prevKeyRef.current = layoutKey;
  }, [containerRef, layoutKey, enabled]);
}
