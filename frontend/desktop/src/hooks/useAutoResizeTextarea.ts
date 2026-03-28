import type { RefObject } from "react";
import { useEffect } from "react";

/** Auto-resizes a textarea up to `maxLines` based on its scroll height. */
export function useAutoResizeTextarea(
  ref: RefObject<HTMLTextAreaElement | null>,
  value: string,
  maxLines = 3,
): void {
  useEffect(() => {
    const textarea = ref.current;
    if (!textarea) {
      return;
    }

    textarea.style.height = "0px";

    const computed = window.getComputedStyle(textarea);
    const lineHeight = Number.parseFloat(computed.lineHeight) || 24;
    const paddingTop = Number.parseFloat(computed.paddingTop) || 0;
    const paddingBottom = Number.parseFloat(computed.paddingBottom) || 0;
    const maxHeight = lineHeight * maxLines + paddingTop + paddingBottom;
    const nextHeight = Math.min(textarea.scrollHeight, maxHeight);

    textarea.style.height = `${nextHeight}px`;
    textarea.style.overflowY = textarea.scrollHeight > maxHeight ? "auto" : "hidden";
  }, [ref, value, maxLines]);
}
