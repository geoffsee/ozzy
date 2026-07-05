# Project overview

- **API + web UI** in `crates/oz-api` (Rust Worker with an embedded React UI)
- **CLI** in `crates/oz-cli`
- **Shared types** in `crates/oz-core`
- **Node SDK** in `packages/oz-node-sdk` (generated from OpenAPI)

Session-authenticated requests that mutate data require `GET /api/csrf` then `X-CSRF-Token` on `POST`/`PUT`/`DELETE`.
API-key requests skip CSRF.
