# Wrangler configuration

`wrangler.toml` is expected in `crates/oz-api` and must configure:

- `name`, `main` (`build/index.js`), and `compatibility_date`.
- `[build].command` for Rust worker build (`worker-build`).

For D1:

- Keep `binding` as `DB` (code expects this name).
- Point `migrations_dir` to `../../migrations`.
- Set `database_name` and `database_id` for your environment.

Runtime vars:

- `OZ_BASE_URL`: public base URL for the deployment.
- `TELEMETRY_SINK_URL`: base URL for the g-telemetry ingestion worker. The web UI is built with `${TELEMETRY_SINK_URL}/v1/events` baked in at deploy time via `build.rs`. Leave unset in `[env.test]` to disable telemetry locally.

For the CLI, set the same variable (or `OZ_TELEMETRY_ENDPOINT` with the full `/v1/events` URL) in your shell to enable anonymous usage telemetry.

`[env.test]` can include values for `OZ_ENV=test`, test GitHub API base and secrets.
OAuth stubs should only apply in test environment.

## Local secrets

`crates/oz-api/.dev.vars` should include:

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `OZ_MASTER_KEY`
- `OZ_API_KEY_PEPPER`
- `D1_DATABASE_ID`

Do not put secrets in `wrangler.toml`.
