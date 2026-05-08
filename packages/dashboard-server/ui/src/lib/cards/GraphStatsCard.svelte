<script lang="ts">
  import Card from './Card.svelte';
  import type { Probe, GraphStats } from '../types';

  interface Props { probe: Probe<GraphStats> | null; }
  let { probe }: Props = $props();
</script>

<Card title="Graph Stats" {probe}>
  {#if probe?.status === 'up'}
    <dl>
      <dt>Nodes</dt>    <dd class="num">{probe.data.nodes.toLocaleString()}</dd>
      <dt>Edges</dt>    <dd class="num">{probe.data.edges.toLocaleString()}</dd>
      <dt>Orphans</dt>  <dd class="num">{probe.data.orphans}{probe.data.orphans > 0 ? ' ⚠' : ''}</dd>
      <dt>Tags</dt>     <dd class="num">{probe.data.tags.toLocaleString()}</dd>
    </dl>
  {/if}
</Card>
