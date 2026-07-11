# Versioning strategy

Use semantic versioning (`MAJOR.MINOR.PATCH`) for the published Node SDK.

- `PATCH`: bugfixes, refactors, or generated output updates with no API contract change.
- `MINOR`: non-breaking additions (new endpoints, request/response fields, or headers).
- `MAJOR`: breaking changes (route/path/method removal, required shape changes, auth behavior changes).

Release flow:

1. Run `bun run sdk:rebuild`.
2. Update `packages/ozzy-node-sdk/package.json` version.
3. Publish with `bun --cwd packages/ozzy-node-sdk publish --access public`.
4. Tag release (example: `v0.1.1`) and include a short changelog note.
