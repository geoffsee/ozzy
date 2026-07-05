# Tooling and root scripts

The repository uses a root Bun workspace with orchestrating scripts in `package.json`.

Run these from the repo root:

```bash
bun install
bun run build
bun run api:build
bun run api:dev
bun run openapi
bun run sdk:build
bun run sdk:rebuild
bun run sdk:publish
bun run workspace:install
bun run build:web
bun run dev:web
bun run test:web
bun run test:api
bun run test:cli
bun run test:all
```

Script purpose:
- `build`: builds Rust API and CLI crates.
- `api:build`: builds `crates/oz-api`.
- `api:dev`: runs `wrangler dev -e test` from `crates/oz-api`.
- `api:deploy`: deployment command (documented in [Deployment process](deployment.md)).
- `cli:build` / `cli:install`: build and install the CLI.
- `openapi`: regenerates `target/openapi/openapi.json` via `cargo build -p oz-api`.
- `sdk:build`: generates `packages/oz-node-sdk/dist` from OpenAPI.
- `sdk:rebuild`: regenerates OpenAPI then SDK.
- `sdk:publish`: regenerates OpenAPI then publishes `@oz/oz-node-sdk`.
- `workspace:install`: installs workspace deps for root, `apps/web`, and `packages/oz-node-sdk`.
- `build:web` / `dev:web` / `test:web`: run web client scripts.
- `test:api` / `test:cli` / `test:all`: run Rust tests.
