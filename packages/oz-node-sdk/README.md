# oz-node-sdk

Generated Node.js SDK for the oz API.

## Install

```bash
bun add @oz/oz-node-sdk
```

or

```bash
npm install @oz/oz-node-sdk
```

## Build

From this directory run:

```bash
OPENAPI_SPEC_PATH=../../target/openapi/openapi.json bun run build
```

The script reads the generated OpenAPI artifact and writes SDK artifacts to `dist/`.

## Publish

```bash
bun run clean
OPENAPI_SPEC_PATH=../../target/openapi/openapi.json bun run build
bun publish --access public
```

`prepublishOnly` is configured to regenerate `dist` automatically before publish.

## Usage

```js
import { OzNodeSdkClient } from '@oz/oz-node-sdk';

const client = new OzNodeSdkClient({
  baseUrl: 'http://localhost:8787',
  apiKey: 'oz_live_xxx',
});

const projects = await client.GET_v2_projects();
```
