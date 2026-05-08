/**
 * Emacs client wrapper for executing elisp via emacsclient.
 *
 * Higher-level callers (MCP servers, scripts) should prefer this over
 * spawning emacsclient directly. The {pkg} option auto-loads a project's
 * elisp package by convention: --pkg=NAME loads from elisp/NAME/.
 */

import { execFileSync, type ExecFileSyncOptions } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// dist/emacs-client.js → ../bin/emacs-eval
const PKG_ROOT = join(__dirname, '..');
const EMACS_EVAL = join(PKG_ROOT, 'bin', 'emacs-eval');

export interface EvalOptions {
  /** Elisp package to auto-load (looked up under <emacs>/elisp/<pkg>/). */
  pkg?: string;
  /** Override the load path; default is <emacs>/elisp/<pkg>. */
  loadPath?: string;
  /** Timeout in milliseconds. */
  timeout?: number;
}

const ELISP_UNESCAPE: Record<string, string> = {
  '"': '"',
  '\\': '\\',
  n: '\n',
  t: '\t',
  r: '\r',
};

function unescapeElispString(s: string): string {
  return s.replace(/\\(.)/g, (_, ch: string) => ELISP_UNESCAPE[ch] ?? ch);
}

/** Parse a single emacsclient stdout line into a JS value. */
export function parseElispResult(output: string): unknown {
  const trimmed = output.trim();

  if (trimmed === 'nil') return null;
  if (trimmed === 't') return true;

  if (trimmed.startsWith('"') && trimmed.endsWith('"')) {
    return unescapeElispString(trimmed.slice(1, -1));
  }

  if (/^-?\d+(\.\d+)?$/.test(trimmed)) {
    return Number(trimmed);
  }

  // Lists, plists, alists — return raw; callers using
  // `claude-skill-json-encode` should JSON.parse the inner string.
  return trimmed;
}

/** Evaluate an elisp expression via the emacs-eval wrapper. */
export function evalElisp(expr: string, opts: EvalOptions = {}): unknown {
  const args: string[] = [];
  if (opts.pkg) args.push(`--pkg=${opts.pkg}`);
  if (opts.loadPath) args.push(`--load-path=${opts.loadPath}`);
  args.push(expr);

  const execOpts: ExecFileSyncOptions = {
    encoding: 'utf-8',
    timeout: opts.timeout ?? 30_000,
    stdio: ['pipe', 'pipe', 'pipe'],
  };

  try {
    const result = execFileSync(EMACS_EVAL, args, execOpts);
    return parseElispResult(result.toString());
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    const stderr =
      (error as { stderr?: Buffer | string }).stderr?.toString().trim() ?? '';
    throw new Error(`emacs-eval failed: ${msg}${stderr ? '\n' + stderr : ''}`);
  }
}

/** Escape a string for safe inclusion inside an elisp double-quoted literal. */
export function escapeElispString(s: string): string {
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}

/** Wrap a JS string as an elisp string literal: foo → "foo" with escaping. */
export function quoteElispString(s: string): string {
  return `"${escapeElispString(s)}"`;
}

/** camelCase → :kebab-case keyword (handles leading capital). */
function toKeyword(key: string): string {
  const kebab = key
    .replace(/([A-Z])/g, '-$1')
    .toLowerCase()
    .replace(/^-/, '');
  return `:${kebab}`;
}

/**
 * Build elisp keyword arguments from a JS object.
 *
 * camelCase keys → :kebab-case keywords.
 * Strings are quoted, booleans → t/nil, arrays → quoted lists,
 * objects → quoted alists with string keys.
 */
export function buildKeywordArgs(args: Record<string, unknown>): string {
  const parts: string[] = [];

  for (const [key, value] of Object.entries(args)) {
    if (value === undefined || value === null) continue;

    const kw = toKeyword(key);

    if (typeof value === 'string') {
      parts.push(`${kw} ${quoteElispString(value)}`);
    } else if (typeof value === 'boolean') {
      parts.push(`${kw} ${value ? 't' : 'nil'}`);
    } else if (typeof value === 'number') {
      parts.push(`${kw} ${value}`);
    } else if (Array.isArray(value)) {
      const items = value
        .map((v) =>
          typeof v === 'string' ? quoteElispString(v) : String(v),
        )
        .join(' ');
      parts.push(`${kw} '(${items})`);
    } else if (typeof value === 'object') {
      const items = Object.entries(value as Record<string, string>)
        .map(
          ([k, v]) =>
            `(${quoteElispString(k)} . ${quoteElispString(String(v))})`,
        )
        .join(' ');
      parts.push(`${kw} '(${items})`);
    }
  }

  return parts.join(' ');
}

/** Quick health check: is the Emacs daemon reachable? */
export function isDaemonRunning(): boolean {
  try {
    execFileSync('emacsclient', ['--eval', 't'], {
      encoding: 'utf-8',
      timeout: 5_000,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    return true;
  } catch {
    return false;
  }
}
