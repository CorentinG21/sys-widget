import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ─── Types matching Rust MetricsPayload ──────────────────────────────────────

export interface CpuMetrics {
  percent: number;
  temp: number | null;
}

export interface GpuMetrics {
  name: string;
  percent: number;
  temp: number | null;
  vram_used: number;
  vram_total: number;
}

export interface RamMetrics {
  percent: number;
  used: number;
  total: number;
}

export interface DiskInfo {
  mount: string;
  percent: number;
  used: number;
  total: number;
}

export interface NetworkMetrics {
  upload: number;
  download: number;
}

export interface MetricsPayload {
  cpu: CpuMetrics;
  gpu: GpuMetrics | null;
  ram: RamMetrics;
  disks: DiskInfo[];
  network: NetworkMetrics;
}

// ─── Reactive state (Svelte 5 $state) ────────────────────────────────────────

export const metrics = $state<MetricsPayload>({
  cpu:     { percent: 0, temp: null },
  gpu:     null,
  ram:     { percent: 0, used: 0, total: 0 },
  disks:   [],
  network: { upload: 0, download: 0 },
});

// ─── Listener lifecycle ───────────────────────────────────────────────────────

let unlisten: UnlistenFn | null = null;

/**
 * Subscribe to the "metrics-updated" Tauri event and update reactive state.
 * Call once from the root component's `onMount`.
 */
export async function startListening(): Promise<void> {
  unlisten = await listen<MetricsPayload>('metrics-updated', (event) => {
    const p = event.payload;
    metrics.cpu     = p.cpu;
    metrics.gpu     = p.gpu;
    metrics.ram     = p.ram;
    metrics.disks   = p.disks;
    metrics.network = p.network;
  });
}

/**
 * Unsubscribe from the Tauri event.
 * Call from the root component's `onDestroy` if hot-reload is needed.
 */
export function stopListening(): void {
  unlisten?.();
  unlisten = null;
}
