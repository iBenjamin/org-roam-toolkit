/**
 * Probe the org-roam MCP server by spawning it and doing an
 * `initialize` + `tools/list` JSON-RPC handshake over stdio.
 *
 * Cost: ~1 second per probe (process startup + two roundtrips).
 * Mitigation: results are cached in `ttl-cache` for 5s upstream.
 */

import { spawn } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { runProbe, type Probe } from '@org-roam-toolkit/emacs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// dist/probe-mcp.js → ../../../mcp-servers/org-roam/dist/index.js
const MCP_BINARY = join(
  __dirname,
  '..',
  '..',
  '..',
  'mcp-servers',
  'org-roam',
  'dist',
  'index.js',
);

const PROBE_TIMEOUT_MS = 5_000;

export interface McpHealth {
  binary: string;
  tools: number;
  serverInfo: { name: string; version: string };
}

interface JsonRpcMessage {
  jsonrpc: '2.0';
  id?: number;
  method?: string;
  params?: unknown;
  result?: {
    serverInfo?: { name: string; version: string };
    tools?: { name: string }[];
  };
  error?: { code: number; message: string };
}

export async function probeMcp(): Promise<Probe<McpHealth>> {
  return runProbe(
    () =>
      new Promise<McpHealth>((resolve, reject) => {
        const proc = spawn('node', [MCP_BINARY], {
          stdio: ['pipe', 'pipe', 'pipe'],
        });

        let serverInfo: McpHealth['serverInfo'] | null = null;
        let buf = '';
        let done = false;

        const finish = (err?: Error, ok?: McpHealth): void => {
          if (done) return;
          done = true;
          clearTimeout(timer);
          proc.kill('SIGTERM');
          if (err) reject(err);
          else resolve(ok!);
        };

        const timer = setTimeout(
          () => finish(new Error('mcp probe timed out')),
          PROBE_TIMEOUT_MS,
        );

        proc.on('error', (e: Error) => finish(e));
        proc.on('exit', (code: number | null) => {
          if (!done)
            finish(new Error(`mcp server exited unexpectedly (code ${code ?? '?'})`));
        });

        proc.stdout.on('data', (chunk: Buffer) => {
          buf += chunk.toString('utf8');
          let idx: number;
          while ((idx = buf.indexOf('\n')) >= 0) {
            const line = buf.slice(0, idx);
            buf = buf.slice(idx + 1);
            if (!line.trim()) continue;
            let msg: JsonRpcMessage;
            try {
              msg = JSON.parse(line) as JsonRpcMessage;
            } catch {
              continue; // ignore non-JSON lines
            }
            if (msg.error) {
              finish(new Error(`mcp error: ${msg.error.message}`));
              return;
            }
            if (msg.id === 1 && msg.result?.serverInfo) {
              serverInfo = msg.result.serverInfo;
              proc.stdin.write(
                JSON.stringify({
                  jsonrpc: '2.0',
                  id: 2,
                  method: 'tools/list',
                  params: {},
                }) + '\n',
              );
            } else if (msg.id === 2 && msg.result?.tools) {
              if (!serverInfo) {
                finish(new Error('tools/list responded before initialize'));
                return;
              }
              finish(undefined, {
                binary: MCP_BINARY,
                tools: msg.result.tools.length,
                serverInfo,
              });
            }
          }
        });

        proc.stdin.write(
          JSON.stringify({
            jsonrpc: '2.0',
            id: 1,
            method: 'initialize',
            params: {
              protocolVersion: '2024-11-05',
              capabilities: {},
              clientInfo: { name: 'dashboard-server', version: '0.1.0' },
            },
          }) + '\n',
        );
      }),
  );
}
