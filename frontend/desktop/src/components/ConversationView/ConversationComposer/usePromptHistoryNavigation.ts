import { useEffect, useState } from "react";
import type { KeyboardEvent, RefObject } from "react";

interface UsePromptHistoryNavigationParams {
  prompt: string;
  promptHistory: string[];
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  onPromptChange: (prompt: string) => void;
  onSubmit: () => Promise<void>;
  disabled: boolean;
}

export function usePromptHistoryNavigation({
  prompt,
  promptHistory,
  textareaRef,
  onPromptChange,
  onSubmit,
  disabled,
}: UsePromptHistoryNavigationParams) {
  const [historyIndex, setHistoryIndex] = useState<number>(-1);
  const [draftBeforeHistory, setDraftBeforeHistory] = useState("");

  useEffect(() => {
    setHistoryIndex(-1);
    setDraftBeforeHistory("");
  }, [promptHistory]);

  function updatePromptFromHistory(nextPrompt: string): void {
    onPromptChange(nextPrompt);
    requestAnimationFrame(() => {
      const textarea = textareaRef.current;
      if (!textarea) {
        return;
      }

      const cursor = nextPrompt.length;
      textarea.setSelectionRange(cursor, cursor);
    });
  }

  function canNavigateHistory(
    event: KeyboardEvent<HTMLTextAreaElement>,
    direction: "up" | "down",
  ): boolean {
    const textarea = event.currentTarget;
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
      return false;
    }
    if (textarea.selectionStart !== textarea.selectionEnd) {
      return false;
    }

    const beforeCursor = textarea.value.slice(0, textarea.selectionStart);
    const afterCursor = textarea.value.slice(textarea.selectionEnd);
    return direction === "up" ? !beforeCursor.includes("\n") : !afterCursor.includes("\n");
  }

  function handleHistoryNavigation(direction: "up" | "down"): void {
    if (promptHistory.length === 0) {
      return;
    }

    if (direction === "up") {
      if (historyIndex === -1) {
        setDraftBeforeHistory(prompt);
        setHistoryIndex(promptHistory.length - 1);
        updatePromptFromHistory(promptHistory[promptHistory.length - 1] ?? "");
        return;
      }

      const nextIndex = Math.max(0, historyIndex - 1);
      setHistoryIndex(nextIndex);
      updatePromptFromHistory(promptHistory[nextIndex] ?? "");
      return;
    }

    if (historyIndex === -1) {
      return;
    }

    const nextIndex = historyIndex + 1;
    if (nextIndex >= promptHistory.length) {
      setHistoryIndex(-1);
      updatePromptFromHistory(draftBeforeHistory);
      return;
    }

    setHistoryIndex(nextIndex);
    updatePromptFromHistory(promptHistory[nextIndex] ?? "");
  }

  const handlePromptChange = (nextPrompt: string): void => {
    onPromptChange(nextPrompt);
    if (historyIndex !== -1) {
      setHistoryIndex(-1);
      setDraftBeforeHistory("");
    }
  };

  const handlePromptKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>): void => {
    if (event.key === "ArrowUp" && canNavigateHistory(event, "up")) {
      event.preventDefault();
      handleHistoryNavigation("up");
      return;
    }

    if (event.key === "ArrowDown" && canNavigateHistory(event, "down")) {
      event.preventDefault();
      handleHistoryNavigation("down");
      return;
    }

    if (event.key !== "Enter" || event.shiftKey) {
      return;
    }

    event.preventDefault();
    if (!disabled) {
      void onSubmit();
    }
  };

  const resetHistory = (): void => {
    setHistoryIndex(-1);
    setDraftBeforeHistory("");
  };

  return {
    handlePromptChange,
    handlePromptKeyDown,
    resetHistory,
  };
}
