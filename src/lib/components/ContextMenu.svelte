<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    x: number;
    y: number;
    visible: boolean;
    onclose: () => void;
  }

  const { x, y, visible, onclose }: Props = $props();

  async function restart() {
    onclose();
    await invoke('restart_app');
  }

  async function quit() {
    onclose();
    await invoke('quit_app');
  }
</script>

{#if visible}
  <!-- Backdrop: clicking outside closes the menu -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={onclose}></div>

  <div
    class="context-menu"
    style="left: {x}px; top: {y}px;"
    role="menu"
  >
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
    min-width: 160px;
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
  }

  .menu-item:hover {
    background: rgba(255, 255, 255, 0.08);
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
