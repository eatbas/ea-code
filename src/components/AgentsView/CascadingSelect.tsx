import type { ReactNode } from "react";
import { useState, useEffect, useRef, useCallback } from "react";
import type { AgentBackend, AppSettings, CliHealth } from "../../types";
import { BACKEND_OPTIONS } from "../shared/constants";
import { backendLabel, modelLabel, getModelOptionsForBackend } from "./agentHelpers";

/** Props for the CascadingSelect component. */
export interface CascadingSelectProps {
  backend: AgentBackend;
  model: string;
  settings: AppSettings;
  optional?: boolean;
  cliHealth: CliHealth | null;
  cliHealthChecking: boolean;
  onChange: (backend: AgentBackend | null, model: string | null) => void;
}

/** Two-level hover dropdown (backend then models). */
export function CascadingSelect({
  backend,
  model,
  settings,
  optional,
  cliHealth,
  cliHealthChecking,
  onChange,
}: CascadingSelectProps): ReactNode {
  const [open, setOpen] = useState(false);
  const [hoveredBackend, setHoveredBackend] = useState<AgentBackend | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const backendOptions = cliHealth
    ? BACKEND_OPTIONS.filter((opt) => cliHealth[opt.value].available)
    : [];
  const hasSelectableBackends = backendOptions.length > 0;
  const triggerDisabled = cliHealthChecking || (!optional && !hasSelectableBackends);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent): void {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  // Close on Escape
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    },
    [],
  );
  useEffect(() => {
    if (!open) return;
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [open, handleKeyDown]);

  // Ensure disabled state closes the dropdown immediately.
  useEffect(() => {
    if (triggerDisabled) {
      setOpen(false);
      setHoveredBackend(null);
    }
  }, [triggerDisabled]);

  const isSkipped = !backend;
  const displayText = cliHealthChecking && !cliHealth
    ? "Checking installed CLIs..."
    : isSkipped
      ? "Skip"
      : `${backendLabel(backend)} · ${modelLabel(backend, model)}`;

  return (
    <div ref={containerRef} className="relative">
      {/* Trigger button */}
      <button
        type="button"
        onClick={() => {
          if (triggerDisabled) return;
          setOpen((prev) => !prev);
          setHoveredBackend(null);
        }}
        disabled={triggerDisabled}
        className={`flex w-full items-center justify-between rounded border px-3 py-2 text-sm transition-colors ${
          isSkipped
            ? "border-[#2e2e48] bg-[#1a1a24] text-[#6b6b80]"
            : "border-[#2e2e48] bg-[#1a1a24] text-[#e4e4ed]"
        } hover:border-[#3e3e58] focus:border-[#3e3e58] focus:outline-none disabled:cursor-not-allowed disabled:opacity-60`}
      >
        <span className="truncate">{displayText}</span>
        <svg
          className={`ml-2 h-3.5 w-3.5 shrink-0 text-[#6b6b80] transition-transform ${open ? "rotate-180" : ""}`}
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M3 4.5L6 7.5L9 4.5" />
        </svg>
      </button>

      {/* Dropdown popover */}
      {open && (
        <div className="absolute left-0 z-50 mt-1 flex">
          {/* Backend list */}
          <div className="min-w-[140px] rounded-l border border-[#2e2e48] bg-[#1a1a2e] py-1 shadow-lg">
            {optional && (
              <button
                type="button"
                onClick={() => {
                  onChange(null, null);
                  setOpen(false);
                }}
                onMouseEnter={() => setHoveredBackend(null)}
                className={`flex w-full items-center px-3 py-1.5 text-sm transition-colors ${
                  isSkipped
                    ? "bg-[#24243a] text-[#e4e4ed]"
                    : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                }`}
              >
                Skip
              </button>
            )}
            {backendOptions.map((opt) => {
              const isActive = opt.value === hoveredBackend;
              const isCurrent = opt.value === backend;
              return (
                <button
                  key={opt.value}
                  type="button"
                  onMouseEnter={() => setHoveredBackend(opt.value)}
                  onClick={() => setHoveredBackend(opt.value)}
                  className={`flex w-full items-center justify-between px-3 py-1.5 text-sm transition-colors ${
                    isActive
                      ? "bg-[#24243a] text-[#e4e4ed]"
                      : isCurrent
                        ? "text-[#e4e4ed]"
                        : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                  }`}
                >
                  <span>{opt.label}</span>
                  <svg
                    className="ml-2 h-3 w-3 shrink-0 text-[#6b6b80]"
                    viewBox="0 0 12 12"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M4.5 3L7.5 6L4.5 9" />
                  </svg>
                </button>
              );
            })}
            {backendOptions.length === 0 && (
              <span className="block px-3 py-1.5 text-xs text-[#6b6b80]">
                No installed CLIs
              </span>
            )}
          </div>

          {/* Model submenu */}
          {hoveredBackend && (
            <div className="min-w-[150px] rounded-r border border-l-0 border-[#2e2e48] bg-[#1a1a2e] py-1 shadow-lg">
              {(() => {
                const models = getModelOptionsForBackend(hoveredBackend, settings);
                if (models.length === 0) {
                  return (
                    <span className="block px-3 py-1.5 text-xs text-[#6b6b80]">
                      No models enabled
                    </span>
                  );
                }
                return models.map((m) => {
                  const isSelected = hoveredBackend === backend && m.value === model;
                  return (
                    <button
                      key={m.value}
                      type="button"
                      onClick={() => {
                        onChange(hoveredBackend, m.value);
                        setOpen(false);
                      }}
                      className={`flex w-full items-center gap-2 px-3 py-1.5 text-sm transition-colors ${
                        isSelected
                          ? "bg-[#24243a] text-[#e4e4ed]"
                          : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                      }`}
                    >
                      {isSelected && (
                        <svg
                          className="h-3 w-3 shrink-0 text-[#e4e4ed]"
                          viewBox="0 0 12 12"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="2"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        >
                          <path d="M2.5 6L5 8.5L9.5 3.5" />
                        </svg>
                      )}
                      <span>{m.label}</span>
                    </button>
                  );
                });
              })()}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
