<script lang="ts">
  import Card from './Card.svelte';
  import type { Probe, RoamConfig } from '../types';
  import { formatBytes } from '../format';

  interface Props { probe: Probe<RoamConfig> | null; }
  let { probe }: Props = $props();
</script>

<Card title="org-roam Config" {probe}>
  {#if probe?.status === 'up'}
    <dl>
      <dt>Directory</dt>  <dd>{probe.data.dir}</dd>
      <dt>DB</dt>         <dd>{probe.data.dbPath} {probe.data.dbExists ? `(${formatBytes(probe.data.dbSize)})` : '⚠ missing'}</dd>
      <dt>Subdirs</dt>    <dd>{probe.data.subdirs.join(', ') || '(none)'}</dd>
    </dl>
  {/if}
</Card>
