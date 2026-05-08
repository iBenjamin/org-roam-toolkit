/**
 * MCP Resources for org-roam server.
 *
 * Same probe data the dashboard exposes over HTTP, surfaced here as
 * MCP resources so the model can read it directly.
 *
 * URIs:
 *   health://daemon      → emacs daemon health
 *   health://mcp         → this MCP server's own metadata (no recursion)
 *   config://org-roam    → org-roam configuration
 *   stats://graph        → graph statistics
 */

import {
  probeDaemon,
  probeRoamConfig,
  probeGraphStats,
  runProbe,
  type Probe,
} from '@org-roam-toolkit/emacs';
import type { Resource } from '@modelcontextprotocol/sdk/types.js';

const SERVER_START = Date.now();

export interface McpSelfHealth {
  name: string;
  version: string;
  uptimeSeconds: number;
  tools: number;
}

interface ResourceDef {
  uri: string;
  name: string;
  description: string;
  read(): Promise<Probe<unknown>>;
}

/** Build the resource registry. `toolCount` is needed for self-health. */
export function buildResources(opts: {
  serverName: string;
  serverVersion: string;
  toolCount: number;
}): ResourceDef[] {
  const selfHealth = (): Promise<Probe<McpSelfHealth>> =>
    runProbe(() => ({
      name: opts.serverName,
      version: opts.serverVersion,
      uptimeSeconds: (Date.now() - SERVER_START) / 1000,
      tools: opts.toolCount,
    }));

  return [
    {
      uri: 'health://daemon',
      name: 'Emacs daemon health',
      description: 'Liveness, pid, uptime, loaded features',
      read: probeDaemon,
    },
    {
      uri: 'health://mcp',
      name: 'MCP server self-health',
      description: 'This server\'s own metadata (version, uptime, tool count)',
      read: selfHealth,
    },
    {
      uri: 'config://org-roam',
      name: 'org-roam configuration',
      description: 'org-roam-directory, db path/size, subdirectories',
      read: probeRoamConfig,
    },
    {
      uri: 'stats://graph',
      name: 'org-roam graph stats',
      description: 'Node count, edge count, orphans, tags',
      read: probeGraphStats,
    },
  ];
}

/** Convert a ResourceDef to the MCP-SDK shape used by ListResourcesRequest. */
export function toMcpResource(def: ResourceDef): Resource {
  return {
    uri: def.uri,
    name: def.name,
    description: def.description,
    mimeType: 'application/json',
  };
}
