# CLI usage

Download the CLI from the latest release:

- https://github.com/geoffsee/ozzy/releases/latest
- Download the binary for your OS/architecture and add `ozzy` to your `PATH`.

```bash
curl -L https://github.com/geoffsee/ozzy/releases/latest/download/ozzy-<os>-<arch>.tar.gz | tar -xz
chmod +x ozzy && mv ozzy /usr/local/bin/ozzy
```

Configure credentials in `~/.config/ozzy/config.toml`:

```bash
ozzy auth login --api-key ozzy_live_... --api-url https://your-ozzy-api.example.com
```

(`OZZY_API_URL` and `OZZY_API_KEY` override saved config for one invocation.)

```bash
ozzy project list
ozzy secrets list --project my-app
ozzy secrets get DATABASE_URL --project my-app
ozzy secrets set DATABASE_URL --project my-app "postgres://..."
echo -n "super-secret" | ozzy secrets set API_TOKEN --project my-app --from-stdin
ozzy secrets delete OLD_KEY --project my-app
ozzy auth logout
```

API keys must start with `ozzy_live_` and be at least 32 characters after the prefix.
