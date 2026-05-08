import { evalElisp } from '../emacs-client.js';
import { runProbe, type Probe } from './types.js';

export interface GraphStats {
  nodes: number;
  edges: number;
  orphans: number;
  tags: number;
}

export async function probeGraphStats(): Promise<Probe<GraphStats>> {
  return runProbe(() => {
    const raw = evalElisp('(org-roam-skill-probe-graph-stats)', {
      pkg: 'org-roam-skill',
    });
    if (typeof raw !== 'string') {
      throw new Error(`unexpected probe payload: ${String(raw)}`);
    }
    return JSON.parse(raw) as GraphStats;
  });
}
