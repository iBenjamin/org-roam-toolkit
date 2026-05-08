/**
 * Single source of truth for the four health probes.
 * Both the HTTP API (server.ts) and MCP resources (mcp-servers/org-roam)
 * call into this file via `probeAll()` or individual probes.
 */

import {
  probeDaemon,
  probeRoamConfig,
  probeGraphStats,
  type Probe,
  type DaemonHealth,
  type RoamConfig,
  type GraphStats,
} from '@org-roam-toolkit/emacs';
import { probeMcp, type McpHealth } from './probe-mcp.js';
import { TtlCache } from './cache.js';

export type ProbeName = 'daemon' | 'mcp' | 'roamConfig' | 'graphStats';

export interface AllProbes {
  daemon: Probe<DaemonHealth>;
  mcp: Probe<McpHealth>;
  roamConfig: Probe<RoamConfig>;
  graphStats: Probe<GraphStats>;
}

const CACHE_TTL_MS = 5_000;
const cache = new TtlCache<unknown>(CACHE_TTL_MS);

const probes: { [K in ProbeName]: () => Promise<AllProbes[K]> } = {
  daemon: probeDaemon,
  mcp: probeMcp,
  roamConfig: probeRoamConfig,
  graphStats: probeGraphStats,
};

export async function getProbe<K extends ProbeName>(
  name: K,
): Promise<AllProbes[K]> {
  return cache.get(name, probes[name] as () => Promise<unknown>) as Promise<
    AllProbes[K]
  >;
}

export async function getAllProbes(): Promise<AllProbes> {
  const [daemon, mcp, roamConfig, graphStats] = await Promise.all([
    getProbe('daemon'),
    getProbe('mcp'),
    getProbe('roamConfig'),
    getProbe('graphStats'),
  ]);
  return { daemon, mcp, roamConfig, graphStats };
}
