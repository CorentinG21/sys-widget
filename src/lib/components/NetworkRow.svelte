<script lang="ts">
  import { netColors, formatRate, latencyColor } from '$lib/utils/colors';
  import type { NetworkMetrics } from '$lib/stores/metrics.svelte';

  interface Props {
    network: NetworkMetrics;
  }

  const { network }: Props = $props();

  const uploadColor   = $derived(netColors(network.upload).upload);
  const downloadColor = $derived(netColors(network.download).download);
  const pingColor     = $derived(
    network.latency_ms != null ? latencyColor(network.latency_ms) : 'rgba(255,255,255,0.3)'
  );
  const pingLabel     = $derived(
    network.latency_ms != null ? `${network.latency_ms}ms` : '--'
  );
</script>

<div class="net-row">
  <span class="lbl">NET</span>
  <div class="net-values">
    <span style="color: {uploadColor}; font-variant-numeric: tabular-nums;">
      ↑ {formatRate(network.upload)}
    </span>
    <span style="color: {downloadColor}; font-variant-numeric: tabular-nums;">
      ↓ {formatRate(network.download)}
    </span>
    <span class="ping" style="color: {pingColor};">• {pingLabel}</span>
  </div>
</div>

<style>
  .net-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .lbl {
    color: rgba(255, 255, 255, 0.45);
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    width: 36px;
    flex-shrink: 0;
  }

  .net-values {
    display: flex;
    gap: 14px;
    font-size: 12px;
  }

  .ping {
    font-variant-numeric: tabular-nums;
    opacity: 0.9;
  }
</style>
