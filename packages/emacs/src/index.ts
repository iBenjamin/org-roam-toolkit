export {
  evalElisp,
  parseElispResult,
  buildKeywordArgs,
  escapeElispString,
  quoteElispString,
  isDaemonRunning,
  type EvalOptions,
} from './emacs-client.js';

export {
  runProbe,
  probeDaemon,
  probeRoamConfig,
  probeGraphStats,
  type Probe,
  type DaemonHealth,
  type RoamConfig,
  type GraphStats,
} from './probes/index.js';
