<script lang="ts">
  import { thresholdColor } from '$lib/utils/colors';
  import Sparkline from './Sparkline.svelte';

  interface Props {
    label: string;
    percent: number;
    temp?: number | null;
    tempMax?: number | null;
    tempSuffix?: string;
    detail?: string;
    na?: boolean;
    history?: number[];
    subExtra?: string;
  }

  const {
    label,
    percent,
    temp = null,
    tempMax = null,
    tempSuffix = '°',
    detail = '',
    na = false,
    history = [],
    subExtra = '',
  }: Props = $props();

  const color    = $derived(na ? 'rgba(255,255,255,0.20)' : thresholdColor(percent));
  const barWidth = $derived(na ? 0 : Math.min(100, Math.max(0, percent)));
  const hasSubline = $derived(!na && (temp !== null || !!detail || !!subExtra));

  // Build sub-line segments with · separator
  const subParts = $derived.by(() => {
    const parts: string[] = [];
    if (temp !== null && temp !== undefined) {
      const tempStr = `${temp.toFixed(0)}${tempSuffix}`;
      const maxStr = (tempMax !== null && tempMax !== undefined)
        ? ` (max ${tempMax.toFixed(0)}${tempSuffix})`
        : '';
      parts.push(`${tempStr}${maxStr}`);
    }
    if (detail) parts.push(detail);
    return parts;
  });
</script>

<div class="metric-row">
  <!-- Main line: label | bar | % — sparkline anchors here -->
  <div class="main-line">
    <span class="lbl">{label}</span>
    <div class="bar-track">
      <div class="bar-fill" style="width: {barWidth}%; background: {color};">
        <div class="bar-shimmer"></div>
      </div>
    </div>
    <span class="pct" style="color: {color};">
      {#if na}N/A{:else}{percent.toFixed(0)}%{/if}
    </span>

    {#if history.length >= 2 && !na}
      <div class="sparkline-wrap">
        <Sparkline values={history} {color} />
      </div>
    {/if}
  </div>

  <!-- Sub-line: temp · detail · subExtra -->
  {#if hasSubline}
    <div class="sub-line">
      {#each subParts as part, i}
        {#if i > 0}<span class="sep">·</span>{/if}
        <span class="sub-fixed">{part}</span>
      {/each}
      {#if subExtra}
        {#if subParts.length > 0}<span class="sep">·</span>{/if}
        <span class="sub-extra">{subExtra}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .main-line {
    position: relative;
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
    height: 5px;
    background: rgba(255, 255, 255, 0.07);
    border-radius: 3px;
    overflow: hidden;
  }

  .bar-fill {
    position: relative;
    height: 100%;
    border-radius: 3px;
    transition: width 0.4s ease, background-color 0.3s ease;
    min-width: 2px;
    overflow: hidden;
  }

  /* Subtle right-side shimmer — adds depth to every bar */
  .bar-shimmer {
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, rgba(255,255,255,0) 40%, rgba(255,255,255,0.18) 100%);
    border-radius: inherit;
    pointer-events: none;
  }

  .pct {
    font-size: 12px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  /* ── Sub-line ── */
  .sub-line {
    display: flex;
    align-items: center;
    gap: 5px;
    padding-left: 42px;
    margin-top: 2px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.40);
    overflow: hidden;
    white-space: nowrap;
  }

  .sub-fixed {
    flex-shrink: 0;
  }

  .sep {
    color: rgba(255, 255, 255, 0.18);
    flex-shrink: 0;
    font-size: 10px;
  }

  .sub-extra {
    color: rgba(255, 255, 255, 0.28);
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
    background: rgba(10, 10, 10, 0.90);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 4px;
    padding: 2px 4px;
  }

  .metric-row:hover .sparkline-wrap {
    opacity: 1;
  }
</style>
