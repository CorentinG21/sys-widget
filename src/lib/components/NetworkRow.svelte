<script lang="ts">
  import { netColors, formatRate } from '$lib/utils/colors';
  import type { NetworkMetrics } from '$lib/stores/metrics.svelte';

  interface Props {
    network: NetworkMetrics;
  }

  const { network }: Props = $props();

  const uploadColor   = $derived(netColors(network.upload).upload);
  const downloadColor = $derived(netColors(network.download).download);
</script>

<div class="metric-row net-row">
  <span class="metric-label">NET</span>

  <span class="net-values">
    <span class="net-item">
      <span class="net-arrow" style="color: {uploadColor};">↑</span>
      <span style="color: {uploadColor}; font-variant-numeric: tabular-nums;">
        {formatRate(network.upload)}
      </span>
    </span>
    <span class="net-item">
      <span class="net-arrow" style="color: {downloadColor};">↓</span>
      <span style="color: {downloadColor}; font-variant-numeric: tabular-nums;">
        {formatRate(network.download)}
      </span>
    </span>
  </span>
</div>

<style>
  .net-row {
    align-items: center;
  }

  .net-values {
    flex: 1;
    display: flex;
    gap: 12px;
    font-size: 12px;
  }

  .net-item {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .net-arrow {
    font-size: 11px;
  }
</style>
