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

<div class="metric-row net-row" data-tauri-drag-region>
  <span class="lbl">NET</span>
  <div class="net-values">
    <span style="color: {uploadColor}; font-variant-numeric: tabular-nums;">
      ↑ {formatRate(network.upload)}
    </span>
    <span style="color: {downloadColor}; font-variant-numeric: tabular-nums;">
      ↓ {formatRate(network.download)}
    </span>
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
    gap: 12px;
    font-size: 12px;
  }
</style>
