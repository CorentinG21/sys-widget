<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import { load } from '@tauri-apps/plugin-store';

  import { metrics, cpuHistory, gpuHistory, startListening, stopListening } from '$lib/stores/metrics.svelte';

  // Cache the window handle — getCurrentWindow() is an IPC call, no need to repeat it.
  const appWindow = getCurrentWindow();
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
    // Flip left/up if the menu would overflow the window, then clamp to [0, max].
    const rawX = e.clientX + MENU_WIDTH  > window.innerWidth  ? e.clientX - MENU_WIDTH  : e.clientX;
    const rawY = e.clientY + MENU_HEIGHT > window.innerHeight ? e.clientY - MENU_HEIGHT : e.clientY;
    menuX = Math.max(0, Math.min(rawX, window.innerWidth  - MENU_WIDTH));
    menuY = Math.max(0, Math.min(rawY, window.innerHeight - MENU_HEIGHT));
    menuVisible = true;
  }

  function closeMenu() { menuVisible = false; }

  // ── Position persistence ─────────────────────────────────────────────────

  const STORE_PATH = 'config.json';
  const POS_KEY    = 'position';

  async function savePosition(): Promise<void> {
    try {
      const pos = await appWindow.outerPosition();
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
        await appWindow.setPosition({ type: 'Physical', x: pos.x, y: pos.y });
      }
    } catch (e) {
      console.warn('[store] restorePosition failed:', e);
    }
  }

  // ── Lifecycle ────────────────────────────────────────────────────────────

  onMount(async () => {
    await restorePosition();          // position first (window still hidden)
    await appWindow.setAlwaysOnBottom(true);
    await appWindow.show();           // reveal only after position is correct
    await appWindow.onMoved(savePosition);
    await startListening();

    // Listen for update notifications from the Rust backend.
    await listen<{ version: string }>('update-available', (event) => {
      updateVersion = event.payload.version;
    });
  });

  onDestroy(() => { stopListening(); });

  const ramDetail  = $derived(`${formatBytes(metrics.ram.used)} / ${formatBytes(metrics.ram.total)}`);
  // Don't show VRAM detail for iGPUs with shared memory (vram_total = 0)
  const vramDetail = $derived(
    metrics.gpu && metrics.gpu.vram_total > 0
      ? `${formatBytes(metrics.gpu.vram_used)} / ${formatBytes(metrics.gpu.vram_total)}`
      : ''
  );
  const cpuSubExtra = $derived(
    metrics.top_cpu
      ? `🔥 ${metrics.top_cpu.name} · ${metrics.top_cpu.cpu_percent.toFixed(0)}%`
      : ''
  );

  // ── Drag ─────────────────────────────────────────────────────────────────
  // mousedown bubbles from ALL children (even pointer-events:auto metric-rows)
  // up to .widget. We call startDragging() explicitly so any left-click anywhere
  // in the widget initiates hold-and-drag, regardless of what element was clicked.
  function onWidgetMouseDown(e: MouseEvent) {
    if (e.button === 0) {         // left button only — ignore right-click (context menu)
      appWindow.startDragging();
    }
  }
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
  role="application"
  aria-label="SysmonWidget"
  onmousedown={onWidgetMouseDown}
  oncontextmenu={onContextMenu}
>

  {#if updateVersion}
    <UpdateBanner version={updateVersion} />
  {/if}

  <MetricRow label="CPU" percent={metrics.cpu.percent} temp={metrics.cpu.temp} history={cpuHistory} subExtra={cpuSubExtra} />

  {#if metrics.gpu}
    <MetricRow label="GPU" percent={metrics.gpu.percent} temp={metrics.gpu.temp} detail={vramDetail} history={gpuHistory} />
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
