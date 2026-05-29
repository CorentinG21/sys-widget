<script lang="ts">
  import { thresholdColor } from '$lib/utils/colors';
  import Sparkline from './Sparkline.svelte';

  interface Props {
    /** Short label shown on the left, e.g. "CPU", "RAM". */
    label: string;
    /** Usage 0–100. */
    percent: number;
    /** Optional temperature in °C — shown on sub-line. */
    temp?: number | null;
    /** Optional detail string (e.g. "12.8 / 31.9 GB") — shown on sub-line. */
    detail?: string;
    /** When true, renders the row in a muted "N/A" state (sensor unavailable). */
    na?: boolean;
    /** Historical readings for the sparkline (last 30 s). */
    history?: number[];
    /** Extra text appended to the sub-line (e.g. "🔥 chrome.exe · 3%"). */
    subExtra?: string;
  }

  const {
    label,
    percent,
    temp = null,
    detail = '',
    na = false,
    history = [],
    subExtra = '',
  }: Props = $props();

  const color    = $derived(na ? 'rgba(255,255,255,0.25)' : thresholdColor(percent));
  const barWidth = $derived(na ? 0 : Math.min(100, Math.max(0, percent)));
  const hasSubline = $derived(!na && (temp !== null || !!detail || !!subExtra));
</script>

<div class="metric-row">
  <!-- Main line: label | bar | % — sparkline anchors here so it centers on the bar -->
  <div class="main-line">
    <span class="lbl">{label}</span>
    <div class="bar-track">
      <div class="bar-fill" style="width: {barWidth}%; background: {color};"></div>
    </div>
    <span class="pct" style="color: {color};">
      {#if na}N/A{:else}{percent.toFixed(0)}%{/if}
    </span>

    <!-- Sparkline anchored inside main-line so top:50% centers on bar height, not sub-line -->
    {#if history.length >= 2 && !na}
      <div class="sparkline-wrap">
        <Sparkline values={history} {color} />
      </div>
    {/if}
  </div>

  <!-- Sub-line: temp · detail · subExtra -->
  {#if hasSubline}
    <div class="sub-line">
      {#if temp !== null}
        <span class="sub-fixed">{temp.toFixed(0)}°C</span>
      {/if}
      {#if detail}
        <span class="sub-fixed">{detail}</span>
      {/if}
      {#if subExtra}
        <span class="sub-extra">{subExtra}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .metric-row {
    /* no position:relative needed — sparkline anchors to .main-line now */
  }

  /* ── Main line: label | bar | % ── */
  .main-line {
    position: relative; /* sparkline-wrap anchors here */
    display: grid;
    grid-template-columns: 36px 1fr 28px;
    gap: 6px;
    align-items: center;
  }

  .lbl {
    color: rgba(255, 255, 255, 0.45);
    font-size: 12px;
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
    font-size: 12px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  /* ── Sub-line ── */
  .sub-line {
    display: flex;
    gap: 8px;
    padding-left: 42px; /* 36px label + 6px gap */
    margin-top: 2px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.40);
    overflow: hidden;
    white-space: nowrap;
  }

  /* temp/detail are short — don't let them shrink or wrap */
  .sub-fixed {
    flex-shrink: 0;
  }

  /* subExtra (top process) takes remaining space and truncates */
  .sub-extra {
    color: rgba(255, 255, 255, 0.30);
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  /* ── Sparkline overlay ── */
  .sparkline-wrap {
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    opacity: 0;
    transition: opacity 0.15s ease;
    pointer-events: none;
    background: rgba(10, 10, 10, 0.85);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    padding: 2px 4px;
  }

  .metric-row:hover .sparkline-wrap {
    opacity: 1;
  }
</style>
