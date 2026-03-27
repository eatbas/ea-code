import type { ReactNode } from "react";

/** Dimension classes for each supported size variant. */
const SIZE_CLASSES: Record<CheckmarkSize, string> = {
  sm: "h-2.5 w-2.5",
  md: "h-3 w-3",
};

type CheckmarkSize = "sm" | "md";

interface CheckmarkProps {
  /** Predefined size variant. */
  size: CheckmarkSize;
  /** Optional extra CSS classes (merged after the size classes). */
  className?: string;
}

/** Reusable checkmark tick SVG icon. */
export function Checkmark({ size, className }: CheckmarkProps): ReactNode {
  const sizeClass = SIZE_CLASSES[size];
  const combinedClass = className
    ? `${sizeClass} ${className}`
    : sizeClass;

  return (
    <svg
      className={combinedClass}
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M2.5 6L5 8.5L9.5 3.5" />
    </svg>
  );
}
