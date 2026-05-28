<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import { load } from '@tauri-apps/plugin-store';

  import { metrics, startListening, stopListening } from '$lib/stores/metrics.svelte';
  import MetricRow from '$lib/components/MetricRow.svelte';
  import DiskRow from '$lib/components/DiskRow.svelte';
  import NetworkRow from '$lib/components/NetworkRow.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import { formatBytes } from '$lib/utils/colors';

  // ── Context menu state ───────────────────────────────────────────────────

  let menuVisible = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);

  const MENU_WIDTH  = 190; // matches ContextMenu min-width
  const MENU_HEIGHT = 110; // approximate height (3 items)

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    // Flip left if menu would overflow the right edge of the window.
    menuX = e.clientX + MENU_WIDTH  > window.innerWidth  ? e.clientX - MENU_WIDTH  : e.clientX;
    // Flip up if menu would overflow the bottom edge.
    menuY = e.clientY + MENU_HEIGHT > window.innerHeight ? e.clientY - MENU_HEIGHT : e.clientY;
    menuVisible = true;
  }

  function closeMenu() {
    menuVisible = false;
  }

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
    // Always-on-bottom: sit behind desktop icons, above wallpaper.
    await getCurrentWindow().setAlwaysOnBottom(true);

    // Restore last position.
    await restorePosition();

    // Save position when the window is moved.
    await getCurrentWindow().onMoved(savePosition);

    // Start receiving metrics from the Rust backend.
    await startListening();
  });

  onDestroy(() => {
    stopListening();
  });

  const ramDetail = $derived(
    `${formatBytes(metrics.ram.used)} / ${formatBytes(metrics.ram.total)}`
  );

  const vramDetail = $derived(
    metrics.gpu
      ? `${formatBytes(metrics.gpu.vram_used)} / ${formatBytes(metrics.gpu.vram_total)}`
      : ''
  );
</script>

<ContextMenu x={menuX} y={menuY} visible={menuVisible} onclose={closeMenu} />

<!-- data-tauri-drag-region makes the entire widget draggable -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="widget"
  data-tauri-drag-region
  role="application"
  aria-label="SysmonWidget"
  oncontextmenu={onContextMenu}
>

  <!-- CPU -->
  <MetricRow
    label="CPU"
    percent={metrics.cpu.percent}
    temp={metrics.cpu.temp}
  />

  <!-- GPU (only when detected) -->
  {#if metrics.gpu}
    <MetricRow
      label="GPU"
      percent={metrics.gpu.percent}
      temp={metrics.gpu.temp}
      detail={vramDetail}
    />
  {/if}

  <!-- RAM -->
  <MetricRow
    label="RAM"
    percent={metrics.ram.percent}
    detail={ramDetail}
  />

  {#if metrics.disks.length > 0}
    <div class="divider"></div>

    <!-- Disks -->
    {#each metrics.disks as disk (disk.mount)}
      <DiskRow {disk} />
    {/each}
  {/if}

  <div class="divider"></div>

  <!-- Network -->
  <NetworkRow network={metrics.network} />

</div>
