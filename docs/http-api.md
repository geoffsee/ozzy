# HTTP API (`/v2`)

Use an API key as a bearer token.

```bash
curl -H "Authorization: Bearer ozzy_live_..." \
  https://your-ozzy-api.example.com/v2/projects

curl -X POST -H "Authorization: Bearer ozzy_live_..." \
  -H "Content-Type: application/json" \
  -d '{"project":"my-app","key":"DATABASE_URL"}' \
  https://your-ozzy-api.example.com/v2/secrets/read

curl -X PUT -H "Authorization: Bearer ozzy_live_..." \
  -H "Content-Type: application/json" \
  -d '{"project":"my-app","key":"DATABASE_URL","value":"postgres://..."}' \
  https://your-ozzy-api.example.com/v2/secrets/write
```

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/v2/projects` | List projects the key can access |
| `POST` | `/v2/secrets/list` | List secret metadata |
| `POST` | `/v2/secrets/read` | Read a secret value |
| `PUT` | `/v2/secrets/write` | Create or update a secret |
| `POST` | `/v2/secrets/delete` | Delete a secret |

Legacy `/v1/projects/{slug}/secrets...` routes still exist for older clients.
