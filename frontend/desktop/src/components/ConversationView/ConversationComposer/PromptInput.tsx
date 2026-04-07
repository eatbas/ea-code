import { useCallback } from "react";
import type { ClipboardEvent, KeyboardEvent, ReactNode, RefObject } from "react";

interface PromptInputProps {
  prompt: string;
  disabled: boolean;
  placeholder: string;
  textareaRef: RefObject<HTMLTextAreaElement | null>;
  onPromptChange: (prompt: string) => void;
  onKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
  onImagePaste: (files: File[]) => void;
}

export function PromptInput({
  prompt,
  disabled,
  placeholder,
  textareaRef,
  onPromptChange,
  onKeyDown,
  onImagePaste,
}: PromptInputProps): ReactNode {
  const handlePaste = useCallback((event: ClipboardEvent<HTMLTextAreaElement>) => {
    const items = Array.from(event.clipboardData.items);
    const imageFiles = items
      .filter((item) => item.type.startsWith("image/"))
      .map((item) => item.getAsFile())
      .filter((file): file is File => file !== null);

    if (imageFiles.length > 0) {
      onImagePaste(imageFiles);
    }
    // Do NOT preventDefault — text paste continues normally
  }, [onImagePaste]);

  return (
    <label className="block">
      <span className="sr-only">Prompt</span>
      <textarea
        ref={textareaRef}
        value={prompt}
        disabled={disabled}
        onChange={(event) => onPromptChange(event.target.value)}
        onKeyDown={onKeyDown}
        onPaste={handlePaste}
        rows={1}
        placeholder={placeholder}
        className={`w-full resize-none bg-transparent px-4 py-3 text-sm leading-6 text-fg placeholder:text-fg-faint focus:outline-none ${disabled ? "cursor-not-allowed opacity-50" : ""}`}
      />
    </label>
  );
}
