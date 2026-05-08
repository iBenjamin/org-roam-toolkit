<script lang="ts">
  import Card from './Card.svelte';
  import type { Probe, DaemonHealth } from '../types';
  import { formatUptime } from '../format';

  interface Props { probe: Probe<DaemonHealth> | null; }
  let { probe }: Props = $props();
</script>

<Card title="Emacs Daemon" {probe}>
  {#if probe?.status === 'up'}
    <dl>
      <dt>PID</dt>      <dd class="num">{probe.data.pid}</dd>
      <dt>Uptime</dt>   <dd class="num">{formatUptime(probe.data.uptimeSeconds)}</dd>
      <dt>Loaded</dt>   <dd>{probe.data.loadedFeatures.join(', ') || '(none)'}</dd>
    </dl>
  {/if}
</Card>
