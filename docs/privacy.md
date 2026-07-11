# Privacy and telemetry

ozzy may send **anonymous usage telemetry** from the Web UI and CLI to help understand how the software is used and where it fails. Telemetry is optional for self-hosted deployments and can be disabled entirely.

Telemetry is collected through [g-telemetry](https://github.com/geoffsee/g-telemetry), a self-hosted analytics sink. The official ozzy deployment sends events to a dedicated ingestion worker; you control whether telemetry is enabled on your own instance via `TELEMETRY_SINK_URL` in `wrangler.toml` (see [Wrangler configuration](wrangler.md)).

## What is collected

When telemetry is enabled, clients send JSON events containing:

| Field | Description |
|---|---|
| `app_id` | Surface identifier (`ozzy-web` or `ozzy-cli`) |
| `instance_id` | Random UUID persisted locally on first use |
| `event_name` | Event type (for example `app_started`, `cli_invoked`) |
| `app_version` | ozzy Web UI or CLI version |
| `client_version` | Telemetry client library version |
| `platform` | Browser platform or operating system |
| `properties` | Event-specific dimensions (see below) |
| `timestamp` | Unix timestamp in milliseconds |

### Web UI events

The browser client may record:

- `app_started` — page load path
- `session_loaded` — authenticated session established
- `auth_logout` — user signed out
- `project_created` — project slug (not display name or secrets)
- `api_key_created` — project slug
- `secret_saved` — project slug (never secret names or values)

### CLI events

The CLI may record:

- `cli_invoked` — subcommand name (for example `secrets set`)
- `cli.error` — error-level failures (message and error metadata only; no secret values)

## What is not collected

Telemetry is designed to stay anonymous:

- No names, email addresses, GitHub usernames, or other account identifiers
- No API keys, secret names, secret values, or project member data
- No IP addresses stored by the ingestion sink
- No cross-site tracking or third-party analytics scripts

Do not put personally identifiable information in telemetry `properties`. ozzy does not intentionally send PII, but misconfiguration of a custom sink is your responsibility.

## When telemetry runs

| Surface | Enabled when | Disabled when |
|---|---|---|
| Web UI | `TELEMETRY_SINK_URL` is set at **build/deploy** time | Variable unset (default for `[env.test]`) |
| CLI | `TELEMETRY_SINK_URL` or `OZZY_TELEMETRY_ENDPOINT` is set in the environment | Variables unset |

Local development and test deployments typically have telemetry off unless you configure it explicitly.

## How to opt out

Telemetry respects standard opt-out signals from g-telemetry:

1. Set `DO_NOT_TRACK=1` in the environment (Web UI and CLI).
2. Set an app-specific override:
   - Web UI: `OZZY_WEB_NO_TELEMETRY=1`
   - CLI: `OZZY_CLI_NO_TELEMETRY=1`
3. Self-host without `TELEMETRY_SINK_URL` so clients never initialize telemetry.
4. For the Web UI only, pass `telemetryEnabled: false` if you embed `@anon-telemetry/client` yourself (not applicable to the stock bundled UI).

## Data storage and access

Events are POSTed to `{TELEMETRY_SINK_URL}/v1/events` and stored in the operator’s g-telemetry deployment (Cloudflare Worker + D1 by default). Access to aggregate reporting endpoints on that sink is restricted to the operator; ozzy itself does not expose telemetry data through the secrets API.

If you operate your own ozzy instance, you choose whether to enable telemetry and where the sink lives. If you use someone else’s hosted ozzy deployment, their privacy policy and telemetry configuration apply.

## Questions

For issues with the telemetry clients or sink, see the [g-telemetry repository](https://github.com/geoffsee/g-telemetry). For ozzy-specific behavior, open an issue on the [ozzy repository](https://github.com/geoffsee/ozzy).
