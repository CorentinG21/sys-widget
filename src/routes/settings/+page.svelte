<script lang="ts">
  import '../../app.css';
  import { onMount } from 'svelte';
  import { getCurrentWindow, PhysicalPosition } from '@tauri-apps/api/window';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { load } from '@tauri-apps/plugin-store';
  import {
    settings,
    loadSettings,
    saveAndEmit,
    applyToDocument,
    type AccentColor,
    type Transparency,
  } from '$lib/stores/settings.svelte';

  const win = getCurrentWindow();

  onMount(async () => {
    await loadSettings();
    await applyToDocument();
  });

  async function setAccent(val: AccentColor) {
    settings.accentColor = val;
    await applyToDocument();
    await saveAndEmit();
  }

  async function setTransparency(val: Transparency) {
    settings.transparency = val;
    await applyToDocument();
    await saveAndEmit();
  }

  async function toggleDetails() {
    settings.showDetails = !settings.showDetails;
    await applyToDocument();
    await saveAndEmit();
  }

  async function toggleLocked() {
    settings.locked = !settings.locked;
    await saveAndEmit();
  }

  async function anchorTo(corner: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right') {
    const mainWin = await WebviewWindow.getByLabel('main');
    if (!mainWin) return;

    const monitor = await mainWin.currentMonitor();
    const widgetSize = await mainWin.outerSize();
    if (!monitor || !widgetSize) return;

    const mx = monitor.position.x;
    const my = monitor.position.y;
    const mw = monitor.size.width;
    const mh = monitor.size.height;
    const margin = 12;

    let x: number, y: number;
    if (corner === 'top-left')          { x = mx + margin;                        y = my + margin; }
    else if (corner === 'top-right')    { x = mx + mw - widgetSize.width - margin; y = my + margin; }
    else if (corner === 'bottom-left')  { x = mx + margin;                        y = my + mh - widgetSize.height - margin; }
    else                                { x = mx + mw - widgetSize.width - margin; y = my + mh - widgetSize.height - margin; }

    await mainWin.setPosition(new PhysicalPosition(x, y));

    const store = await load('config.json');
    await store.set('position', { x, y });
    await store.save();
  }

  function close() { win.hide(); }

  const ACCENTS: { id: AccentColor; label: string; color: string }[] = [
    { id: 'cyan',    label: 'Cyan Néon',     color: '#06d6a0' },
    { id: 'matrix',  label: 'Vert Matrix',   color: '#00ff41' },
    { id: 'white',   label: 'Blanc Épuré',   color: '#e0e0e0' },
    { id: 'windows', label: 'Thème Windows', color: '#0078d4' },
  ];

  const TRANSPARENCIES: { id: Transparency; label: string }[] = [
    { id: 'opaque', label: 'Opaque' },
    { id: 'glass',  label: 'Glassmorphism' },
    { id: 'ultra',  label: 'Ultra-transparent' },
  ];
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="panel" onmousedown={(e) => {
  if (e.button !== 0) return;
  if ((e.target as HTMLElement).closest('button, input, a, [role="button"]')) return;
  win.startDragging();
}}>

  <div class="header">
    <span class="title">⚙ Paramètres</span>
    <button class="close-btn" onclick={close}>✕</button>
  </div>

  <!-- ── Couleur d'accentuation ── -->
  <section>
    <div class="section-label">Couleur</div>
    <div class="swatches">
      {#each ACCENTS as accent}
        <button
          class="swatch"
          class:active={settings.accentColor === accent.id}
          style="background: {accent.color};"
          title={accent.label}
          onclick={() => setAccent(accent.id)}
        ></button>
      {/each}
    </div>
  </section>

  <div class="divider"></div>

  <!-- ── Transparence ── -->
  <section>
    <div class="section-label">Transparence</div>
    <div class="radio-group">
      {#each TRANSPARENCIES as t}
        <button
          class="radio-btn"
          class:active={settings.transparency === t.id}
          onclick={() => setTransparency(t.id)}
        >
          {t.label}
        </button>
      {/each}
    </div>
  </section>

  <div class="divider"></div>

  <!-- ── Toggles ── -->
  <section>
    <button class="toggle-row" onclick={toggleDetails}>
      <span>{settings.showDetails ? '✓' : '○'} Afficher les détails</span>
    </button>
    <button class="toggle-row" onclick={toggleLocked}>
      <span>{settings.locked ? '✓' : '○'} Verrouiller la position</span>
    </button>
  </section>

  <div class="divider"></div>

  <!-- ── Ancrage ── -->
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
  html, body {
    width: 100%;
    height: 100%;
    background: transparent;
    overflow: hidden;
    user-select: none;
  }

  .panel {
    font-family: 'Consolas', 'Cascadia Code', 'Courier New', monospace;
    font-size: 12px;
    background: rgba(12, 12, 12, 0.92);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 12px;
    padding: 10px 14px 14px;
    color: #e8e8e8;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 100vh;
    box-sizing: border-box;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2px;
  }

  .title {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.45);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.3);
    font-size: 12px;
    cursor: pointer;
    padding: 0 2px;
    font-family: inherit;
    pointer-events: auto;
    transition: color 0.15s;
  }

  .close-btn:hover { color: #ff6b6b; }

  .section-label {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.3);
    margin-bottom: 6px;
  }

  section {
    display: flex;
    flex-direction: column;
  }

  .swatches {
    display: flex;
    gap: 8px;
  }

  .swatch {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    pointer-events: auto;
    transition: border-color 0.15s, transform 0.1s;
  }

  .swatch.active {
    border-color: rgba(255, 255, 255, 0.8);
    transform: scale(1.15);
  }

  .radio-group {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .radio-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.45);
    font-family: inherit;
    font-size: 11px;
    text-align: left;
    padding: 3px 0;
    cursor: pointer;
    pointer-events: auto;
    transition: color 0.15s;
  }

  .radio-btn.active { color: #e8e8e8; }
  .radio-btn.active::before { content: '● '; }
  .radio-btn:not(.active)::before { content: '○ '; }

  .toggle-row {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.6);
    font-family: inherit;
    font-size: 11px;
    text-align: left;
    padding: 4px 0;
    cursor: pointer;
    pointer-events: auto;
    transition: color 0.15s;
    width: 100%;
  }

  .toggle-row:hover { color: #e8e8e8; }

  .anchor-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }

  .anchor-btn {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 5px;
    color: rgba(255, 255, 255, 0.55);
    font-family: inherit;
    font-size: 10px;
    padding: 5px 6px;
    cursor: pointer;
    pointer-events: auto;
    text-align: left;
    transition: background 0.15s, color 0.15s;
  }

  .anchor-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #e8e8e8;
  }

  .divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.07);
  }
</style>
