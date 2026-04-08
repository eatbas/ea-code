import assert from "node:assert/strict";
import test from "node:test";

import {
  getSymphonyStartupStatus,
  shouldAutoRefreshOnReady,
} from "./symphonyStartup";

test("startup status stays initialising while the sidecar is still booting", () => {
  const status = getSymphonyStartupStatus({
    sidecarReady: null,
    sidecarError: null,
    apiHealth: null,
    apiChecking: false,
    versionsLoading: false,
    providerCount: 0,
  });

  assert.equal(status.phase, "initialising");
  assert.equal(status.sidecar, "pending");
  assert.equal(status.api, "pending");
});

test("startup status becomes checking after the sidecar is ready but before API data arrives", () => {
  const status = getSymphonyStartupStatus({
    sidecarReady: true,
    sidecarError: null,
    apiHealth: null,
    apiChecking: false,
    versionsLoading: false,
    providerCount: 0,
  });

  assert.equal(status.phase, "checking");
  assert.equal(status.sidecar, "ready");
  assert.equal(status.api, "pending");
});

test("startup status becomes connected after healthy API data arrives", () => {
  const status = getSymphonyStartupStatus({
    sidecarReady: true,
    sidecarError: null,
    apiHealth: {
      connected: true,
      url: "http://127.0.0.1:8719",
      status: "ok",
      musicianCount: 4,
    },
    apiChecking: false,
    versionsLoading: false,
    providerCount: 3,
  });

  assert.equal(status.phase, "connected");
  assert.equal(status.api, "ready");
  assert.equal(status.providers, "ready");
  assert.equal(status.versions, "ready");
});

test("startup status becomes failed when the sidecar reports an error", () => {
  const status = getSymphonyStartupStatus({
    sidecarReady: false,
    sidecarError: "Port already in use",
    apiHealth: null,
    apiChecking: false,
    versionsLoading: false,
    providerCount: 0,
  });

  assert.equal(status.phase, "failed");
  assert.equal(status.errorMessage, "Port already in use");
  assert.equal(status.sidecar, "failed");
});

test("startup status becomes failed when the API health check reports a disconnection", () => {
  const status = getSymphonyStartupStatus({
    sidecarReady: true,
    sidecarError: null,
    apiHealth: {
      connected: false,
      url: "http://127.0.0.1:8719",
      error: "connection refused",
    },
    apiChecking: false,
    versionsLoading: false,
    providerCount: 0,
  });

  assert.equal(status.phase, "failed");
  assert.equal(status.errorMessage, "connection refused");
  assert.equal(status.api, "failed");
});

test("auto-refresh triggers only when the sidecar becomes ready", () => {
  assert.equal(shouldAutoRefreshOnReady(undefined, true), true);
  assert.equal(shouldAutoRefreshOnReady(null, true), true);
  assert.equal(shouldAutoRefreshOnReady(false, true), true);
  assert.equal(shouldAutoRefreshOnReady(true, true), false);
  assert.equal(shouldAutoRefreshOnReady(null, null), false);
  assert.equal(shouldAutoRefreshOnReady(true, false), false);
});
