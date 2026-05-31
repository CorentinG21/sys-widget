<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { LogicalSize } from '@tauri-apps/api/dpi';
  import { listen } from '@tauri-apps/api/event';
  import { load } from '@tauri-apps/plugin-store';

  import { metrics, cpuHistory, gpuHistory, startListening, stopListening } from '$lib/stores/metrics.svelte';
  import { settings, loadSettings, applyToDocument } from '$lib/stores/settings.svelte';

  const appWindow = getCurrentWindow();
  import MetricRow    from '$lib/components/MetricRow.svelte';
  import DiskRow      from '$lib/components/DiskRow.svelte';
  import NetworkRow   from '$lib/components/NetworkRow.svelte';
  import ContextMenu  from '$lib/components/ContextMenu.svelte';
  import UpdateBanner from '$lib/components/UpdateBanner.svelte';
  import SettingsPanel from '$lib/components/SettingsPanel.svelte';
  import { formatBytes } from '$lib/utils/colors';

  // ── Update state ─────────────────────────────────────────────────────────

  let updateVersion = $state<string | null>(null);

  // ── Settings panel state ─────────────────────────────────────────────────

  let settingsOpen = $state(false);

  const WIDGET_W = 320;
  const PANEL_W  = 210;
  const GAP      = 8;
  const WINDOW_H = 600;

  let panelOnLeft = $state(false);

  async function openSettings() {
    // Check if there's room to the right; if not, put panel on the left
    const pos     = await appWindow.outerPosition();
    const monitor = await appWindow.currentMonitor();
    const scale   = monitor?.scaleFactor ?? 1;
    const mRight  = (monitor?.position.x ?? 0) + (monitor?.size.width ?? 1920);
    const expandedW = Math.round((WIDGET_W + GAP + PANEL_W) * scale);
    panelOnLeft = (pos.x + expandedW) > mRight;

    settingsOpen = true;
    await appWindow.setSize(new LogicalSize(WIDGET_W + GAP + PANEL_W, WINDOW_H));

    // If panel goes on the left, shift the window left to keep widget in place
    if (panelOnLeft) {
      const shiftPx = Math.round((PANEL_W + GAP) * scale);
      await appWindow.setPosition({ type: 'Physical', x: pos.x - shiftPx, y: pos.y });
    }
  }

  async function closeSettings() {
    const pos = await appWindow.outerPosition();
    const monitor = await appWindow.currentMonitor();
    const scale   = monitor?.scaleFactor ?? 1;

    settingsOpen = false;
    await appWindow.setSize(new LogicalSize(WIDGET_W, WINDOW_H));

    // Undo the left-shift if panel was on the left
    if (panelOnLeft) {
      const shiftPx = Math.round((PANEL_W + GAP) * scale);
      await appWindow.setPosition({ type: 'Physical', x: pos.x + shiftPx, y: pos.y });
    }
  }

  // ── Context menu state ───────────────────────────────────────────────────

  let menuVisible = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);

  const MENU_WIDTH  = 230;
  const MENU_HEIGHT = 210;

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
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
    await restorePosition();
    await appWindow.setAlwaysOnBottom(true);
    await appWindow.show();
    await appWindow.onMoved(savePosition);
    await appWindow.onCloseRequested(async () => { await savePosition(); });
    await startListening();
    await loadSettings();
    await applyToDocument();

    await listen<{ version: string }>('update-available', (event) => {
      updateVersion = event.payload.version;
    });
  });

  onDestroy(() => { stopListening(); });

  const ramDetail  = $derived(`${formatBytes(metrics.ram.used)} / ${formatBytes(metrics.ram.total)}`);
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

  function onWidgetMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    if (settings.locked) return;
    const target = e.target as HTMLElement;
    if (target.closest('button, input, a, [role="button"]')) return;
    appWindow.startDragging();
  }
</script>

<ContextMenu
  x={menuX}
  y={menuY}
  visible={menuVisible}
  updateVersion={updateVersion}
  onclose={closeMenu}
  onsaveposition={savePosition}
  onsettings={openSettings}
/>

<div class="app-layout" class:panel-left={panelOnLeft}>
  {#if settingsOpen && panelOnLeft}
    <SettingsPanel onclose={closeSettings} />
  {/if}

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

  {#if settingsOpen && !panelOnLeft}
    <SettingsPanel onclose={closeSettings} />
  {/if}
</div>

<style>
  .app-layout {
    display: flex;
    flex-direction: row;
    align-items: flex-start;
    gap: 8px;
    width: fit-content;
  }
</style>
