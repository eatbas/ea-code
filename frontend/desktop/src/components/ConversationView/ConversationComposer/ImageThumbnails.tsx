import { type ReactNode, useState } from "react";
import { Trash2 } from "lucide-react";

interface ImageThumbnailsProps {
  previews: Array<{ previewUrl: string; label: string }>;
  onRemove: (index: number) => void;
  onPreview: (previewUrl: string) => void;
}

export function ImageThumbnails({
  previews,
  onRemove,
  onPreview,
}: ImageThumbnailsProps): ReactNode {
  const [confirmIndex, setConfirmIndex] = useState<number | null>(null);

  return (
    <div className="flex flex-wrap gap-2 px-4 pt-3">
      {previews.map((image, index) => (
        <div key={image.previewUrl} className="flex items-center gap-1.5 rounded-lg border border-edge bg-surface px-1.5 py-1">
          <button
            type="button"
            onClick={() => onPreview(image.previewUrl)}
            className="h-10 w-10 shrink-0 overflow-hidden rounded-md focus:outline-none focus:ring-1 focus:ring-accent"
            title={`View ${image.label}`}
          >
            <img
              src={image.previewUrl}
              alt={image.label}
              className="h-full w-full object-cover"
            />
          </button>

          {confirmIndex === index ? (
            <div className="flex items-center gap-1">
              <span className="text-[11px] text-fg-muted">Delete?</span>
              <button
                type="button"
                onClick={() => {
                  onRemove(index);
                  setConfirmIndex(null);
                }}
                className="rounded bg-danger-bg px-1.5 py-0.5 text-[11px] font-medium text-danger-text transition-colors hover:bg-danger-bg-hover hover:text-danger-text-hover"
              >
                Yes
              </button>
              <button
                type="button"
                onClick={() => setConfirmIndex(null)}
                className="rounded px-1.5 py-0.5 text-[11px] font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg"
              >
                No
              </button>
            </div>
          ) : (
            <button
              type="button"
              onClick={() => setConfirmIndex(index)}
              className="flex h-5 w-5 items-center justify-center rounded text-fg-faint transition-colors hover:bg-active hover:text-danger-text"
              aria-label={`Remove ${image.label}`}
              title="Remove image"
            >
              <Trash2 size={12} />
            </button>
          )}
        </div>
      ))}
    </div>
  );
}
