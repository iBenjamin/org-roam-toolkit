#!/usr/bin/env node
/**
 * dashboard-server — local HTTP server backing the org-roam-toolkit dashboard UI.
 *
 *   dashboard-serve [--port=N] [--host=H]
 *
 * Defaults: 127.0.0.1:9876. Bind only to loopback unless the user explicitly
 * overrides --host. There is no auth.
 */

import { Hono } from 'hono';
import { serveStatic } from '@hono/node-server/serve-static';
import { serve } from '@hono/node-server';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { existsSync } from 'node:fs';
import { getAllProbes, getProbe, type ProbeName } from './probes-bundle.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// dist/server.js → ../ui/dist
const UI_DIST = join(__dirname, '..', 'ui', 'dist');

interface ParsedArgs {
  port: number;
  host: string;
  help: boolean;
}

function parseArgs(argv: string[]): ParsedArgs {
  const args: ParsedArgs = {
    port: Number(process.env.DASHBOARD_PORT ?? 9876),
    host: process.env.DASHBOARD_HOST ?? '127.0.0.1',
    help: false,
  };
  for (const a of argv) {
    if (a === '--help' || a === '-h') args.help = true;
    else if (a.startsWith('--port=')) args.port = Number(a.slice('--port='.length));
    else if (a.startsWith('--host=')) args.host = a.slice('--host='.length);
  }
  return args;
}

function usage(): void {
  process.stdout.write(
    `dashboard-serve — org-roam-toolkit observability dashboard\n` +
      `\n` +
      `Usage:\n` +
      `  dashboard-serve [--port=N] [--host=H]\n` +
      `\n` +
      `Defaults:\n` +
      `  --port  9876   (env: DASHBOARD_PORT)\n` +
      `  --host  127.0.0.1   (env: DASHBOARD_HOST; do not bind 0.0.0.0 — no auth)\n`,
  );
}

const PROBE_NAMES: readonly ProbeName[] = [
  'daemon',
  'mcp',
  'roamConfig',
  'graphStats',
];

const PROBE_NAME_BY_SLUG: Record<string, ProbeName> = {
  daemon: 'daemon',
  mcp: 'mcp',
  'roam-config': 'roamConfig',
  'graph-stats': 'graphStats',
};

export function buildApp(): Hono {
  const app = new Hono();

  app.get('/api/health', async (c) => c.json(await getAllProbes()));

  app.get('/api/health/:name', async (c) => {
    const name = PROBE_NAME_BY_SLUG[c.req.param('name')];
    if (!name) {
      return c.json(
        { error: `unknown probe; valid: ${Object.keys(PROBE_NAME_BY_SLUG).join(', ')}` },
        404,
      );
    }
    return c.json(await getProbe(name));
  });

  if (existsSync(UI_DIST)) {
    app.use('/*', serveStatic({ root: UI_DIST }));
  } else {
    app.get('/', (c) =>
      c.text(
        `UI bundle not built. Run:\n  npm -w @org-roam-toolkit/dashboard-server run build:ui\n`,
        503,
      ),
    );
  }

  return app;
}

export function main(): void {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    usage();
    return;
  }
  const app = buildApp();
  const { port, host } = args;
  serve({ fetch: app.fetch, port, hostname: host });
  process.stdout.write(
    `dashboard-server listening on http://${host}:${port} (probes: ${PROBE_NAMES.join(', ')})\n`,
  );
}

const isMain =
  process.argv[1] !== undefined &&
  fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) main();
