<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy, tick } from 'svelte';
  import { availableMonitors, currentMonitor, getCurrentWindow, type Monitor } from '@tauri-apps/api/window';
  import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { register, unregister } from '@tauri-apps/plugin-global-shortcut';
  import { load } from '@tauri-apps/plugin-store';

  import { metrics, metricsStatus, cpuHistory, gpuHistory, cpuTempMax, gpuTempMax, startListening, stopListening } from '$lib/stores/metrics.svelte';
  import { settings, loadSettings, applyToDocument } from '$lib/stores/settings.svelte';

  const appWindow = getCurrentWindow();
  import MetricRow    from '$lib/components/MetricRow.svelte';
  import DiskRow      from '$lib/components/DiskRow.svelte';
  import NetworkRow   from '$lib/components/NetworkRow.svelte';
  import ContextMenu  from '$lib/components/ContextMenu.svelte';
  import UpdateBanner from '$lib/components/UpdateBanner.svelte';
  import SettingsPanel from '$lib/components/SettingsPanel.svelte';
  import { formatBytes } from '$lib/utils/colors';
  import { CLICK_THROUGH_SHORTCUT } from '$lib/shortcuts';
  import {
    anchorPosition, closestMonitor, ensureOnScreen, growsUpward, isCorner,
    panelGoesLeft, placeMenu, widgetOrigin, windowOrigin,
    type Corner, type MonitorArea, type Point, type Size,
  } from '$lib/utils/geometry';

  // ── Update state ─────────────────────────────────────────────────────────

  let updateVersion = $state<string | null>(null);

  // ── Window geometry ──────────────────────────────────────────────────────
  //
  // The window is always exactly as large as its content (widget, plus the
  // settings panel or the room the context menu needs), so no invisible area
  // swallows clicks meant for the desktop.
  //
  // Source of truth: `widgetPos`, the widget's top-left on screen in physical
  // pixels. The window rectangle is derived from it, never the other way round
  // (except when the user drags the window). That is also what gets saved.

  const PANEL_W = 210;   // CSS px, must match SettingsPanel's width
  const GAP     = 8;     // CSS px, must match .app-layout's gap
  const MARGIN  = 12;    // CSS px between an anchored widget and the screen edge

  const STORE_PATH = 'config.json';
  const POS_KEY    = 'position';
  const ANCHOR_KEY = 'anchor';

  let layoutEl = $state<HTMLDivElement>();
  let widgetEl = $state<HTMLDivElement>();

  let settingsOpen = $state(false);
  /** Settings panel goes to the left of the widget (no room on the right). */
  let panelOnLeft = $state(false);
  /** Extra content (panel, menu room) extends above the widget instead of below. */
  let growUp = $state(false);

  let widgetPos: Point | null = null;
  /** Corner the widget is pinned to; re-applied whenever its size changes. Cleared by a manual drag. */
  let anchor: Corner | null = null;
  let monitors: MonitorArea[] = [];
  /** Last rectangle we gave the window: position in physical px, size in CSS px. */
  let lastWindow: { x: number; y: number; w: number; h: number } | null = null;
  let windowShown = false;

  function toArea(m: Monitor): MonitorArea {
    const work = m.workArea ?? { position: m.position, size: m.size };
    return {
      bounds:   { x: m.position.x,    y: m.position.y,    width: m.size.width,    height: m.size.height },
      workArea: { x: work.position.x, y: work.position.y, width: work.size.width, height: work.size.height },
    };
  }

  async function refreshMonitors(): Promise<void> {
    try {
      let list = await availableMonitors();
      if (list.length === 0) {
        const current = await currentMonitor();
        if (current) list = [current];
      }
      monitors = list.map(toArea);
    } catch (e) {
      console.warn('[geometry] monitors unavailable:', e);
    }
  }

  function widgetSizePx(scale: number): Size {
    const r = widgetEl!.getBoundingClientRect();
    return { width: Math.round(r.width * scale), height: Math.round(r.height * scale) };
  }

  function centreOf(pos: Point, size: Size): Point {
    return { x: pos.x + size.width / 2, y: pos.y + size.height / 2 };
  }

  /** Moves `widgetPos` into the pinned corner, using the widget's real size and the work area. */
  function applyAnchor(scale: number): void {
    if (!anchor || !widgetPos || !widgetEl) return;
    const size = widgetSizePx(scale);
    const monitor = closestMonitor(monitors, centreOf(widgetPos, size));
    if (!monitor) return;
    const next = anchorPosition(anchor, monitor.workArea, size, Math.round(MARGIN * scale));
    if (next.x !== widgetPos.x || next.y !== widgetPos.y) {
      widgetPos = next;
      schedulePersist();
    }
  }

  /** Decides on which side of the widget extra content goes, from where the widget is on its monitor. */
  function updatePlacement(scale: number): void {
    if (!widgetPos || !widgetEl) return;
    const size = widgetSizePx(scale);
    const area = closestMonitor(monitors, centreOf(widgetPos, size))?.workArea;
    if (!area) return;
    panelOnLeft = panelGoesLeft(widgetPos.x, size.width, Math.round((PANEL_W + GAP) * scale), area);
    growUp = growsUpward(widgetPos.y, size.height, area);
  }

  let syncing = false;
  let syncQueued = false;

  /** Resizes and moves the window so it wraps the content while the widget stays put. */
  async function syncWindow(): Promise<void> {
    if (syncing) { syncQueued = true; return; }
    syncing = true;
    try {
      do {
        syncQueued = false;
        await tick();
        await applyGeometry();
      } while (syncQueued);
    } catch (e) {
      console.error('[geometry] sync failed:', e);
    } finally {
      syncing = false;
    }
  }

  async function applyGeometry(): Promise<void> {
    // While the user drags the window the OS owns its position: wait for the drop.
    if (!layoutEl || !widgetEl || !widgetPos || userMoving) return;

    const scale = await appWindow.scaleFactor();
    applyAnchor(scale);

    const layout = layoutEl.getBoundingClientRect();
    const widget = widgetEl.getBoundingClientRect();
    if (layout.width < 1 || layout.height < 1) return;   // not laid out yet

    const w = Math.ceil(layout.right);
    const h = Math.ceil(layout.bottom);
    const origin = windowOrigin(widgetPos, { x: widget.left, y: widget.top }, scale);

    const resized = !lastWindow || lastWindow.w !== w || lastWindow.h !== h;
    const moved   = !lastWindow || lastWindow.x !== origin.x || lastWindow.y !== origin.y;
    if (!resized && !moved) return;

    // Recorded before the calls: onWindowMoved() uses it to recognise our own moves.
    lastWindow = { x: origin.x, y: origin.y, w, h };
    if (resized) await appWindow.setSize(new LogicalSize(w, h));
    if (moved)   await appWindow.setPosition(new PhysicalPosition(origin.x, origin.y));
    // setSize can reset TOPMOST / BOTTOM on Windows
    if (resized && windowShown) await applyWindowLayer();
  }

  // ── Position persistence ─────────────────────────────────────────────────

  let persistTimer: ReturnType<typeof setTimeout> | undefined;

  function schedulePersist(): void {
    clearTimeout(persistTimer);
    persistTimer = setTimeout(() => { void persistPosition(); }, 400);
  }

  /** Saves the widget position (never the expanded window's) and the pinned corner. */
  async function persistPosition(): Promise<void> {
    clearTimeout(persistTimer);
    if (!widgetPos) return;
    try {
      const store = await load(STORE_PATH);
      await store.set(POS_KEY, { x: widgetPos.x, y: widgetPos.y });
      await store.set(ANCHOR_KEY, anchor);
      await store.save();
    } catch (e) {
      console.warn('[store] persistPosition failed:', e);
    }
  }

  /** Restores the saved position (pulled back on screen if its monitor is gone) and sizes the window. */
  async function initGeometry(): Promise<void> {
    await refreshMonitors();
    const scale = await appWindow.scaleFactor();

    let saved: Point | null = null;
    try {
      const store = await load(STORE_PATH);
      const pos = await store.get<{ x: number; y: number }>(POS_KEY);
      if (pos && typeof pos.x === 'number' && typeof pos.y === 'number') saved = { x: pos.x, y: pos.y };
      const savedAnchor = await store.get<unknown>(ANCHOR_KEY);
      anchor = isCorner(savedAnchor) ? savedAnchor : null;
    } catch (e) {
      console.warn('[store] restore failed:', e);
    }

    await tick();
    if (saved && widgetEl) {
      widgetPos = ensureOnScreen(saved, widgetSizePx(scale), monitors);
    } else {
      // First launch: keep wherever the OS put the window.
      const pos = await appWindow.outerPosition();
      widgetPos = { x: pos.x, y: pos.y };
    }
    applyAnchor(scale);
    updatePlacement(scale);
    await syncWindow();
  }

  // ── Window moved (user drag) ─────────────────────────────────────────────

  let userMoving = false;
  let moveTimer: ReturnType<typeof setTimeout> | undefined;

  function isOurMove(pos: Point): boolean {
    return !!lastWindow && Math.abs(pos.x - lastWindow.x) <= 1 && Math.abs(pos.y - lastWindow.y) <= 1;
  }

  function onWindowMoved(pos: Point): void {
    if (isOurMove(pos)) return;          // echo of our own setPosition
    userMoving = true;
    clearTimeout(moveTimer);
    moveTimer = setTimeout(() => { void onUserMoveEnd(); }, 300);   // one handling per drag, not per pixel
  }

  async function onUserMoveEnd(): Promise<void> {
    userMoving = false;
    try {
      const pos = await appWindow.outerPosition();
      if (!isOurMove(pos) && widgetEl) {
        const scale = await appWindow.scaleFactor();
        const widget = widgetEl.getBoundingClientRect();
        widgetPos = widgetOrigin({ x: pos.x, y: pos.y }, { x: widget.left, y: widget.top }, scale);
        if (lastWindow) lastWindow = { ...lastWindow, x: pos.x, y: pos.y };
        anchor = null;                   // a manual placement replaces the pinned corner
        await refreshMonitors();
        updatePlacement(scale);
        await persistPosition();
      }
    } catch (e) {
      console.warn('[geometry] move handling failed:', e);
    }
    await syncWindow();
  }

  // ── Anchor widget to screen corner ──────────────────────────────────────

  async function anchorTo(corner: Corner) {
    try {
      anchor = corner;
      await refreshMonitors();
      const scale = await appWindow.scaleFactor();
      applyAnchor(scale);
      updatePlacement(scale);
      await syncWindow();
      await persistPosition();
    } catch (e) {
      console.error('[anchor]', e);
    }
  }

  // ── Settings panel ───────────────────────────────────────────────────────

  async function openSettings() {
    try {
      await refreshMonitors();
      updatePlacement(await appWindow.scaleFactor());
    } catch (e) {
      console.warn('[geometry] placement failed:', e);
    }
    settingsOpen = true;
    await syncWindow();
  }

  async function closeSettings() {
    settingsOpen = false;
    await syncWindow();
  }

  // ── Context menu ─────────────────────────────────────────────────────────

  let menuVisible = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuEl = $state<HTMLElement | null>(null);
  /** Minimum layout height while the menu is open, so the window is tall enough to show it. */
  let menuRoom = $state(0);

  async function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    if (!layoutEl) return;
    const click = { x: e.clientX, y: e.clientY };
    menuX = click.x;
    menuY = click.y;
    menuVisible = true;
    await tick();                        // the menu is in the DOM now: measure it

    const menu = { width: menuEl?.offsetWidth ?? 230, height: menuEl?.offsetHeight ?? 300 };
    const layout = layoutEl.getBoundingClientRect();
    const height = Math.max(layout.height, menu.height);
    const extra  = height - layout.height;
    menuRoom = extra > 0 ? height : 0;

    // When growing upward the extra room is added above the widget, which
    // pushes the clicked point down by the same amount inside the window.
    const pos = placeMenu(
      { x: click.x, y: click.y + (growUp ? extra : 0) },
      menu,
      { width: Math.max(layout.width, menu.width), height },
    );
    menuX = pos.x;
    menuY = pos.y;
    await syncWindow();
  }

  function closeMenu() {
    menuVisible = false;
    menuRoom = 0;
    void syncWindow();
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

  // ── Click-through ─────────────────────────────────────────────────────────

  async function toggleClickThrough() {
    settings.clickThrough = !settings.clickThrough;
    await appWindow.setIgnoreCursorEvents(settings.clickThrough);
  }

  // ── Lifecycle ────────────────────────────────────────────────────────────

  let resizeObserver: ResizeObserver | undefined;
  const unlisteners: UnlistenFn[] = [];
  let shortcutRegistered = false;

  function suppressNativeMenu(e: MouseEvent) { e.preventDefault(); }

  onMount(async () => {
    // Everything before show() is guarded: a failure here must never leave the widget invisible.
    try {
      await loadSettings();
    } catch (e) {
      console.error('[settings] load failed:', e);
    }
    await applyToDocument();
    try {
      await initGeometry();
    } catch (e) {
      console.error('[geometry] init failed:', e);
    }
    await appWindow.show();
    windowShown = true;

    resizeObserver = new ResizeObserver(() => { void syncWindow(); });
    if (layoutEl) resizeObserver.observe(layoutEl);
    if (widgetEl) resizeObserver.observe(widgetEl);

    unlisteners.push(await appWindow.onMoved(({ payload }) => onWindowMoved(payload)));
    unlisteners.push(await appWindow.onScaleChanged(() => { void syncWindow(); }));
    unlisteners.push(await appWindow.onCloseRequested(async () => { await persistPosition(); }));
    await startListening();
    // Apply saved poll interval to the Rust loop
    await invoke('set_poll_interval', { ms: settings.pollInterval * 1000 });
    // Apply saved window layer
    await applyWindowLayer();

    unlisteners.push(await listen<{ version: string }>('update-available', (event) => {
      updateVersion = event.payload.version;
    }));
    // Tray → "Réinitialiser la position": brings back a widget lost off-screen.
    unlisteners.push(await listen('reset-position', () => { void anchorTo('top-right'); }));

    // Re-assert window layer when the widget regains focus
    window.addEventListener('focus', applyWindowLayer);

    // The WebView's own menu (Reload, Print…) must not show up on the panel or backdrop.
    if (!import.meta.env.DEV) document.addEventListener('contextmenu', suppressNativeMenu);

    // Global shortcut to toggle click-through — fire only on key down, not key up.
    // Another application may already own the combination: the widget works without it.
    try {
      await register(CLICK_THROUGH_SHORTCUT, (event) => {
        if (event.state === 'Pressed') toggleClickThrough();
      });
      shortcutRegistered = true;
    } catch (e) {
      console.warn(`[shortcut] could not register ${CLICK_THROUGH_SHORTCUT}:`, e);
    }
  });

  onDestroy(async () => {
    stopListening();
    resizeObserver?.disconnect();
    clearTimeout(moveTimer);
    clearTimeout(persistTimer);
    for (const unlisten of unlisteners) unlisten();
    window.removeEventListener('focus', applyWindowLayer);
    document.removeEventListener('contextmenu', suppressNativeMenu);
    if (shortcutRegistered) await unregister(CLICK_THROUGH_SHORTCUT);
  });

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

  const cpuTempMaxConverted = $derived(convertTemp(cpuTempMax.value));
  const gpuTempMaxConverted = $derived(convertTemp(gpuTempMax.value));

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
  bind:el={menuEl}
  onclose={closeMenu}
  onsaveposition={persistPosition}
  onsettings={openSettings}
/>

<div
  class="app-layout"
  class:panel-left={panelOnLeft}
  class:grow-up={growUp}
  style:min-height={menuRoom ? `${menuRoom}px` : null}
  bind:this={layoutEl}
>
  {#if settingsOpen && panelOnLeft}
    <SettingsPanel onclose={closeSettings} onanchor={anchorTo} />
  {/if}

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="widget"
    class:dragging={dragging}
    role="application"
    aria-label="SysmonWidget"
    bind:this={widgetEl}
    onmousedown={onWidgetMouseDown}
    oncontextmenu={onContextMenu}
  >

    {#if updateVersion}
      <UpdateBanner version={updateVersion} />
    {/if}

    {#if settings.showCpu}
      <MetricRow label="CPU" percent={metrics.cpu.percent} temp={convertTemp(metrics.cpu.temp)} tempMax={cpuTempMaxConverted} tempSuffix={tempSuffix} history={cpuHistory} subExtra={cpuSubExtra} />
    {/if}

    {#if settings.showGpu}
      {#if metrics.gpu}
        <MetricRow label="GPU" percent={metrics.gpu.percent} temp={convertTemp(metrics.gpu.temp)} tempMax={gpuTempMaxConverted} tempSuffix={tempSuffix} detail={vramDetail} history={gpuHistory} />
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
      <NetworkRow network={metrics.network} ready={metricsStatus.received} />
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
    /* max-content: the layout is measured to size the window, so it must not
       depend on the window's current width. */
    width: max-content;
  }

  /* Widget in the lower half of the screen: the panel and the menu's extra
     room extend above it, so it stays where it is. */
  .app-layout.grow-up {
    align-items: flex-end;
  }
</style>
