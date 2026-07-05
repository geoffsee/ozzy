# CLI usage

Download the CLI from the latest release:

- https://github.com/geoffsee/oz/releases/latest
- Download the binary for your OS/architecture and add `oz` to your `PATH`.

```bash
curl -L https://github.com/geoffsee/oz/releases/latest/download/oz-<os>-<arch>.tar.gz | tar -xz
chmod +x oz && mv oz /usr/local/bin/oz
```

Configure credentials in `~/.config/oz/config.toml`:

```bash
oz auth login --api-key oz_live_... --api-url https://your-oz-api.example.com
```

(`OZ_API_URL` and `OZ_API_KEY` override saved config for one invocation.)

```bash
oz project list
oz secrets list --project my-app
oz secrets get DATABASE_URL --project my-app
oz secrets set DATABASE_URL --project my-app "postgres://..."
echo -n "super-secret" | oz secrets set API_TOKEN --project my-app --from-stdin
oz secrets delete OLD_KEY --project my-app
oz auth logout
```

API keys must start with `oz_live_` and be at least 32 characters after the prefix.
