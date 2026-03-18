import { useCallback, useEffect, useRef } from "react";
import type { RefObject } from "react";

interface StickyAutoScrollResult<T extends HTMLElement> {
  scrollRef: RefObject<T | null>;
  onScroll: () => void;
}

const DEFAULT_BOTTOM_THRESHOLD_PX = 16;

/**
 * Keeps a scroll container pinned to the bottom until the user scrolls away.
 * Auto-scrolling resumes once the user returns to the latest content.
 */
export function useStickyAutoScroll<T extends HTMLElement>(
  dependencyKey: string | number,
  thresholdPx = DEFAULT_BOTTOM_THRESHOLD_PX,
): StickyAutoScrollResult<T> {
  const scrollRef = useRef<T>(null);
  const shouldAutoScrollRef = useRef(true);

  const onScroll = useCallback(() => {
    const element = scrollRef.current;
    if (!element) return;

    const distanceFromBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
    shouldAutoScrollRef.current = distanceFromBottom <= thresholdPx;
  }, [thresholdPx]);

  useEffect(() => {
    const element = scrollRef.current;
    if (!element || !shouldAutoScrollRef.current) return;

    element.scrollTop = element.scrollHeight;
  }, [dependencyKey]);

  return { scrollRef, onScroll };
}
