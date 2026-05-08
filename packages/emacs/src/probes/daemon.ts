import { evalElisp, isDaemonRunning } from '../emacs-client.js';
import { runProbe, type Probe } from './types.js';

export interface DaemonHealth {
  pid: number;
  uptimeSeconds: number;
  loadedFeatures: string[];
}

export async function probeDaemon(): Promise<Probe<DaemonHealth>> {
  return runProbe(() => {
    if (!isDaemonRunning()) {
      throw new Error('emacs daemon not reachable (emacsclient -e t failed)');
    }
    const raw = evalElisp('(claude-skill-probe-daemon)', {
      pkg: 'claude-skill-base',
    });
    if (typeof raw !== 'string') {
      throw new Error(`unexpected probe payload: ${String(raw)}`);
    }
    return JSON.parse(raw) as DaemonHealth;
  });
}
