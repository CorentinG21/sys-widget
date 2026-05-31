<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import { onMount } from 'svelte';

  interface Props {
    x: number;
    y: number;
    visible: boolean;
    updateVersion: string | null;
    onclose: () => void;
    /** Called before any action that closes/restarts the app so position is saved first. */
    onsaveposition: () => Promise<void>;
  }

  const { x, y, visible, updateVersion, onclose, onsaveposition }: Props = $props();

  let version = $state('…');
  let startupEnabled = $state(false);

  onMount(async () => {
    version = await getVersion();
    startupEnabled = await invoke<boolean>('startup_is_registered');
  });

  async function toggleStartup() {
    startupEnabled = await invoke<boolean>('startup_toggle');
  }

  async function checkUpdate() {
    onclose();
    await invoke('check_update');
  }

  async function installUpdate() {
    onclose();
    await onsaveposition();          // persist position before the installer replaces the app
    await invoke('install_update');
  }

  async function restart() {
    onclose();
    await onsaveposition();          // persist position before restart_app exits immediately
    await invoke('restart_app');
  }

  async function quit() {
    onclose();
    await onsaveposition();          // persist on clean quit too
    await invoke('quit_app');
  }
</script>

{#if visible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={onclose}></div>

  <div class="context-menu" style="left: {x}px; top: {y}px;" role="menu">

    <div class="menu-version" role="menuitem" aria-disabled="true">
      SysmonWidget v{version}
    </div>

    <div class="menu-divider"></div>

    {#if updateVersion}
      <button class="menu-item menu-item--update" role="menuitem" onclick={installUpdate}>
        Mettre à jour v{updateVersion}
      </button>
    {/if}

    <button class="menu-item" role="menuitem" onclick={checkUpdate}>
      Rechercher une mise à jour
    </button>

    <div class="menu-divider"></div>

    <button class="menu-item" role="menuitem" onclick={toggleStartup}>
      {startupEnabled ? '✓' : '○'} Démarrer avec Windows
    </button>

    <button class="menu-item" role="menuitem" onclick={restart}>
      Redémarrer
    </button>

    <div class="menu-divider"></div>

    <button class="menu-item menu-item--danger" role="menuitem" onclick={quit}>
      Quitter
    </button>

  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 99;
  }

  .context-menu {
    position: fixed;
    z-index: 100;
    min-width: 200px;
    background: rgba(18, 18, 18, 0.92);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 8px;
    padding: 4px 0;
    font-family: 'Consolas', monospace;
    font-size: 12px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }

  .menu-version {
    padding: 7px 14px;
    color: rgba(255, 255, 255, 0.35);
    font-size: 11px;
    cursor: default;
    user-select: none;
  }

  .menu-item {
    display: block;
    width: 100%;
    padding: 7px 14px;
    background: transparent;
    border: none;
    color: #e8e8e8;
    font-family: inherit;
    font-size: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
    pointer-events: auto;
  }

  .menu-item:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .menu-item--update {
    color: #ffd166;
  }

  .menu-item--update:hover {
    background: rgba(255, 209, 102, 0.12);
  }

  .menu-item--danger:hover {
    background: rgba(255, 107, 107, 0.15);
    color: #ff6b6b;
  }

  .menu-divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.07);
    margin: 3px 0;
  }
</style>
