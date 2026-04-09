import { type ReactNode, useCallback, useEffect } from "react";
import { X } from "lucide-react";

interface ImagePreviewModalProps {
  src: string;
  onClose: () => void;
}

/** Full-screen modal overlay for previewing an attached image. */
export function ImagePreviewModal({ src, onClose }: ImagePreviewModalProps): ReactNode {
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    },
    [onClose],
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label="Image preview"
    >
      <button
        type="button"
        onClick={onClose}
        className="absolute right-4 top-4 flex h-8 w-8 items-center justify-center rounded-full bg-surface/80 text-fg-muted transition-colors hover:bg-surface hover:text-fg"
        aria-label="Close preview"
      >
        <X size={18} />
      </button>
      <img
        src={src}
        alt="Image preview"
        className="max-h-[85vh] max-w-[90vw] rounded-lg object-contain shadow-2xl"
        onClick={(event) => event.stopPropagation()}
      />
    </div>
  );
}
