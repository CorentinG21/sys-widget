<script lang="ts">
  import { onMount } from 'svelte';
  import { load } from '@tauri-apps/plugin-store';
  import { settings, saveSettings } from '$lib/stores/settings.svelte';
  import type { AccentColor, Transparency } from '$lib/stores/settings.svelte';

  interface Props {
    onclose: () => void;
    onanchor: (corner: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right') => Promise<void>;
  }
  const { onclose, onanchor }: Props = $props();

  // Local state drives the UI reactivity
  let accentColor  = $state<AccentColor>('cyan');
  let transparency = $state<number>(78);
  let showDetails  = $state(true);

  onMount(() => {
    accentColor  = settings.accentColor;
    transparency = settings.transparency;
    showDetails  = settings.showDetails;
  });

  function applyAccent(val: AccentColor) {
    document.documentElement.dataset.theme = val;
  }

  function applyTransp(val: number) {
    document.documentElement.style.setProperty(
      '--glass-bg',
      `rgba(10, 10, 10, ${(val / 100).toFixed(2)})`
    );
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
    applyAccent(val);
    await saveSettings();
  }

  async function setTransparency(val: number) {
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


  const ACCENTS: { id: AccentColor; color: string; label: string }[] = [
    { id: 'cyan',    color: '#06d6a0', label: 'Cyan Néon' },
    { id: 'matrix',  color: '#00ff41', label: 'Vert Matrix' },
    { id: 'white',   color: '#e0e0e0', label: 'Blanc Épuré' },
    { id: 'neutral', color: '#909090', label: 'Gris Neutre' },
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
    <div class="transp-header">
      <span class="section-label">Transparence</span>
      <span class="transp-value">{transparency}%</span>
    </div>
    <input
      type="range"
      class="transp-slider"
      min="20"
      max="98"
      step="1"
      value={transparency}
      oninput={(e) => setTransparency(Number(e.currentTarget.value))}
    />
    <div class="transp-labels">
      <span>Transparent</span>
      <span>Opaque</span>
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
      <button class="anchor-btn" onclick={() => onanchor('top-left')}>↖ Haut gauche</button>
      <button class="anchor-btn" onclick={() => onanchor('top-right')}>↗ Haut droite</button>
      <button class="anchor-btn" onclick={() => onanchor('bottom-left')}>↙ Bas gauche</button>
      <button class="anchor-btn" onclick={() => onanchor('bottom-right')}>↘ Bas droite</button>
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

  /* ── Transparency slider ── */
  .transp-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }
  .transp-header .section-label { margin-bottom: 0; }
  .transp-value {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.45);
    font-variant-numeric: tabular-nums;
  }

  .transp-slider {
    width: 100%;
    height: 4px;
    appearance: none;
    -webkit-appearance: none;
    background: rgba(255, 255, 255, 0.12);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
    pointer-events: auto;
  }
  .transp-slider::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #e8e8e8;
    cursor: pointer;
    border: 2px solid rgba(255, 255, 255, 0.3);
    box-shadow: 0 1px 4px rgba(0,0,0,0.4);
    transition: transform 0.1s;
  }
  .transp-slider::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }
  .transp-labels {
    display: flex;
    justify-content: space-between;
    font-size: 9px;
    color: rgba(255, 255, 255, 0.22);
    margin-top: 4px;
  }

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
