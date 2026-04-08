import { useCallback, useEffect, useRef, useState } from "react";
import type { RefObject } from "react";

interface StickyAutoScrollResult<T extends HTMLElement> {
  scrollRef: RefObject<T | null>;
  /** True when the user is in the middle of the scroll (not at top, not at bottom). */
  showScrollButtons: boolean;
  /** Smoothly scrolls back to the bottom and re-enables auto-scroll. */
  scrollToBottom: () => void;
  /** Smoothly scrolls to the top. */
  scrollToTop: () => void;
}

const BOTTOM_THRESHOLD_PX = 80;

/**
 * Keeps a scroll container pinned to the bottom while new content streams in.
 *
 * Strategy:
 * - `shouldAutoScroll` ref starts true.
 * - A native `scroll` event listener (not React synthetic — fires synchronously)
 *   flips the ref based on distance from bottom, but ONLY when we are not in
 *   the middle of a programmatic scroll (`isAutoScrolling` guard).
 * - A MutationObserver watches for DOM changes and scrolls to bottom when
 *   `shouldAutoScroll` is true.
 * - User scrolling up disables auto-scroll. Scrolling back near the bottom
 *   re-enables it.
 */
export function useStickyAutoScroll<T extends HTMLElement>(
  _dependencyKey: string | number,
): StickyAutoScrollResult<T> {
  const scrollRef = useRef<T>(null);
  const shouldAutoScrollRef = useRef(true);
  const isAutoScrollingRef = useRef(false);
  const [showScrollButtons, setShowScrollButtons] = useState(false);

  const doScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    isAutoScrollingRef.current = true;
    el.scrollTop = el.scrollHeight;
    // Hold the guard for two frames so the browser's scroll event
    // (which may fire in the same or next frame) is caught.
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        isAutoScrollingRef.current = false;
      });
    });
  }, []);

  // Native scroll listener — fires synchronously, before rAF.
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;

    const handleScroll = (): void => {
      if (isAutoScrollingRef.current) return;
      const dist = el.scrollHeight - el.scrollTop - el.clientHeight;
      const atBottom = dist <= BOTTOM_THRESHOLD_PX;
      const atTop = el.scrollTop <= BOTTOM_THRESHOLD_PX;
      shouldAutoScrollRef.current = atBottom;
      // Only show buttons when scrolled to the middle — not at top, not at bottom.
      setShowScrollButtons(!atBottom && !atTop);
    };

    el.addEventListener("scroll", handleScroll, { passive: true });
    return () => el.removeEventListener("scroll", handleScroll);
  }, []);

  // MutationObserver — scroll on DOM changes when sticky.
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;

    const observer = new MutationObserver(() => {
      if (!shouldAutoScrollRef.current) return;
      doScroll();
    });

    observer.observe(el, { childList: true, subtree: true, characterData: true });

    // Initial scroll.
    if (shouldAutoScrollRef.current) {
      doScroll();
    }

    return () => observer.disconnect();
  }, [doScroll]);

  // Fallback for full React subtree replacements.
  useEffect(() => {
    if (!shouldAutoScrollRef.current) return;
    doScroll();
  }, [_dependencyKey, doScroll]);

  const scrollToBottom = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    shouldAutoScrollRef.current = true;
    isAutoScrollingRef.current = true;
    setShowScrollButtons(false);
    el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        isAutoScrollingRef.current = false;
      });
    });
  }, []);

  const scrollToTop = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    shouldAutoScrollRef.current = false;
    isAutoScrollingRef.current = true;
    setShowScrollButtons(false);
    el.scrollTo({ top: 0, behavior: "smooth" });
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        isAutoScrollingRef.current = false;
      });
    });
  }, []);

  return { scrollRef, showScrollButtons, scrollToBottom, scrollToTop };
}
