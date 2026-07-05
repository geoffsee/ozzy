# oz

Self-hosted secrets management.

- [Quick Start](#quick-start)
- [Documentation](docs/README.md)
- [Project overview](docs/project-overview.md)
- [Tooling and scripts](docs/tooling-and-scripts.md)
- [Web UI usage](docs/web-ui-usage.md)
- [CLI usage](docs/cli-usage.md)
- [HTTP API (`/v2`)](docs/http-api.md)
- [Local development](docs/local-development.md)
- [Deployment process](docs/deployment.md)
- [Wrangler and deployment requirements](docs/wrangler.md)
- [OpenAPI + SDK generation](docs/openapi-and-sdk.md)
- [Versioning strategy](docs/versioning.md)
- [Privacy and telemetry](docs/privacy.md)

For detailed guides (usage, setup, API, deployment, SDK/versioning), start with the documentation index above.

## Quick start

### Installation

Find the latest version for your platform [here](https://github.com/geoffsee/oz/releases/latest)

```bash
curl -L https://github.com/geoffsee/oz/releases/latest/download/oz-<os>-<arch>.tar.gz | tar -xz
chmod +x oz && mv oz /usr/local/bin/oz
```

### Usage

1. Visit the Web UI at `https://your-oz-api.example.com` and authenticate with your Github account. 
2. Configure a project
3. Create an API key for the project
4. Use the API key to authenticate with the CLI
5. Use the CLI to manage secrets (it is also possible to use the Web UI)

```bash
# Login to your deployed oz endpoint
oz auth login --api-key your-api-key --api-url https://your-oz-api.example.com

# Set and then fetch a secret
oz secrets set DATABASE_URL --project my-app "postgres://..."
oz secrets get DATABASE_URL --project my-app
```

## SDK usage

### Node.js SDK

```bash
npm install @oz/oz-node-sdk
```

```js
import { OzNodeSdkClient } from '@oz/oz-node-sdk';

const client = new OzNodeSdkClient({
  baseUrl: 'https://your-oz-api.example.com',
  apiKey: 'oz_live_xxx',
});

const project = 'my-app';

// List secrets in a project
const secretList = await client.v2_list_secrets({ body: { project } });
console.log(secretList);

// Write a secret
await client.v2_write_secret({
  body: {
    project,
    key: 'DATABASE_URL',
    value: 'postgres://...',
  },
});

// Read a secret
const secret = await client.v2_read_secret({ body: { project, key: 'DATABASE_URL' } });
console.log(secret.value);

// Delete a secret
await client.v2_delete_secret({ body: { project, key: 'DATABASE_URL' } });
```
