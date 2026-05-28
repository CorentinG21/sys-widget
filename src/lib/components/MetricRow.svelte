<script lang="ts">
  import { thresholdColor } from '$lib/utils/colors';
  import { formatBytes } from '$lib/utils/colors';

  interface Props {
    /** Short label shown on the left, e.g. "CPU", "RAM". */
    label: string;
    /** Usage 0–100. */
    percent: number;
    /** Optional temperature in °C. */
    temp?: number | null;
    /** Optional right-side detail, e.g. "7.2 / 16.0 GB". */
    detail?: string;
  }

  const { label, percent, temp = null, detail = '' }: Props = $props();

  const color   = $derived(thresholdColor(percent));
  const barWidth = $derived(Math.min(100, Math.max(0, percent)));
</script>

<div class="metric-row">
  <span class="metric-label">{label}</span>

  <div class="bar-track">
    <div
      class="bar-fill"
      style="width: {barWidth}%; background: {color};"
    ></div>
  </div>

  <span class="metric-values">
    <span style="color: {color};">{percent.toFixed(0)}%</span>
    {#if temp !== null && temp !== undefined}
      <span class="metric-temp">{temp.toFixed(0)}°C</span>
    {/if}
    {#if detail}
      <span class="metric-temp">{detail}</span>
    {/if}
  </span>
</div>
