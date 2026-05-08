import { evalElisp } from '../emacs-client.js';
import { runProbe, type Probe } from './types.js';

export interface RoamConfig {
  dir: string;
  dbPath: string;
  dbExists: boolean;
  dbSize: number;
  subdirs: string[];
}

export async function probeRoamConfig(): Promise<Probe<RoamConfig>> {
  return runProbe(() => {
    const raw = evalElisp('(org-roam-skill-probe-config)', {
      pkg: 'org-roam-skill',
    });
    if (typeof raw !== 'string') {
      throw new Error(`unexpected probe payload: ${String(raw)}`);
    }
    return JSON.parse(raw) as RoamConfig;
  });
}
