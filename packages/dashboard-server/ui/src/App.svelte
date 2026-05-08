<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { AllProbes } from './lib/types';
  import DaemonCard from './lib/cards/DaemonCard.svelte';
  import McpCard from './lib/cards/McpCard.svelte';
  import RoamConfigCard from './lib/cards/RoamConfigCard.svelte';
  import GraphStatsCard from './lib/cards/GraphStatsCard.svelte';

  const POLL_MS = 5_000;

  let probes = $state<AllProbes | null>(null);
  let lastError = $state<string | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;

  async function refresh(): Promise<void> {
    try {
      const r = await fetch('/api/health');
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      probes = (await r.json()) as AllProbes;
      lastError = null;
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    void refresh();
    timer = setInterval(refresh, POLL_MS);
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
  });
</script>

<svelte:head>
  <style>
    :root {
      color-scheme: dark;
      --bg:        #0d0d0d;
      --fg:        #e5e5e5;
      --border:    #2a2a2a;
      --card-bg:   #161616;
      --muted:     #888;
    }
    *, *::before, *::after { box-sizing: border-box; }
    body { margin: 0; background: var(--bg); color: var(--fg); font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif; }
  </style>
</svelte:head>

<main>
  <header class="page-header">
    <h1>claude-skills</h1>
    <span class="poll-info">refresh every {POLL_MS / 1000}s</span>
    {#if lastError}<span class="banner-error">{lastError}</span>{/if}
  </header>

  <div class="grid">
    <DaemonCard     probe={probes?.daemon     ?? null} />
    <McpCard        probe={probes?.mcp        ?? null} />
    <RoamConfigCard probe={probes?.roamConfig ?? null} />
    <GraphStatsCard probe={probes?.graphStats ?? null} />
  </div>
</main>

<style>
  main { max-width: 1200px; margin: 0 auto; padding: 32px 24px; }

  .page-header {
    display: flex;
    align-items: baseline;
    gap: 16px;
    margin-bottom: 24px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 12px;
  }
  h1 { font-size: 18px; margin: 0; font-weight: 600; }
  .poll-info { color: var(--muted); font-size: 12px; }
  .banner-error {
    margin-left: auto;
    color: #ef4444;
    font-size: 12px;
    font-family: ui-monospace, SFMono-Regular, monospace;
  }

  .grid {
    display: grid;
    gap: 16px;
    grid-template-columns: repeat(2, 1fr);
  }

  @media (max-width: 800px) {
    .grid { grid-template-columns: 1fr; }
  }
</style>
