import { describe, it, expect } from 'vitest';
import { connectionStatus, formatNetRate } from './network';

describe('formatNetRate', () => {
  it('shows an idle connection as 0.0 in kilobytes, never in bytes', () => {
    expect(formatNetRate(0, 'fr')).toBe('0,0 Ko/s');
    expect(formatNetRate(0, 'en')).toBe('0.0 KB/s');
    expect(formatNetRate(512, 'fr')).toBe('0,5 Ko/s');
  });

  it('uses the French decimal comma and units in French, English ones in English', () => {
    expect(formatNetRate(5017.6, 'fr')).toBe('4,9 Ko/s');
    expect(formatNetRate(5017.6, 'en')).toBe('4.9 KB/s');
    expect(formatNetRate(13_002_342.4, 'fr')).toBe('12,4 Mo/s');
    expect(formatNetRate(13_002_342.4, 'en')).toBe('12.4 MB/s');
  });

  it('switches to megabytes before reaching 1000 kilobytes, so it never shows "1000,0 Ko/s"', () => {
    expect(formatNetRate(1_023_948, 'fr')).toBe('999,9 Ko/s');
    expect(formatNetRate(1_024_000, 'fr')).toBe('1,0 Mo/s');
  });

  it('goes up to gigabytes for very fast links', () => {
    expect(formatNetRate(1.5 * 1_073_741_824, 'fr')).toBe('1,5 Go/s');
    expect(formatNetRate(1.5 * 1_073_741_824, 'en')).toBe('1.5 GB/s');
  });

  it('never gets wider than 10 characters, so the row does not shift', () => {
    for (let bytes = 1; bytes < 2e12; bytes *= 1.37) {
      expect(formatNetRate(bytes, 'fr').length, `for ${bytes} B/s`).toBeLessThanOrEqual(10);
    }
  });

  it('treats a negative or invalid value as no traffic', () => {
    expect(formatNetRate(-5, 'fr')).toBe('0,0 Ko/s');
    expect(formatNetRate(Number.NaN, 'fr')).toBe('0,0 Ko/s');
  });
});

describe('connectionStatus', () => {
  it('is offline when there is no latency measurement', () => {
    expect(connectionStatus(null)).toBe('offline');
  });

  it('is ok below 100 ms, which covers fiber, DSL and most 4G links', () => {
    expect(connectionStatus(0)).toBe('ok');
    expect(connectionStatus(46)).toBe('ok');
    expect(connectionStatus(99)).toBe('ok');
  });

  it('is slow from 100 ms', () => {
    expect(connectionStatus(100)).toBe('slow');
    expect(connectionStatus(650)).toBe('slow');
  });
});
