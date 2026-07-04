## oz

This repo includes a Cloudflare Worker API in `crates/oz-api`.

### `wrangler.toml` requirements

The Worker is configured by `crates/oz-api/wrangler.toml`. These values must be correct for your environment:

- `name`: Worker name.
- `main`: built Worker entrypoint (`build/index.js`).
- `compatibility_date`: Cloudflare compatibility date.
- `[build].command`: Rust Worker build command (`worker-build`).

#### D1 binding

Under `[[d1_databases]]`:

- `binding` should stay `DB` (code expects this binding name).
- `database_name` should match your D1 database.
- `database_id` must be your real D1 database UUID.
- `migrations_dir` should point to `../../migrations`.

#### Runtime vars

Under `[vars]`:

- `OZ_BASE_URL`: public base URL for this API deployment.

#### Test environment

`[env.test]` and related sections define local test values (`OZ_TEST_MODE`, test GitHub API base, fake test D1 id, and test secrets).

### Secrets and local dev values

Do **not** put secrets in `wrangler.toml` for normal environments. Use secrets / dev vars instead.

For local development, `crates/oz-api/.dev.vars` should contain:

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `OZ_MASTER_KEY` (base64 key material)
- `OZ_API_KEY_PEPPER`
- `D1_DATABASE_ID`

### Notes

- `wrangler.toml` is gitignored at repo root, so each developer can keep local/project-specific values.
- Keep `binding = "DB"` unchanged unless you also update the Worker code.