<script lang="ts">
  import { thresholdColor, formatBytes } from '$lib/utils/colors';
  import type { DiskInfo } from '$lib/stores/metrics.svelte';

  interface Props {
    disk: DiskInfo;
  }

  const { disk }: Props = $props();

  const color    = $derived(thresholdColor(disk.percent));
  const barWidth = $derived(Math.min(100, Math.max(0, disk.percent)));

  /** Show only the drive letter + colon on Windows, e.g. "C:" from "C:\\" */
  const mountLabel = $derived(
    disk.mount.length >= 2 && disk.mount[1] === ':'
      ? disk.mount.slice(0, 2)
      : disk.mount
  );

  const detail = $derived(
    `${formatBytes(disk.used)} / ${formatBytes(disk.total)}`
  );
</script>

<div class="metric-row disk-row">
  <span class="metric-label">{mountLabel}</span>

  <div class="bar-track">
    <div
      class="bar-fill"
      style="width: {barWidth}%; background: {color};"
    ></div>
  </div>

  <span class="metric-values">
    <span style="color: {color};">{disk.percent.toFixed(0)}%</span>
    <span class="metric-temp">{detail}</span>
  </span>
</div>
