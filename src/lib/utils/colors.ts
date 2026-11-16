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
