# Project overview

- **API + web UI** in `crates/ozzy-api` (Rust Worker with an embedded React UI)
- **CLI** in `crates/ozzy-cli`
- **Shared types** in `crates/ozzy-core`
- **Node SDK** in `packages/ozzy-node-sdk` (generated from OpenAPI)

Session-authenticated requests that mutate data require `GET /api/csrf` then `X-CSRF-Token` on `POST`/`PUT`/`DELETE`.
API-key requests skip CSRF.
