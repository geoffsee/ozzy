use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use anon_telemetry::TelemetryClient;
use serde_json::{json, Value};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

struct TelemetryHandle {
    _runtime: tokio::runtime::Runtime,
    client: Arc<TelemetryClient>,
}

static TELEMETRY: OnceLock<TelemetryHandle> = OnceLock::new();

pub fn init() {
    let Some(endpoint) = events_endpoint() else {
        return;
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(_) => return,
    };

    let client = runtime.block_on(TelemetryClient::new(
        "oz-cli",
        &endpoint,
        Some(env!("CARGO_PKG_VERSION")),
    ));

    let _ = tracing_subscriber::registry()
        .with(
            client
                .tracing_layer()
                .with_event_name_prefix("cli")
                .with_span_context(false),
        )
        .try_init();

    let _ = TELEMETRY.set(TelemetryHandle {
        _runtime: runtime,
        client,
    });
}

pub fn track(event_name: &str, properties: HashMap<String, Value>) {
    let Some(handle) = TELEMETRY.get() else {
        return;
    };

    handle.client.track(event_name, Some(properties));
}

pub fn track_command(command: &str) {
    track(
        "cli_invoked",
        HashMap::from([("command".to_string(), json!(command))]),
    );
}

pub fn report_error(error: &anyhow::Error) {
    if TELEMETRY.get().is_some() {
        tracing::error!(error = %error, "command failed");
    }
}

fn events_endpoint() -> Option<String> {
    std::env::var("OZ_TELEMETRY_ENDPOINT")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var("TELEMETRY_SINK_URL")
                .ok()
                .filter(|value| !value.is_empty())
                .map(|sink| format!("{}/v1/events", sink.trim_end_matches('/')))
        })
}
