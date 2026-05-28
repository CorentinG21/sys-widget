<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import { load } from '@tauri-apps/plugin-store';

  import { metrics, startListening, stopListening } from '$lib/stores/metrics.svelte';
  import MetricRow    from '$lib/components/MetricRow.svelte';
  import DiskRow      from '$lib/components/DiskRow.svelte';
  import NetworkRow   from '$lib/components/NetworkRow.svelte';
  import ContextMenu  from '$lib/components/ContextMenu.svelte';
  import UpdateBanner from '$lib/components/UpdateBanner.svelte';
  import { formatBytes } from '$lib/utils/colors';

  // ── Update state ─────────────────────────────────────────────────────────

  let updateVersion = $state<string | null>(null);

  // ── Context menu state ───────────────────────────────────────────────────

  let menuVisible = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);

  const MENU_WIDTH  = 230;   // min-width 200px + borders + "Rechercher une mise à jour" overflow
  const MENU_HEIGHT = 210;   // ~7 items × 26px + dividers + padding

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    menuX = e.clientX + MENU_WIDTH  > window.innerWidth  ? e.clientX - MENU_WIDTH  : e.clientX;
    menuY = e.clientY + MENU_HEIGHT > window.innerHeight ? e.clientY - MENU_HEIGHT : e.clientY;
    menuVisible = true;
  }

  function closeMenu() { menuVisible = false; }

  // ── Position persistence ─────────────────────────────────────────────────

  const STORE_PATH = 'config.json';
  const POS_KEY    = 'position';

  async function savePosition(): Promise<void> {
    try {
      const win = getCurrentWindow();
      const pos = await win.outerPosition();
      const store = await load(STORE_PATH);
      await store.set(POS_KEY, { x: pos.x, y: pos.y });
      await store.save();
    } catch (e) {
      console.warn('[store] savePosition failed:', e);
    }
  }

  async function restorePosition(): Promise<void> {
    try {
      const store = await load(STORE_PATH);
      const pos = await store.get<{ x: number; y: number }>(POS_KEY);
      if (pos && typeof pos.x === 'number' && typeof pos.y === 'number') {
        await getCurrentWindow().setPosition({ type: 'Physical', x: pos.x, y: pos.y });
      }
    } catch (e) {
      console.warn('[store] restorePosition failed:', e);
    }
  }

  // ── Lifecycle ────────────────────────────────────────────────────────────

  onMount(async () => {
    await getCurrentWindow().setAlwaysOnBottom(true);
    await restorePosition();
    await getCurrentWindow().onMoved(savePosition);
    await startListening();

    // Listen for update notifications from the Rust backend.
    await listen<{ version: string }>('update-available', (event) => {
      updateVersion = event.payload.version;
    });
  });

  onDestroy(() => { stopListening(); });

  const ramDetail  = $derived(`${formatBytes(metrics.ram.used)} / ${formatBytes(metrics.ram.total)}`);
  const vramDetail = $derived(
    metrics.gpu ? `${formatBytes(metrics.gpu.vram_used)} / ${formatBytes(metrics.gpu.vram_total)}` : ''
  );
</script>

<ContextMenu
  x={menuX}
  y={menuY}
  visible={menuVisible}
  updateVersion={updateVersion}
  onclose={closeMenu}
/>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="widget"
  data-tauri-drag-region
  role="application"
  aria-label="SysmonWidget"
  oncontextmenu={onContextMenu}
>

  {#if updateVersion}
    <UpdateBanner version={updateVersion} />
  {/if}

  <MetricRow label="CPU" percent={metrics.cpu.percent} temp={metrics.cpu.temp} />

  {#if metrics.gpu}
    <MetricRow label="GPU" percent={metrics.gpu.percent} temp={metrics.gpu.temp} detail={vramDetail} />
  {:else}
    <MetricRow label="GPU" percent={0} na={true} />
  {/if}

  <MetricRow label="RAM" percent={metrics.ram.percent} detail={ramDetail} />

  {#if metrics.disks.length > 0}
    <div class="divider"></div>
    {#each metrics.disks as disk (disk.mount)}
      <DiskRow {disk} />
    {/each}
  {/if}

  <div class="divider"></div>

  <NetworkRow network={metrics.network} />

</div>
