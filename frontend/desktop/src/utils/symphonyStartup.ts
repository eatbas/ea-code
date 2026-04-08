import type { ApiHealth } from "../types";

export type SymphonyStartupPhase = "initialising" | "checking" | "connected" | "failed";

export type SymphonyStartupDiagnosticState = "pending" | "ready" | "warning" | "failed";

export interface SymphonyStartupStatus {
  phase: SymphonyStartupPhase;
  errorMessage: string | null;
  sidecar: SymphonyStartupDiagnosticState;
  api: SymphonyStartupDiagnosticState;
  providers: SymphonyStartupDiagnosticState;
  versions: SymphonyStartupDiagnosticState;
}

interface SymphonyStartupStatusInput {
  sidecarReady: boolean | null;
  sidecarError?: string | null;
  apiHealth: ApiHealth | null;
  apiChecking: boolean;
  versionsLoading: boolean;
  providerCount: number;
}

export function shouldAutoRefreshOnReady(
  previousReady: boolean | null | undefined,
  nextReady: boolean | null,
): boolean {
  return nextReady === true && previousReady !== true;
}

export function getSymphonyStartupStatus({
  sidecarReady,
  sidecarError = null,
  apiHealth,
  apiChecking,
  versionsLoading,
  providerCount,
}: SymphonyStartupStatusInput): SymphonyStartupStatus {
  const errorMessage = getSymphonyStartupErrorMessage(sidecarReady, sidecarError, apiHealth);
  const sidecar = getSidecarState(sidecarReady, sidecarError);
  const api = getApiState(sidecarReady, apiHealth, apiChecking);
  const providers = getProvidersState(sidecarReady, apiHealth, apiChecking, providerCount);
  const versions = getVersionsState(sidecarReady, apiHealth, versionsLoading);

  if (sidecar === "failed" || api === "failed") {
    return {
      phase: "failed",
      errorMessage,
      sidecar,
      api,
      providers,
      versions,
    };
  }

  if (sidecarReady !== true) {
    return {
      phase: "initialising",
      errorMessage,
      sidecar,
      api,
      providers,
      versions,
    };
  }

  if (apiChecking || versionsLoading || apiHealth === null) {
    return {
      phase: "checking",
      errorMessage,
      sidecar,
      api,
      providers,
      versions,
    };
  }

  return {
    phase: "connected",
    errorMessage,
    sidecar,
    api,
    providers,
    versions,
  };
}

function getSidecarState(
  sidecarReady: boolean | null,
  sidecarError: string | null,
): SymphonyStartupDiagnosticState {
  if (sidecarError || sidecarReady === false) {
    return "failed";
  }
  if (sidecarReady === true) {
    return "ready";
  }
  return "pending";
}

function getApiState(
  sidecarReady: boolean | null,
  apiHealth: ApiHealth | null,
  apiChecking: boolean,
): SymphonyStartupDiagnosticState {
  if (sidecarReady !== true || apiChecking || apiHealth === null) {
    return "pending";
  }
  return apiHealth.connected ? "ready" : "failed";
}

function getProvidersState(
  sidecarReady: boolean | null,
  apiHealth: ApiHealth | null,
  apiChecking: boolean,
  providerCount: number,
): SymphonyStartupDiagnosticState {
  if (sidecarReady !== true || apiChecking || apiHealth === null) {
    return "pending";
  }
  if (!apiHealth.connected) {
    return "failed";
  }
  if (providerCount === 0) {
    return "warning";
  }
  return "ready";
}

function getVersionsState(
  sidecarReady: boolean | null,
  apiHealth: ApiHealth | null,
  versionsLoading: boolean,
): SymphonyStartupDiagnosticState {
  if (sidecarReady !== true || versionsLoading || apiHealth === null) {
    return "pending";
  }
  return apiHealth.connected ? "ready" : "failed";
}

function getSymphonyStartupErrorMessage(
  sidecarReady: boolean | null,
  sidecarError: string | null,
  apiHealth: ApiHealth | null,
): string | null {
  if (sidecarError) {
    return sidecarError;
  }
  if (sidecarReady === false) {
    return "Symphony failed to start.";
  }
  if (apiHealth && !apiHealth.connected) {
    return apiHealth.error ?? apiHealth.status ?? "Symphony is unavailable.";
  }
  return null;
}
