/**
 * Returns a CSS color for a 0–100% usage value.
 *
 * < 70   → green  #06d6a0
 * 70–89  → yellow #ffd166
 * ≥ 90   → red    #ff6b6b
 */
export function thresholdColor(percent: number): string {
  if (percent >= 90) return '#ff6b6b';
  if (percent >= 70) return '#ffd166';
  return '#06d6a0';
}

/**
 * Returns upload and download colors for a network rate in bytes/s.
 *
 * Rate (MB/s)  upload       download
 * < 1          green        cyan
 * 1–9          yellow       yellow
 * ≥ 10         red          red
 */
export function netColors(bytesPerSec: number): { upload: string; download: string } {
  const mbps = bytesPerSec / 1_048_576;
  if (mbps >= 10) return { upload: '#ff6b6b', download: '#ff6b6b' };
  if (mbps >= 1)  return { upload: '#ffd166', download: '#ffd166' };
  return { upload: '#06d6a0', download: '#74d7f7' };
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
