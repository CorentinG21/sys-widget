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

  const detail = $derived(`${formatBytes(disk.used)} / ${formatBytes(disk.total)}`);
</script>

<div class="metric-row">
  <!-- Main line: label | bar | % -->
  <div class="main-line">
    <span class="lbl">{mountLabel}</span>
    <div class="bar-track">
      <div class="bar-fill" style="width: {barWidth}%; background: {color};"></div>
    </div>
    <span class="pct" style="color: {color};">{disk.percent.toFixed(0)}%</span>
  </div>

  <!-- Sub-line: used / total -->
  <div class="sub-line">{detail}</div>
</div>

<style>
  .metric-row {
    position: relative;
  }

  .main-line {
    display: grid;
    grid-template-columns: 36px 1fr 28px;
    gap: 6px;
    align-items: center;
  }

  .lbl {
    color: rgba(255, 255, 255, 0.45);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .bar-track {
    height: 4px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 2px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.4s ease, background-color 0.3s ease;
    min-width: 2px;
  }

  .pct {
    font-size: 11px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .sub-line {
    padding-left: 42px; /* 36px label + 6px gap */
    margin-top: 2px;
    font-size: 9px;
    color: rgba(255, 255, 255, 0.28);
  }
</style>
