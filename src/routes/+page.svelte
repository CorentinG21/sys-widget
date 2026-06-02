<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { availableMonitors, currentMonitor, getCurrentWindow } from '@tauri-apps/api/window';
  import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
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

  function pointInMonitor(x: number, y: number, monitor: Awaited<ReturnType<typeof currentMonitor>>) {
    if (!monitor) return false;
    const left = monitor.position.x;
    const top = monitor.position.y;
    const right = left + monitor.size.width;
    const bottom = top + monitor.size.height;
    return x >= left && x < right && y >= top && y < bottom;
  }

  // Returns monitor bounds in physical pixels. When the settings panel is open,
  // currentMonitor() can be skewed by the expanded window crossing screens, so
  // prefer the monitor containing the widget center.
  async function getMonitorBounds(widgetX?: number, widgetY?: number) {
    let monitor = null;
    if (typeof widgetX === 'number' && typeof widgetY === 'number') {
      monitor = (await availableMonitors()).find((m) => pointInMonitor(widgetX + 1, widgetY + 1, m)) ?? null;
    }
    monitor ??= await currentMonitor();
    if (monitor) {
      return {
        right:   monitor.position.x + monitor.size.width,
        originX: monitor.position.x,
        originY: monitor.position.y,
        width:   monitor.size.width,
        height:  monitor.size.height,
        dpr:     monitor.scaleFactor,
      };
    }
    // Fallback if currentMonitor() returns null
    const dpr = window.devicePixelRatio || 1;
    return {
      right:   Math.round(window.screen.width  * dpr),
      originX: 0,
      originY: 0,
      width:   Math.round(window.screen.width  * dpr),
      height:  Math.round(window.screen.height * dpr),
      dpr,
    };
  }

  async function openSettings() {
    const pos     = await appWindow.outerPosition();
    const bounds  = await getMonitorBounds(pos.x, pos.y);
    const dpr     = bounds.dpr;
    const expandedW = Math.round((WIDGET_W + GAP + PANEL_W) * dpr);

    panelOnLeft = (pos.x + expandedW) > bounds.right;
    settingsOpen = true;
    await appWindow.setSize(new LogicalSize(WIDGET_W + GAP + PANEL_W, WINDOW_H));

    if (panelOnLeft) {
      const shiftPx = Math.round((PANEL_W + GAP) * dpr);
      await appWindow.setPosition(new PhysicalPosition(pos.x - shiftPx, pos.y));
    }
  }

  async function closeSettings() {
    const pos = await appWindow.outerPosition();
    const dpr = (await getMonitorBounds()).dpr;

    settingsOpen = false;
    await appWindow.setSize(new LogicalSize(WIDGET_W, WINDOW_H));

    if (panelOnLeft) {
      const shiftPx = Math.round((PANEL_W + GAP) * dpr);
      await appWindow.setPosition(new PhysicalPosition(pos.x + shiftPx, pos.y));
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
        await appWindow.setPosition(new PhysicalPosition(pos.x, pos.y));
      }
    } catch (e) {
      console.warn('[store] restorePosition failed:', e);
    }
  }

  // ── Anchor widget to screen corner ──────────────────────────────────────

  async function anchorTo(corner: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right') {
    try {
      const pos = await appWindow.outerPosition();
      const widgetXNow = settingsOpen && panelOnLeft
        ? pos.x + Math.round((PANEL_W + GAP) * (window.devicePixelRatio || 1))
        : pos.x;
      const bounds  = await getMonitorBounds(widgetXNow, pos.y);
      const dpr     = bounds.dpr;
      const widgetW = Math.round(WIDGET_W * dpr);
      const m       = Math.round(12 * dpr);
      const winH    = Math.round(WINDOW_H * dpr);

      // Target position of the widget itself (not the full window)
      let widgetX: number, y: number;
      if      (corner === 'top-left')    { widgetX = bounds.originX + m;                           y = bounds.originY + m; }
      else if (corner === 'top-right')   { widgetX = bounds.originX + bounds.width - widgetW - m;  y = bounds.originY + m; }
      else if (corner === 'bottom-left') { widgetX = bounds.originX + m;                           y = bounds.originY + bounds.height - winH - m; }
      else                               { widgetX = bounds.originX + bounds.width - widgetW - m;  y = bounds.originY + bounds.height - winH - m; }

      if (settingsOpen) {
        // Right corners → panel must be on the left to stay on-screen
        const newPanelOnLeft = corner === 'top-right' || corner === 'bottom-right';
        const panelPx = Math.round((PANEL_W + GAP) * dpr);
        // If panel is on the left, the full window starts to the left of widgetX
        const windowX = newPanelOnLeft ? widgetX - panelPx : widgetX;
        await appWindow.setPosition(new PhysicalPosition(windowX, y));
        panelOnLeft = newPanelOnLeft;
      } else {
        await appWindow.setPosition(new PhysicalPosition(widgetX, y));
      }

      // Always persist the widget position (not the expanded window position)
      const store = await load(STORE_PATH);
      await store.set(POS_KEY, { x: widgetX, y });
      await store.save();
    } catch (e) {
      console.error('[anchor]', e);
    }
  }

  // ── Window layer ─────────────────────────────────────────────────────────

  async function applyWindowLayer() {
    if (settings.alwaysOnTop) {
      await appWindow.setAlwaysOnTop(true);
      await appWindow.setAlwaysOnBottom(false);
    } else {
      await appWindow.setAlwaysOnTop(false);
      await appWindow.setAlwaysOnBottom(true);
    }
  }

  // ── Lifecycle ────────────────────────────────────────────────────────────

  onMount(async () => {
    await restorePosition();
    await appWindow.show();
    await appWindow.onMoved(savePosition);
    await appWindow.onCloseRequested(async () => { await savePosition(); });
    await startListening();
    await loadSettings();
    await applyToDocument();
    // Apply saved poll interval to the Rust loop
    await invoke('set_poll_interval', { ms: settings.pollInterval * 1000 });
    // Apply saved window layer
    await applyWindowLayer();

    await listen<{ version: string }>('update-available', (event) => {
      updateVersion = event.payload.version;
    });

  });

  onDestroy(() => { stopListening(); });

  // ── Temperature conversion ────────────────────────────────────────────────

  function convertTemp(celsius: number | null | undefined): number | null {
    if (celsius == null) return null;
    if (settings.tempUnit === 'F') return Math.round(celsius * 9 / 5 + 32);
    if (settings.tempUnit === 'K') return Math.round(celsius + 273.15);
    return celsius;
  }

  const tempSuffix = $derived(
    settings.tempUnit === 'F' ? '°F' :
    settings.tempUnit === 'K' ? ' K' : '°C'
  );

  // True if at least one "top" row (CPU/GPU/RAM) is visible
  const hasTopRows  = $derived(settings.showCpu || settings.showGpu || settings.showRam);
  const hasDiskRows = $derived(settings.showDisks && metrics.disks.length > 0);

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

  let dragging = $state(false);

  function onWidgetMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    if (settings.locked) return;
    const target = e.target as HTMLElement;
    if (target.closest('button, input, a, [role="button"]')) return;
    dragging = true;
    appWindow.startDragging();
    // startDragging() hands off to the OS — mouseup never fires in the webview.
    // Reset the visual after a short delay so the outline doesn't get stuck.
    setTimeout(() => { dragging = false; }, 120);
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
    <SettingsPanel onclose={closeSettings} onanchor={anchorTo} />
  {/if}

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="widget"
    class:dragging={dragging}
    role="application"
    aria-label="SysmonWidget"
    onmousedown={onWidgetMouseDown}
    oncontextmenu={onContextMenu}
  >

    {#if updateVersion}
      <UpdateBanner version={updateVersion} />
    {/if}

    {#if settings.showCpu}
      <MetricRow label="CPU" percent={metrics.cpu.percent} temp={convertTemp(metrics.cpu.temp)} tempSuffix={tempSuffix} history={cpuHistory} subExtra={cpuSubExtra} />
    {/if}

    {#if settings.showGpu}
      {#if metrics.gpu}
        <MetricRow label="GPU" percent={metrics.gpu.percent} temp={convertTemp(metrics.gpu.temp)} tempSuffix={tempSuffix} detail={vramDetail} history={gpuHistory} />
      {:else}
        <MetricRow label="GPU" percent={0} na={true} />
      {/if}
    {/if}

    {#if settings.showRam}
      <MetricRow label="RAM" percent={metrics.ram.percent} detail={ramDetail} />
    {/if}

    {#if hasDiskRows}
      {#if hasTopRows}<div class="divider"></div>{/if}
      {#each metrics.disks as disk (disk.mount)}
        <DiskRow {disk} />
      {/each}
    {/if}

    {#if settings.showNetwork}
      {#if hasTopRows || hasDiskRows}<div class="divider"></div>{/if}
      <NetworkRow network={metrics.network} />
    {/if}

  </div>

  {#if settingsOpen && !panelOnLeft}
    <SettingsPanel onclose={closeSettings} onanchor={anchorTo} />
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
