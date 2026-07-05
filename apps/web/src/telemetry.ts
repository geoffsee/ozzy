import { TelemetryClient } from "@anon-telemetry/client";

const ENDPOINT = process.env.BUN_PUBLIC_TELEMETRY_ENDPOINT ?? "";
const APP_VERSION = "0.1.0";

let client: TelemetryClient | null = null;

export function initTelemetry() {
  if (!ENDPOINT) {
    return;
  }

  client = new TelemetryClient({
    appId: "oz-web",
    endpoint: ENDPOINT,
    appVersion: APP_VERSION,
  });

  client.track("app_started", { path: window.location.pathname });
}

export function track(eventName: string, properties: Record<string, unknown> = {}) {
  client?.track(eventName, properties);
}
