<script lang="ts">
  import { netColors, formatRate } from '$lib/utils/colors';
  import type { NetworkInterface } from '$lib/stores/metrics.svelte';

  interface Props {
    iface: NetworkInterface;
  }

  const { iface }: Props = $props();

  const uploadColor   = $derived(netColors(iface.upload).upload);
  const downloadColor = $derived(netColors(iface.download).download);
</script>

<div class="metric-row net-row" data-tauri-drag-region>
  <span class="lbl" title={iface.name}>{iface.name}</span>
  <div class="net-values">
    <span style="color: {uploadColor}; font-variant-numeric: tabular-nums;">
      ↑ {formatRate(iface.upload)}
    </span>
    <span style="color: {downloadColor}; font-variant-numeric: tabular-nums;">
      ↓ {formatRate(iface.download)}
    </span>
  </div>
</div>

<style>
  .net-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* Wider than the default 36px to fit interface names like "Ethernet". */
  .lbl {
    color: rgba(255, 255, 255, 0.45);
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    width: 72px;
    flex-shrink: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .net-values {
    display: flex;
    gap: 12px;
    font-size: 12px;
  }
</style>
