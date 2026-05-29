import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ─── Types matching Rust MetricsPayload ──────────────────────────────────────

export interface CpuMetrics {
  percent: number;
  temp: number | null;
}

export interface GpuMetrics {
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

export interface TopProcess {
  name: string;
  /** % of total CPU (0–100), normalised by core count. */
  cpu_percent: number;
}

export interface MetricsPayload {
  cpu: CpuMetrics;
  gpu: GpuMetrics | null;
  ram: RamMetrics;
  disks: DiskInfo[];
  network: NetworkMetrics;
  top_cpu: TopProcess | null;
}

// ─── Reactive state (Svelte 5 $state) ────────────────────────────────────────

export const metrics = $state<MetricsPayload>({
  cpu:     { percent: 0, temp: null },
  gpu:     null,
  ram:     { percent: 0, used: 0, total: 0 },
  disks:   [],
  network: { upload: 0, download: 0 },
  top_cpu: null,
});

// ─── Sparkline history (last 15 ticks = 30 s at 2 s interval) ────────────────

const MAX_HISTORY = 15;

export const cpuHistory = $state<number[]>([]);
export const gpuHistory = $state<number[]>([]);

function pushHistory(arr: number[], value: number) {
  arr.push(value);
  if (arr.length > MAX_HISTORY) arr.shift();
}

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
    metrics.top_cpu = p.top_cpu;

    pushHistory(cpuHistory, p.cpu.percent);
    pushHistory(gpuHistory, p.gpu ? p.gpu.percent : 0);
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
