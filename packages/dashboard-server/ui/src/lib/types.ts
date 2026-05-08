// Mirrors @org-roam-toolkit/dashboard-server's AllProbes shape.
// Kept inline (not imported) so the UI bundle has zero workspace dep.

export type Probe<T> =
  | { status: 'up'; data: T; probedAt: string }
  | { status: 'down'; error: string; probedAt: string };

export interface DaemonHealth {
  pid: number;
  uptimeSeconds: number;
  loadedFeatures: string[];
}

export interface McpHealth {
  binary: string;
  tools: number;
  serverInfo: { name: string; version: string };
}

export interface RoamConfig {
  dir: string;
  dbPath: string;
  dbExists: boolean;
  dbSize: number;
  subdirs: string[];
}

export interface GraphStats {
  nodes: number;
  edges: number;
  orphans: number;
  tags: number;
}

export interface AllProbes {
  daemon: Probe<DaemonHealth>;
  mcp: Probe<McpHealth>;
  roamConfig: Probe<RoamConfig>;
  graphStats: Probe<GraphStats>;
}
