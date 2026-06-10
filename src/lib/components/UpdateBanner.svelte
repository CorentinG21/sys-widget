<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { settings } from '$lib/stores/settings.svelte';
  import { translations } from '$lib/i18n';
  const t = $derived(translations[settings.lang]);

  interface Props {
    version: string;
  }

  const { version }: Props = $props();

  let installing = $state(false);

  async function update() {
    installing = true;
    await invoke('install_update');
  }
</script>

<button class="update-banner" onclick={update} disabled={installing}>
  {#if installing}
    {t.installing}
  {:else}
    {t.newVersion}{version}{t.clickToUpdate}
  {/if}
</button>

<style>
  .update-banner {
    width: 100%;
    padding: 6px 10px;
    background: rgba(255, 209, 102, 0.15);
    border: 1px solid rgba(255, 209, 102, 0.35);
    border-radius: 6px;
    color: #ffd166;
    font-family: 'Consolas', monospace;
    font-size: 11px;
    text-align: center;
    cursor: pointer;
    transition: background 0.2s;
    pointer-events: auto;
  }

  .update-banner:hover:not(:disabled) {
    background: rgba(255, 209, 102, 0.25);
  }

  .update-banner:disabled {
    opacity: 0.7;
    cursor: wait;
  }
</style>
