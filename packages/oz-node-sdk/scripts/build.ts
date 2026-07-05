#!/usr/bin/env bun
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import Oas from 'oas';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(__dirname, '..');
const repoRoot = path.resolve(packageRoot, '..', '..');
const targetDefault = path.resolve(repoRoot, 'target', 'openapi', 'openapi.json');
const specPath = path.resolve(
  packageRoot,
  process.env.OPENAPI_SPEC_PATH ?? path.relative(packageRoot, targetDefault)
);
if (!fs.existsSync(specPath)) {
  throw new Error(
    'OPENAPI spec not found at ' +
      specPath +
      '. Run `cargo build -p oz-api` (or set OPENAPI_SPEC_PATH) before building the SDK.'
  );
}

const outDir = path.join(packageRoot, 'dist');
const outSpecPath = path.join(outDir, 'openapi.json');
const outIndexPath = path.join(outDir, 'index.js');
const outTypesPath = path.join(outDir, 'index.d.ts');

const rawSpec = fs.readFileSync(specPath, 'utf8');
const spec = JSON.parse(rawSpec);
const oas = Oas.init(spec);
const paths = oas.getPaths();

const endpointEntries = [];
const seenNames = new Set();

const toSafeName = (value) => {
  const normalized = String(value)
    .replace(/[^a-zA-Z0-9_$]/g, '_')
    .replace(/^([0-9])/, '_$1')
    .replace(/_+/g, '_')
    .replace(/^_|_$/g, '');

  const base = normalized || 'operation';
  let name = base;
  let counter = 2;

  while (seenNames.has(name)) {
    name = `${base}_${counter++}`;
  }

  seenNames.add(name);
  return name;
};

for (const [routePath, methods] of Object.entries(paths)) {
  for (const [method, operation] of Object.entries(methods)) {
    if (!operation || typeof operation !== 'object') {
      continue;
    }

    const opId =
      operation.operationId ??
      [method.toUpperCase(), routePath.replace(/[\/{}]/g, '_')]
        .join('_')
        .replace(/_+/g, '_')
        .replace(/^_|_$/g, '');

    endpointEntries.push({
      path: routePath,
      method: method.toUpperCase(),
      operationId: opId,
      safeName: toSafeName(opId),
      summary: operation.summary ?? '',
      tags: Array.isArray(operation.tags) ? operation.tags : [],
    });
  }
}

endpointEntries.sort((left, right) => {
  if (left.path !== right.path) {
    return left.path.localeCompare(right.path);
  }

  return left.method.localeCompare(right.method);
});

const methodDefs = endpointEntries
  .map(
    ({ safeName, method, path: routePath }) =>
      `  async ${safeName}(options = {}) {\n    return this.#request(${JSON.stringify(
        method
      )}, ${JSON.stringify(routePath)}, options);\n  }\n`
  )
  .join('\n');

const endpointList = JSON.stringify(
  endpointEntries.map(({ method, path, operationId, summary, tags }) => ({
    method,
    path,
    operationId,
    summary,
    tags,
  })),
  null,
  2
);

const indexSource = `"use strict";

export const openapiEndpoints = ${endpointList};

const query = (searchParams = {}, search = new URLSearchParams()) => {
  for (const [key, value] of Object.entries(searchParams)) {
    if (value === undefined) {
      continue;
    }

    if (Array.isArray(value)) {
      for (const item of value) {
        search.append(key, String(item));
      }

      continue;
    }

    search.append(key, String(value));
  }

  return search;
};

export class OzNodeSdkClient {
  #baseUrl;
  #apiKey;
  #fetch;
  #defaultHeaders;

  constructor({
    baseUrl = '',
    apiKey,
    fetch: fetchImpl,
    headers = {},
  } = {}) {
    if (typeof fetchImpl !== 'undefined' && typeof fetchImpl !== 'function') {
      throw new TypeError('fetch option must be a function');
    }

    const resolvedFetch = fetchImpl ?? globalThis.fetch;
    if (typeof resolvedFetch !== 'function') {
      throw new Error('No fetch implementation available in this environment');
    }

    this.#baseUrl = baseUrl.replace(/\/$/, '');
    this.#apiKey = apiKey ?? null;
    this.#fetch = resolvedFetch;
    this.#defaultHeaders = { ...headers };
  }

  setApiKey(apiKey) {
    this.#apiKey = apiKey;
  }

  #url(routePath, pathParams = {}, queryParams = {}) {
    const path = routePath.replace(/\{([^}/]+)\}/g, (_match, key) => {
        if (!(key in pathParams)) {
        throw new Error('Missing path parameter: ' + key);
      }

      return encodeURIComponent(String(pathParams[key]));
    });

    const url = new URL(this.#baseUrl + path);
    if (queryParams && Object.keys(queryParams).length > 0) {
      query(url.searchParams, queryParams);
    }

    return url;
  }

  async #request(method, routePath, options = {}) {
    const {
      pathParams = {},
      query: queryParams = {},
      body,
      headers = {},
      signal,
    } = options ?? {};

    const url = this.#url(routePath, pathParams, queryParams);
    const requestHeaders = {
      ...this.#defaultHeaders,
      ...headers,
      Accept: 'application/json',
    };

    if (this.#apiKey) {
      requestHeaders.Authorization = 'Bearer ' + this.#apiKey;
    }

    const init = {
      method,
      headers: requestHeaders,
      signal,
    };

    if (body !== undefined && !['GET', 'HEAD'].includes(method)) {
      requestHeaders['Content-Type'] = requestHeaders['Content-Type'] ?? 'application/json';
      init.body = typeof body === 'string' ? body : JSON.stringify(body);
    }

    const response = await this.#fetch(url.href, init);
    if (!response.ok) {
      const responseText = await response.text();
      throw new Error('API request failed (' + response.status + '): ' + (responseText || 'no response body'));
    }

    if (response.status === 204) {
      return null;
    }

    const responseType = response.headers.get('content-type') || '';
    if (responseType.includes('application/json')) {
      return response.json();
    }

    return response.text();
  }

${methodDefs}
}
`;

const indexTypes = `export interface OzNodeSdkOptions {
  baseUrl?: string;
  apiKey?: string;
  headers?: Record<string, string>;
  fetch?: typeof fetch;
}

export interface OzNodeRequestOptions {
  pathParams?: Record<string, string | number>;
  query?: Record<string, string | number | boolean | Array<string | number | boolean>>;
  body?: unknown;
  headers?: Record<string, string>;
  signal?: AbortSignal;
}

export type OzNodeSdkEndpoint = {
  method: string;
  path: string;
  operationId: string;
  summary?: string;
  tags?: string[];
};

export const openapiEndpoints: OzNodeSdkEndpoint[];

export class OzNodeSdkClient {
  constructor(options?: OzNodeSdkOptions);
  setApiKey(apiKey: string | null): void;
${endpointEntries
  .map(({ safeName }) => `  ${safeName}(options?: OzNodeRequestOptions): Promise<unknown>;`)
  .join('\n')}
}
`;

fs.mkdirSync(outDir, { recursive: true });
fs.copyFileSync(specPath, outSpecPath);
fs.writeFileSync(outIndexPath, indexSource, 'utf8');
fs.writeFileSync(outTypesPath, indexTypes, 'utf8');

console.log(`built oz-node-sdk from ${specPath}`);
console.log(`output: ${outDir}`);
