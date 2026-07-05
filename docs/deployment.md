# Deployment process

Deployment is managed separately from local usage.

## API deployment

From `crates/oz-api`:

```bash
wrangler d1 migrations apply oz --remote
wrangler deploy
```

Or from the repo root:

```bash
bun run api:deploy
```

## Environment and secrets

Deploying requires:

- `crates/oz-api/wrangler.toml` configured for the target environment.
- Required runtime values in project secrets (not checked into source control).
- A valid production `OZ_BASE_URL`.

## SDK publish

Release the generated SDK from the repo root:

```bash
bun run sdk:publish
```

Publishing uses the latest regenerated OpenAPI artifact and runs from the package scripts.
