import type { ReactNode } from "react";

interface ImageThumbnailsProps {
  previews: Array<{ previewUrl: string; label: string }>;
  onRemove: (index: number) => void;
}

export function ImageThumbnails({ previews, onRemove }: ImageThumbnailsProps): ReactNode {
  return (
    <div className="flex flex-wrap gap-2 px-4 pt-3">
      {previews.map((image, index) => (
        <div key={image.previewUrl} className="group relative h-12 w-12 shrink-0">
          <img
            src={image.previewUrl}
            alt={image.label}
            title={image.label}
            className="h-full w-full rounded-lg object-cover border border-edge"
          />
          <button
            type="button"
            onClick={() => onRemove(index)}
            className="absolute -right-1.5 -top-1.5 hidden h-4 w-4 items-center justify-center rounded-full bg-surface text-fg-muted text-[10px] border border-edge group-hover:flex"
            aria-label={`Remove ${image.label}`}
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
