<script lang="ts">
  import Card from './Card.svelte';
  import type { Probe, McpHealth } from '../types';

  interface Props { probe: Probe<McpHealth> | null; }
  let { probe }: Props = $props();
</script>

<Card title="MCP Server" {probe}>
  {#if probe?.status === 'up'}
    <dl>
      <dt>Server</dt>   <dd>{probe.data.serverInfo.name}@{probe.data.serverInfo.version}</dd>
      <dt>Tools</dt>    <dd class="num">{probe.data.tools}</dd>
      <dt>Binary</dt>   <dd>{probe.data.binary}</dd>
    </dl>
  {/if}
</Card>
