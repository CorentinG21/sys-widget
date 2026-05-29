# UI Refonte B1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refondre le layout de toutes les lignes de métriques en style B1 — ligne principale en grid `label | barre | %` avec sous-ligne indentée pour les détails.

**Architecture:** Chaque composant (MetricRow, DiskRow, NetworkRow) gère son propre layout via des styles scopés Svelte. `app.css` ne garde que les styles vraiment globaux (widget container, divider, pointer-events). Le top process passe de div standalone à `subExtra` prop sur le CPU MetricRow.

**Tech Stack:** Svelte 5, SvelteKit, CSS Grid, CSS scoped styles

---

## Fichiers modifiés

- `src/app.css` — Suppression classes devenues inutiles, mise à jour tokens
- `src/lib/components/MetricRow.svelte` — Réécriture complète layout B1
- `src/lib/components/DiskRow.svelte` — Réécriture complète layout B1
- `src/lib/components/NetworkRow.svelte` — Mise à jour layout NET
- `src/routes/+page.svelte` — Suppression top-process div, passage de subExtra

---

## Task 1: Mettre à jour app.css

**Files:**
- Modify: `src/app.css`

- [ ] **Step 1: Remplacer le contenu de app.css**

Le nouveau `app.css` conserve uniquement ce qui est vraiment global — widget, divider, pointer-events. Les classes `.metric-label`, `.metric-values`, `.metric-temp`, `.bar-track`, `.bar-fill`, `.metric-row` layout, `.disk-row`, `.top-process` sont supprimées (chaque composant les redéfinit en scoped).

```css
/* ─────────────────────────────────────────────────────────────────────
   Global reset & design tokens for SysmonWidget glassmorphism UI
   ───────────────────────────────────────────────────────────────────── */

*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

:root {
  /* Typography */
  font-family: 'Consolas', 'Cascadia Code', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.4;
  -webkit-font-smoothing: antialiased;

  /* Transparent base — the WebView background must be transparent */
  background: transparent;
  color: #e8e8e8;

  /* ── Color tokens ── */
  --color-ok:      #06d6a0;   /* green  < 70 % */
  --color-warn:    #ffd166;   /* yellow 70–89 % */
  --color-danger:  #ff6b6b;   /* red    ≥ 90 % */
  --color-dl:      #74d7f7;   /* cyan   download < 1 MB/s */

  /* ── Glassmorphism surface ── */
  --glass-bg:      rgba(10, 10, 10, 0.78);
  --glass-blur:    blur(16px);
  --glass-border:  1px solid rgba(255, 255, 255, 0.08);
  --glass-radius:  12px;
  --glass-padding: 14px 16px;

  /* ── Spacing ── */
  --row-gap: 6px;
}

html, body {
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
  user-select: none;
}

/* ── Widget container ── */
.widget {
  display: flex;
  flex-direction: column;
  gap: var(--row-gap);
  width: 100%;
  height: fit-content;
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: var(--glass-border);
  border-radius: var(--glass-radius);
  padding: var(--glass-padding);
  color: #e8e8e8;
  cursor: default;
}

/* ── All widget children are non-interactive: clicks fall through to .widget
      which carries data-tauri-drag-region and oncontextmenu. ── */
.widget * {
  pointer-events: none;
}

/* ── metric-row needs pointer-events:auto for :hover (sparkline).
      mousedown still bubbles to .widget for drag-region. ── */
.widget .metric-row {
  pointer-events: auto;
}

/* ── Section divider ── */
.divider {
  height: 1px;
  background: rgba(255, 255, 255, 0.07);
  margin: 4px 0;
}
```

- [ ] **Step 2: Vérifier visuellement que le widget container est intact**

