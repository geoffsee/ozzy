# Local development

**Prerequisites:** Rust, Bun, Wrangler.

1. Set up `crates/ozzy-api/wrangler.toml` and `crates/ozzy-api/.dev.vars`.
2. Apply local D1 migrations:

```bash
cd crates/ozzy-api
wrangler d1 migrations apply ozzy-test --local -e test
```

3. Start local Worker in test mode:

```bash
wrangler dev -e test
```

4. Open `http://localhost:8787`, sign in, create a project and key, then run:

```bash
ozzy auth login --api-key ozzy_live_... --api-url http://localhost:8787
ozzy secrets set MY_KEY --project my-app "hello"
```

For UI iteration:

```bash
cd apps/web
bun run dev
```

For production-like local OAuth, run `wrangler dev` without `-e test` and use real GitHub OAuth credentials.
