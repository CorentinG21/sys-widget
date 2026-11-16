<script lang="ts">
  import { connectionStatus, formatNetRate } from '$lib/utils/network';
  import { settings } from '$lib/stores/settings.svelte';
  import { translations } from '$lib/i18n';
  import type { NetworkMetrics } from '$lib/stores/metrics.svelte';

  interface Props {
    network: NetworkMetrics;
    /** False until the first measurement: the connection state is not known yet. */
    ready?: boolean;
  }

  const { network, ready = true }: Props = $props();

  const t = $derived(translations[settings.lang]);
  const status = $derived(connectionStatus(network.latency_ms));

  const statusLabel = $derived(
    !ready               ? '…'
    : status === 'ok'    ? t.netOk
    : status === 'slow'  ? t.netSlow
    :                      t.netOffline
  );
  // The dot is the only color on the row, so it can only mean one thing.
  const dotColor = $derived(
    !ready               ? 'rgba(255, 255, 255, 0.25)'
    : status === 'ok'    ? 'var(--color-ok)'
    : status === 'slow'  ? 'var(--color-warn)'
    :                      'var(--color-danger)'
  );
  // Shown instead of the state while the row is hovered.
  const latencyLabel = $derived(
    ready && network.latency_ms != null ? `${network.latency_ms} ms` : statusLabel
  );
</script>

<div class="net-row">
  <span class="lbl">NET</span>
  <span class="rate down"><span class="arrow">↓</span><span class="num">{formatNetRate(network.download, settings.lang)}</span></span>
  <span class="rate up"><span class="arrow">↑</span><span class="num">{formatNetRate(network.upload, settings.lang)}</span></span>
  <span class="status">
    <span class="dot" style="color: {dotColor};">●</span>
    <span class="status-text">{statusLabel}</span>
    <span class="status-latency">{latencyLabel}</span>
  </span>
</div>

<style>
  .net-row {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    white-space: nowrap;
    /* .widget * disables pointer events; the row needs them for :hover. */
    pointer-events: auto;
  }

  .lbl {
    color: rgba(255, 255, 255, 0.45);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    width: 36px;
    flex-shrink: 0;
  }

  /* Fixed width for the longest value ("↓ 999,9 Ko/s"): whatever the digits
     and unit, the next item never moves. */
  .rate {
    display: inline-block;
    min-width: calc(11ch + 2px);
    flex-shrink: 0;
  }

  /* Theme colors, constant: the speed alone is not a problem, so the values
     never turn yellow or red. */
  .down { color: var(--color-dl); }
  .up   { color: var(--color-ok); }

  .arrow {
    opacity: 0.75;
    margin-right: 2px;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .status {
    margin-left: auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: rgba(255, 255, 255, 0.55);
  }

  .dot {
    margin-right: 3px;
    transition: color 0.3s ease;
  }

  .status-latency { display: none; font-variant-numeric: tabular-nums; }
  .net-row:hover .status-text { display: none; }
  .net-row:hover .status-latency { display: inline; }
</style>
