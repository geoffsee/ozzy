# OpenAPI and Node SDK generation

OpenAPI is generated on every `oz-api` build to:

- `target/openapi/openapi.json` (or under `$CARGO_TARGET_DIR`)

Build it with:

```bash
bun run openapi
```

Generate SDK from the artifact:

```bash
bun run sdk:build
```

Or regenerate both in one step:

```bash
bun run sdk:rebuild
```

Publish the SDK:

```bash
bun run sdk:publish
```
