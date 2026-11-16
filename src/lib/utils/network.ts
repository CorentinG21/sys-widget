import type { Lang } from '$lib/i18n';

export type ConnectionStatus = 'ok' | 'slow' | 'offline';

/** Latency (TCP connect time) from which the connection is shown as slow. */
const SLOW_LATENCY_MS = 100;

const UNITS: Record<Lang, string[]> = {
  fr: ['Ko/s', 'Mo/s', 'Go/s', 'To/s'],
  en: ['KB/s', 'MB/s', 'GB/s', 'TB/s'],
};

/**
 * Network rate for display: kilobytes at least, one decimal, unit in the
 * interface language. The next unit takes over before 1000, so the text is
 * at most 10 characters ("999,9 Mo/s") and the row never shifts.
 */
export function formatNetRate(bytesPerSec: number, lang: Lang): string {
  let value = Number.isFinite(bytesPerSec) && bytesPerSec > 0 ? bytesPerSec / 1024 : 0;
  const units = UNITS[lang];
  let unit = 0;
  // Compare the rounded value: 999.96 would otherwise print as "1000,0".
  while (unit < units.length - 1 && Number(value.toFixed(1)) >= 1000) {
    value /= 1024;
    unit++;
  }
  const text = value.toFixed(1);
  return `${lang === 'fr' ? text.replace('.', ',') : text} ${units[unit]}`;
}

/** Plain-language state of the connection, from the measured latency. */
export function connectionStatus(latencyMs: number | null): ConnectionStatus {
  if (latencyMs == null) return 'offline';
  return latencyMs < SLOW_LATENCY_MS ? 'ok' : 'slow';
}
