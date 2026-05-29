<script lang="ts">
  import { thresholdColor } from '$lib/utils/colors';
  import Sparkline from './Sparkline.svelte';

  interface Props {
    /** Short label shown on the left, e.g. "CPU", "RAM". */
    label: string;
    /** Usage 0–100. */
    percent: number;
    /** Optional temperature in °C. */
    temp?: number | null;
    /** Optional right-side detail, e.g. "7.2 / 16.0 GB". */
    detail?: string;
    /** When true, renders the row in a muted "N/A" state (sensor unavailable). */
    na?: boolean;
    /** Historical readings for the sparkline (last 30 s). */
    history?: number[];
  }

  const { label, percent, temp = null, detail = '', na = false, history = [] }: Props = $props();

  const color    = $derived(na ? 'rgba(255,255,255,0.25)' : thresholdColor(percent));
  const barWidth = $derived(na ? 0 : Math.min(100, Math.max(0, percent)));
</script>

<!-- pointer-events: auto on the row so :hover works for the sparkline.
     mousedown still bubbles to .widget for drag-region. -->
<div class="metric-row">
  <span class="metric-label">{label}</span>

  <div class="bar-track">
    <div
      class="bar-fill"
      style="width: {barWidth}%; background: {color};"
    ></div>
  </div>

  <span class="metric-values">
    {#if na}
      <span style="color: {color};">N/A</span>
    {:else}
      <span style="color: {color};">{percent.toFixed(0)}%</span>
      {#if temp !== null && temp !== undefined}
        <span class="metric-temp">{temp.toFixed(0)}°C</span>
      {/if}
      {#if detail}
        <span class="metric-temp">{detail}</span>
      {/if}
    {/if}
  </span>

  {#if history.length >= 2 && !na}
    <div class="sparkline-wrap">
      <Sparkline values={history} {color} />
    </div>
  {/if}
</div>

<style>
  /* sparkline hidden by default, visible on row hover */
  .sparkline-wrap {
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    opacity: 0;
    transition: opacity 0.15s ease;
    pointer-events: none;
    background: rgba(10, 10, 10, 0.85);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 4px;
    padding: 2px 4px;
  }

  .metric-row:hover .sparkline-wrap {
    opacity: 1;
  }
</style>
