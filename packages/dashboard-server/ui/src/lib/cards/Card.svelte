<script lang="ts">
  import type { Probe } from '../types';
  import { formatRelativeTime } from '../format';

  interface Props {
    title: string;
    probe: Probe<unknown> | null;
    children?: import('svelte').Snippet;
  }

  let { title, probe, children }: Props = $props();

  let statusClass = $derived(
    probe === null ? 'pending' : probe.status === 'up' ? 'up' : 'down',
  );
  let statusLabel = $derived(
    probe === null ? '…' : probe.status === 'up' ? 'up' : 'down',
  );
</script>

<section class="card {statusClass}">
  <header>
    <span class="dot" aria-hidden="true"></span>
    <h2>{title}</h2>
    <span class="status">{statusLabel}</span>
    {#if probe}
      <span class="probed-at">{formatRelativeTime(probe.probedAt)}</span>
    {/if}
  </header>
  <div class="body">
    {#if probe === null}
      <p class="muted">Loading…</p>
    {:else if probe.status === 'down'}
      <p class="error">{probe.error}</p>
    {:else}
      {@render children?.()}
    {/if}
  </div>
</section>

<style>
  .card {
    border: 1px solid var(--border, #2a2a2a);
    border-radius: 8px;
    padding: 16px 20px;
    background: var(--card-bg, #161616);
    transition: border-color 0.2s;
  }
  .card.up    { border-color: #2d6a3a; }
  .card.down  { border-color: #8b2a2a; }
  .card.pending { border-color: #555; }

  header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 12px;
  }
  h2 {
    font-size: 14px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
    flex: 1;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex: none;
  }
  .up    .dot { background: #4ade80; box-shadow: 0 0 6px #4ade80; }
  .down  .dot { background: #ef4444; box-shadow: 0 0 6px #ef4444; }
  .pending .dot { background: #888; }

  .status {
    font-size: 12px;
    font-weight: 500;
    text-transform: uppercase;
  }
  .up    .status { color: #4ade80; }
  .down  .status { color: #ef4444; }

  .probed-at {
    font-size: 11px;
    color: #888;
    font-variant-numeric: tabular-nums;
  }

  .body { font-size: 13px; line-height: 1.5; }
  .body :global(dl) { margin: 0; display: grid; grid-template-columns: max-content 1fr; gap: 4px 14px; }
  .body :global(dt) { color: #999; }
  .body :global(dd) { margin: 0; font-variant-numeric: tabular-nums; word-break: break-all; }
  .body :global(.num) { font-variant-numeric: tabular-nums; }

  .error  { color: #ef4444; font-family: ui-monospace, SFMono-Regular, monospace; font-size: 12px; word-break: break-word; }
  .muted  { color: #888; }
</style>
