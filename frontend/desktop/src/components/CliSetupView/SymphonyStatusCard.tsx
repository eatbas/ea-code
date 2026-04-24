import type { ReactNode } from "react";
import type { ApiHealth } from "../../types";
import type {
  SymphonyStartupDiagnosticState,
  SymphonyStartupStatus,
} from "../../utils/symphonyStartup";

interface SymphonyStatusCardProps {
  status: SymphonyStartupStatus;
  apiHealth: ApiHealth | null;
  providerCount: number;
}

const DIAGNOSTIC_STYLES: Record<SymphonyStartupDiagnosticState, string> = {
  pending: "border-edge bg-elevated text-fg-muted",
  ready: "border-success/30 bg-success/10 text-success",
  warning: "border-warning/30 bg-warning/10 text-warning",
  failed: "border-danger/30 bg-danger/10 text-danger",
};

const SUMMARY_STYLES: Record<SymphonyStartupStatus["phase"], string> = {
  initialising: "border-edge bg-panel",
  checking: "border-edge bg-panel",
  connected: "border-success/25 bg-success/10",
  failed: "border-danger/25 bg-danger/10",
};

interface DiagnosticItem {
  label: string;
  value: string;
  state: SymphonyStartupDiagnosticState;
}

function summaryCopy(status: SymphonyStartupStatus, apiHealth: ApiHealth | null, providerCount: number): {
  title: string;
  body: string;
} {
  if (status.phase === "initialising") {
    return {
      title: "Initialising Symphony",
      body: "Maestro is starting the local Symphony service. API checks will begin automatically when it is ready.",
    };
  }

  if (status.phase === "checking") {
    return {
      title: "Checking Symphony",
      body: "Symphony is up. Maestro is now verifying API health, provider availability, and CLI version data.",
    };
  }

  if (status.phase === "failed") {
    return {
      title: "Symphony needs attention",
      body: status.errorMessage ?? "Maestro could not connect to Symphony. Open CLI Setup diagnostics and retry once the issue is fixed.",
    };
  }

  if (providerCount === 0) {
    return {
      title: "Symphony is ready",
      body: "The service is healthy, but it has not reported any providers yet. Check your installed CLIs if this persists.",
    };
  }

  const musicianSuffix = apiHealth?.musicianCount === 1 ? "" : "s";
  const musicianText = apiHealth?.musicianCount != null
    ? `${apiHealth.musicianCount} musician${musicianSuffix} online`
    : "service healthy";

  return {
    title: "Symphony is ready",
    body: `${musicianText}. ${providerCount} provider${providerCount === 1 ? "" : "s"} detected.`,
  };
}

function diagnosticItems(status: SymphonyStartupStatus, apiHealth: ApiHealth | null, providerCount: number): DiagnosticItem[] {
  return [
    {
      label: "Symphony-API",
      value: status.sidecar === "ready" ? "Ready" : status.sidecar === "failed" ? "Failed" : "Starting",
      state: status.sidecar,
    },
    {
      label: "API",
      value: status.api === "ready"
        ? (apiHealth?.status ?? "Connected")
        : status.api === "failed"
          ? "Unavailable"
          : "Checking",
      state: status.api,
    },
    {
      label: "Providers",
      value: status.providers === "ready"
        ? `${providerCount} detected`
        : status.providers === "warning"
          ? "None detected"
          : status.providers === "failed"
            ? "Unavailable"
            : "Checking",
      state: status.providers,
    },
    {
      label: "CLI versions",
      value: status.versions === "ready"
        ? "Loaded"
        : status.versions === "failed"
          ? "Unavailable"
          : "Checking",
      state: status.versions,
    },
  ];
}

export function SymphonyStatusCard({
  status,
  apiHealth,
  providerCount,
}: SymphonyStatusCardProps): ReactNode {
  const summary = summaryCopy(status, apiHealth, providerCount);
  const items = diagnosticItems(status, apiHealth, providerCount);

  return (
    <section className={`rounded-2xl border px-5 py-5 shadow-[0_0_0_1px_rgba(49,49,52,0.18)] ${SUMMARY_STYLES[status.phase]}`}>
      <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-fg-faint">
            Symphony Status
          </p>
          <h2 className="mt-2 text-lg font-semibold text-fg">{summary.title}</h2>
          <p className="mt-1 text-sm text-fg-muted">{summary.body}</p>
        </div>
        {apiHealth?.url && status.phase === "connected" && (
          <span className="rounded-full border border-edge bg-surface px-3 py-1 text-xs font-mono text-fg-faint">
            {apiHealth.url}
          </span>
        )}
      </div>

      <div className="mt-5 grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {items.map((item) => (
          <div
            key={item.label}
            className={`rounded-xl border px-3 py-3 ${DIAGNOSTIC_STYLES[item.state]}`}
          >
            <p className="text-[11px] font-semibold uppercase tracking-[0.14em]">{item.label}</p>
            <p className="mt-2 text-sm font-semibold">{item.value}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
