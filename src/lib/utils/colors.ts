/**
 * Returns a CSS color var for a 0–100% usage value.
 *
 * < 70   → green  --color-ok
 * 70–89  → yellow --color-warn
 * ≥ 90   → red    --color-danger
 */
export function thresholdColor(percent: number): string {
  if (percent >= 90) return 'var(--color-danger)';
  if (percent >= 70) return 'var(--color-warn)';
  return 'var(--color-ok)';
}

/**
 * Returns upload and download color vars for a network rate in bytes/s.
 *
 * Rate (MB/s)  upload       download
 * < 1          green        cyan
 * 1–9          yellow       yellow
 * ≥ 10         red          red
 */
export function netColors(bytesPerSec: number): { upload: string; download: string } {
  const mbps = bytesPerSec / 1_048_576;
  if (mbps >= 10) return { upload: 'var(--color-danger)', download: 'var(--color-danger)' };
  if (mbps >= 1)  return { upload: 'var(--color-warn)',   download: 'var(--color-warn)' };
  return { upload: 'var(--color-ok)', download: 'var(--color-dl)' };
}

/**
 * Formats a bytes/s value as a human-readable rate string.
 *
 * Examples:
 *   0            → "0 B/s"
 *   512          → "512 B/s"
 *   1536         → "1.5 KB/s"
 *   1_572_864    → "1.5 MB/s"
 */
export function formatRate(bytesPerSec: number): string {
  if (bytesPerSec < 1_024) {
    return `${Math.round(bytesPerSec)} B/s`;
  }
  if (bytesPerSec < 1_048_576) {
    return `${(bytesPerSec / 1_024).toFixed(1)} KB/s`;
  }
  return `${(bytesPerSec / 1_048_576).toFixed(1)} MB/s`;
}

/**
 * Returns a CSS color var for a latency value in ms.
 *
 * < 30ms   → green  --color-ok
 * 30–100ms → yellow --color-warn
 * > 100ms  → red    --color-danger
 */
export function latencyColor(ms: number): string {
  if (ms > 100) return 'var(--color-danger)';
  if (ms > 30)  return 'var(--color-warn)';
  return 'var(--color-ok)';
}

/**
 * Formats bytes as a compact size string (used for RAM / VRAM display).
 *
 * Examples:
 *   0               → "0 B"
 *   1_073_741_824   → "1.0 GB"
 *   536_870_912     → "512 MB"
 */
export function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) {
    return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
  }
  if (bytes >= 1_048_576) {
    return `${Math.round(bytes / 1_048_576)} MB`;
  }
  if (bytes >= 1_024) {
    return `${Math.round(bytes / 1_024)} KB`;
  }
  return `${bytes} B`;
}
