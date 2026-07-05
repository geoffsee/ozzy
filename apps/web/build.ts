#!/usr/bin/env bun
import { existsSync } from "fs";
import { mkdir, readFile, rm, writeFile } from "fs/promises";
import path from "path";

const DEFAULT_OUTFILE = path.resolve("dist", "index.html");

const log = (...args: unknown[]) => console.log("[build]", ...args);

function resolveTelemetryEventsEndpoint(): string {
  const explicit = process.env.BUN_PUBLIC_TELEMETRY_ENDPOINT?.trim();
  if (explicit) {
    return explicit;
  }

  const sink = process.env.TELEMETRY_SINK_URL?.trim();
  if (sink) {
    return `${sink.replace(/\/$/, "")}/v1/events`;
  }

  return process.env.OZ_TELEMETRY_ENDPOINT?.trim() ?? "";
}

function parseOutfile(argv: string[]): string {
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--outfile") {
      return path.resolve(argv[i + 1] ?? DEFAULT_OUTFILE);
    }
    if (arg?.startsWith("--outfile=")) {
      return path.resolve(arg.slice("--outfile=".length));
    }
  }
  return DEFAULT_OUTFILE;
}

const outfile = parseOutfile(process.argv.slice(2));
const outdir = path.resolve("dist");

log("Starting single-file build");
log("Output file:", outfile);
log("Build directory:", outdir);

if (existsSync(outdir)) {
  log("Removing existing build directory");
  await rm(outdir, { recursive: true, force: true });
}

log("Running Bun.build()");

const build = await Bun.build({
  entrypoints: [path.resolve("src", "index.html")],
  outdir,
  minify: true,
  target: "browser",
  sourcemap: "none",
  conditions: ["bun", "import", "default"],
  define: {
    "process.env.NODE_ENV": JSON.stringify("production"),
    "process.env.BUN_PUBLIC_TELEMETRY_ENDPOINT": JSON.stringify(
      resolveTelemetryEventsEndpoint(),
    ),
  },
});

if (!build.success) {
  log("Build failed");
  throw new Error(
      `Single-file web build failed:\n${build.logs
          .map(log => log.message)
          .join("\n")}`,
  );
}

log("Build completed successfully");

const generatedHtmlPath = path.join(outdir, "index.html");
log("Reading generated HTML:", generatedHtmlPath);

const generatedHtml = await readFile(generatedHtmlPath, "utf8");

const readFileWithRetry = async (filePath: string, attempts = 5) => {
  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      log(`Reading asset (${attempt}/${attempts}):`, filePath);
      return await readFile(filePath, "utf8");
    } catch (error) {
      if (
          !(
              error instanceof Error
          ) ||
          !("code" in error) ||
          (error as NodeJS.ErrnoException).code !== "ENOENT" ||
          attempt === attempts
      ) {
        log("Failed to read asset:", filePath);
        throw error;
      }

      log("Asset not ready yet, retrying...");
      await Bun.sleep(20);
    }
  }

  throw new Error(`Unable to read generated asset: ${filePath}`);
};

const inlineTag = async (
    html: string,
    regex: RegExp,
    toTag: (content: string) => string,
    kind: string,
) => {
  const matches = [...html.matchAll(regex)] as RegExpMatchArray[];
  let result = "";
  let cursor = 0;

  for (const match of matches) {
    const [fullMatch, srcOrHref] = match;
    const index = match.index ?? 0;

    result += html.slice(cursor, index);

    const assetPath = path.resolve(path.dirname(generatedHtmlPath), srcOrHref);

    log(`Inlining ${kind}:`, srcOrHref);

    const content = await readFileWithRetry(assetPath);

    log(
        `Embedded ${kind}:`,
        srcOrHref,
        `(${content.length.toLocaleString()} bytes)`,
    );

    result += toTag(content);
    cursor = index + fullMatch.length;
  }

  result += html.slice(cursor);

  const count = matches.length;

  log(`Inlined ${count} ${kind}${count === 1 ? "" : "s"}`);

  return result;
};

const escapeInlineScript = (js: string) => js.replace(/<\/script/gi, "<\\/script");

let htmlText = generatedHtml;

log("Inlining CSS");

htmlText = await inlineTag(
    htmlText,
    /<link[^>]+rel=["']stylesheet["'][^>]+href=["']([^"']+)["'][^>]*>/gi,
    css => `<style>${css}</style>`,
    "stylesheet",
);

log("Inlining JavaScript");

htmlText = await inlineTag(
    htmlText,
    /<script[^>]+src=["']([^"']+)["'][^>]*><\/script>/gi,
    js => `<script>${escapeInlineScript(js)}</script>`,
    "script",
);

log("Verifying output contains no external assets");

if (
    /<script[^>]+src=|<link[^>]+rel=["']stylesheet["']/i.test(htmlText)
) {
  throw new Error(
      "Single-file web build failed: generated output still references external script or stylesheet files",
  );
}

log(
    "Final HTML size:",
    `${(htmlText.length / 1024).toFixed(1)} KiB`,
);

await mkdir(path.dirname(outfile), { recursive: true });

log("Writing output:", outfile);

await writeFile(outfile, htmlText, "utf8");

log("Done");

console.log(`✅ Wrote single-file client bundle to ${outfile}`);