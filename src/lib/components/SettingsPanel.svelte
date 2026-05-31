<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { load } from '@tauri-apps/plugin-store';
  import { invoke } from '@tauri-apps/api/core';
  import { settings, saveSettings } from '$lib/stores/settings.svelte';
  import type { AccentColor, Transparency } from '$lib/stores/settings.svelte';

  interface Props {
    onclose: () => void;
  }
  const { onclose }: Props = $props();

  // Local state drives the UI reactivity
  let accentColor  = $state<AccentColor>('cyan');
  let transparency = $state<Transparency>('glass');
  let showDetails  = $state(true);

  onMount(() => {
    accentColor  = settings.accentColor;
    transparency = settings.transparency;
    showDetails  = settings.showDetails;
  });

  // Apply theme directly to avoid state-reading issues
  async function applyAccent(val: AccentColor) {
    const html = document.documentElement;
    if (val === 'windows') {
      try {
        const hex = await invoke<string>('get_accent_color');
        html.style.setProperty('--windows-accent', hex);
      } catch { /* keep existing fallback */ }
    }
    html.dataset.theme = val;
  }

  function applyTransp(val: Transparency) {
    document.documentElement.dataset.transparency = val;
  }

  function applyDetails(show: boolean) {
    const html = document.documentElement;
    if (show) {
      delete html.dataset.hideDetails;
    } else {
      html.dataset.hideDetails = '';
    }
  }

  async function setAccent(val: AccentColor) {
    accentColor = val;
    settings.accentColor = val;
    await applyAccent(val);
    await saveSettings();
  }

  async function setTransparency(val: Transparency) {
    transparency = val;
    settings.transparency = val;
    applyTransp(val);
    await saveSettings();
  }

  async function toggleDetails() {
    showDetails = !showDetails;
    settings.showDetails = showDetails;
    applyDetails(showDetails);
    await saveSettings();
  }

  async function toggleLocked() {
    settings.locked = !settings.locked;
    await saveSettings();
  }

  async function anchorTo(corner: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right') {
    const win = getCurrentWindow();
    const monitor = await win.currentMonitor();
    const winSize = await win.outerSize();
    if (!monitor || !winSize) return;

    const mx = monitor.position.x;
    const my = monitor.position.y;
    const mw = monitor.size.width;
    const mh = monitor.size.height;
    const scale  = monitor.scaleFactor;
    // Use widget natural width (320 logical px) for corner math, not expanded window
    const widgetW = Math.round(320 * scale);
    const margin  = Math.round(12 * scale);

    let x: number, y: number;
    if (corner === 'top-left')          { x = mx + margin;                y = my + margin; }
    else if (corner === 'top-right')    { x = mx + mw - widgetW - margin; y = my + margin; }
    else if (corner === 'bottom-left')  { x = mx + margin;                y = my + mh - winSize.height - margin; }
    else                                { x = mx + mw - widgetW - margin; y = my + mh - winSize.height - margin; }

    await win.setPosition({ type: 'Physical', x, y });
    try {
      const store = await load('config.json');
      await store.set('position', { x, y });
      await store.save();
    } catch { /* non-blocking */ }
  }

  const ACCENTS: { id: AccentColor; color: string; label: string }[] = [
    { id: 'cyan',    color: '#06d6a0', label: 'Cyan Néon' },
    { id: 'matrix',  color: '#00ff41', label: 'Vert Matrix' },
    { id: 'white',   color: '#e0e0e0', label: 'Blanc Épuré' },
    { id: 'windows', color: '#0078d4', label: 'Thème Windows' },
  ];

  const TRANSPARENCIES: { id: Transparency; label: string }[] = [
    { id: 'opaque', label: 'Opaque' },
    { id: 'glass',  label: 'Glassmorphism' },
    { id: 'ultra',  label: 'Ultra-transparent' },
  ];
</script>

<div class="settings-panel">
  <div class="panel-header">
    <span class="panel-title">⚙ Paramètres</span>
    <button class="close-btn" onclick={onclose}>✕</button>
  </div>

  <section>
    <div class="section-label">Couleur</div>
    <div class="swatches">
      {#each ACCENTS as accent}
        <button
          class="swatch"
          class:active={accentColor === accent.id}
          style="background: {accent.color};"
          title={accent.label}
          onclick={() => setAccent(accent.id)}
        ></button>
      {/each}
    </div>
  </section>

  <div class="sdivider"></div>

  <section>
    <div class="section-label">Transparence</div>
    <div class="radio-group">
      {#each TRANSPARENCIES as t}
        <button
          class="radio-btn"
          class:active={transparency === t.id}
          onclick={() => setTransparency(t.id)}
        >{t.label}</button>
      {/each}
    </div>
  </section>

  <div class="sdivider"></div>

  <section>
    <button class="toggle-row" onclick={toggleDetails}>
      {showDetails ? '✓' : '○'} Afficher les détails
    </button>
    <button class="toggle-row" onclick={toggleLocked}>
      {settings.locked ? '✓' : '○'} Verrouiller la position
    </button>
  </section>

  <div class="sdivider"></div>

  <section>
    <div class="section-label">Ancrer le widget</div>
    <div class="anchor-grid">
      <button class="anchor-btn" onclick={() => anchorTo('top-left')}>↖ Haut gauche</button>
      <button class="anchor-btn" onclick={() => anchorTo('top-right')}>↗ Haut droite</button>
      <button class="anchor-btn" onclick={() => anchorTo('bottom-left')}>↙ Bas gauche</button>
      <button class="anchor-btn" onclick={() => anchorTo('bottom-right')}>↘ Bas droite</button>
    </div>
  </section>
</div>

<style>
  .settings-panel {
    background: rgba(12, 12, 12, 0.92);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 12px;
    padding: 10px 14px 14px;
    color: #e8e8e8;
    font-family: 'Consolas', 'Cascadia Code', 'Courier New', monospace;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 210px;
    flex-shrink: 0;
    pointer-events: auto;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .panel-title {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.4);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.3);
    font-size: 12px;
    cursor: pointer;
    padding: 0 2px;
    font-family: inherit;
    transition: color 0.15s;
  }
  .close-btn:hover { color: #ff6b6b; }

  .section-label {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.3);
    margin-bottom: 5px;
  }

  section { display: flex; flex-direction: column; }

  .swatches { display: flex; gap: 8px; }

  .swatch {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: border-color 0.15s, transform 0.1s;
  }
  .swatch.active {
    border-color: rgba(255, 255, 255, 0.85);
    transform: scale(1.2);
  }

  .radio-group { display: flex; flex-direction: column; gap: 2px; }

  .radio-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.4);
    font-family: inherit;
    font-size: 11px;
    text-align: left;
    padding: 3px 0;
    cursor: pointer;
    transition: color 0.15s;
  }
  .radio-btn.active { color: #e8e8e8; }
  .radio-btn.active::before { content: '● '; }
  .radio-btn:not(.active)::before { content: '○ '; }

  .toggle-row {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.55);
    font-family: inherit;
    font-size: 11px;
    text-align: left;
    padding: 4px 0;
    cursor: pointer;
    width: 100%;
    transition: color 0.15s;
  }
  .toggle-row:hover { color: #e8e8e8; }

  .anchor-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 4px; }

  .anchor-btn {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 5px;
    color: rgba(255, 255, 255, 0.5);
    font-family: inherit;
    font-size: 10px;
    padding: 5px 6px;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s, color 0.15s;
  }
  .anchor-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #e8e8e8;
  }

  .sdivider { height: 1px; background: rgba(255, 255, 255, 0.07); }
</style>