Lance `npm run tauri dev` (en terminal admin) et vérifie que le fond glassmorphism s'affiche encore (les lignes seront cassées — c'est normal, les composants ne sont pas encore mis à jour).

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "refactor: strip app.css to globals only — layout moves to scoped component styles"
```

---

## Task 2: Réécrire MetricRow.svelte

**Files:**
- Modify: `src/lib/components/MetricRow.svelte`

- [ ] **Step 1: Réécrire le composant complet**

```svelte
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
  <!-- Main line: label | bar | % -->
  <div class="main-line">
    <span class="lbl">{label}</span>
    <div class="bar-track">
      <div class="bar-fill" style="width: {barWidth}%; background: {color};"></div>
    </div>
    <span class="pct" style="color: {color};">
      {#if na}N/A{:else}{percent.toFixed(0)}%{/if}
    </span>
  </div>

  <!-- Sub-line: temp · detail · subExtra -->
  {#if hasSubline}
    <div class="sub-line">
      {#if temp !== null && temp !== undefined}
        <span>{temp.toFixed(0)}°C</span>
      {/if}
      {#if detail}
        <span>{detail}</span>
      {/if}
      {#if subExtra}
        <span class="sub-extra">{subExtra}</span>
      {/if}
    </div>
  {/if}

  <!-- Sparkline overlay — visible on :hover -->
  {#if history.length >= 2 && !na}
    <div class="sparkline-wrap">
      <Sparkline values={history} {color} />
    </div>
  {/if}
</div>

<style>
  .metric-row {
    position: relative;
  }

  /* ── Main line: label | bar | % ── */
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
    flex-shrink: 0;
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

  /* ── Sub-line ── */
  .sub-line {
    display: flex;
    gap: 8px;
    padding-left: 42px; /* 36px label + 6px gap */
    margin-top: 2px;
    font-size: 9px;
    color: rgba(255, 255, 255, 0.28);
    overflow: hidden;
  }

  .sub-extra {
    color: rgba(255, 255, 255, 0.18);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
```

- [ ] **Step 2: Vérifier dans le navigateur (tauri dev)**

Les lignes CPU, GPU, RAM doivent afficher :
```
CPU  [░░░░░░░░░░░░░░]  8%
     47°C  🔥 msedgewebview2.exe · 1%
GPU  [░░░░░░░░░░░░░░]  0%
     47°C  1.1 / 8.0 GB
RAM  [████░░░░░░░░░░]  40%
     12.8 / 31.9 GB
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/MetricRow.svelte
git commit -m "feat: MetricRow B1 — grid main-line + sub-line layout"
```

---

## Task 3: Réécrire DiskRow.svelte

**Files:**
- Modify: `src/lib/components/DiskRow.svelte`

- [ ] **Step 1: Réécrire le composant**

```svelte
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
```

- [ ] **Step 2: Vérifier dans le navigateur**

Les disques doivent afficher :
```
C:   [████████░░░░░]  82%
     376.8 / 446.5 GB
E:   [███░░░░░░░░░░]  26%
     238.6 / 931.5 GB
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/DiskRow.svelte
git commit -m "feat: DiskRow B1 — grid main-line + sub-line layout"
```

---

## Task 4: Mettre à jour NetworkRow.svelte

**Files:**
- Modify: `src/lib/components/NetworkRow.svelte`

- [ ] **Step 1: Réécrire le composant**

```svelte
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
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    width: 36px;
    flex-shrink: 0;
  }

  .net-values {
    display: flex;
    gap: 12px;
    font-size: 11px;
  }
</style>
```

- [ ] **Step 2: Vérifier dans le navigateur**

La ligne réseau doit afficher :
```
NET  ↑ 5.8 KB/s  ↓ 2.4 KB/s
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/NetworkRow.svelte
git commit -m "feat: NetworkRow B1 — label + values, no bar"
```

---

## Task 5: Mettre à jour +page.svelte

**Files:**
- Modify: `src/routes/+page.svelte`

Le top process passe de div standalone à prop `subExtra` sur le MetricRow CPU, ce qui le place sur la même sous-ligne que la température.

- [ ] **Step 1: Calculer subExtra en $derived**

Remplacer dans la section script :

```typescript
  // Remplacer les deux lignes ramDetail / vramDetail par :
  const ramDetail  = $derived(`${formatBytes(metrics.ram.used)} / ${formatBytes(metrics.ram.total)}`);
  const vramDetail = $derived(
    metrics.gpu ? `${formatBytes(metrics.gpu.vram_used)} / ${formatBytes(metrics.gpu.vram_total)}` : ''
  );
  const cpuSubExtra = $derived(
    metrics.top_cpu
      ? `🔥 ${metrics.top_cpu.name} · ${metrics.top_cpu.cpu_percent.toFixed(0)}%`
      : ''
  );
```

- [ ] **Step 2: Mettre à jour le template**

Remplacer le bloc CPU + top_cpu par :

```svelte
  <MetricRow
    label="CPU"
    percent={metrics.cpu.percent}
    temp={metrics.cpu.temp}
    history={cpuHistory}
    subExtra={cpuSubExtra}
  />
```

Supprimer le bloc `{#if metrics.top_cpu}...{/if}` qui n'existe plus.

- [ ] **Step 3: Vérifier dans le navigateur**

Le widget complet doit ressembler à :
```
CPU  [█░░░░░░░░░░]  8%
     47°C  🔥 msedgewebview2.exe · 1%
GPU  [░░░░░░░░░░░]  0%
     47°C  1.1 / 8.0 GB
RAM  [████░░░░░░░]  40%
     12.8 / 31.9 GB
──────────────────────────
C:   [████████░░░]  82%
     376.8 / 446.5 GB
E:   [███░░░░░░░░]  26%
     238.6 / 931.5 GB
──────────────────────────
NET  ↑ 5.8 KB/s  ↓ 2.4 KB/s
```

- [ ] **Step 4: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat: move top_cpu to CPU MetricRow subExtra prop"
```

---

## Task 6: Bump de version et tag

**Files:**
- Modify: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`

- [ ] **Step 1: Bump version à 2.2.0**

Dans `src-tauri/tauri.conf.json` :
```json
"version": "2.2.0"
```

Dans `src-tauri/Cargo.toml` :
```toml
version = "2.2.0"
```

- [ ] **Step 2: Commit et tag**

```bash
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m "chore: bump to v2.2.0 — UI refonte B1"
git push origin main
git tag v2.2.0
git push origin v2.2.0
```
