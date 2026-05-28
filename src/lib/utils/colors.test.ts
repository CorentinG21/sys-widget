import { describe, it, expect } from 'vitest';
import { thresholdColor, netColors, formatRate, formatBytes } from './colors';

describe('thresholdColor', () => {
  it('returns green below 70', () => {
    expect(thresholdColor(0)).toBe('#06d6a0');
    expect(thresholdColor(69.9)).toBe('#06d6a0');
  });

  it('returns yellow from 70 to 89', () => {
    expect(thresholdColor(70)).toBe('#ffd166');
    expect(thresholdColor(89.9)).toBe('#ffd166');
  });

  it('returns red at 90 and above', () => {
    expect(thresholdColor(90)).toBe('#ff6b6b');
    expect(thresholdColor(100)).toBe('#ff6b6b');
  });
});

describe('netColors', () => {
  it('returns green/cyan below 1 MB/s', () => {
    const c = netColors(0);
    expect(c.upload).toBe('#06d6a0');
    expect(c.download).toBe('#74d7f7');
  });

  it('returns green/cyan just below 1 MB/s', () => {
    const c = netColors(1_048_575);
    expect(c.upload).toBe('#06d6a0');
    expect(c.download).toBe('#74d7f7');
  });

  it('returns yellow/yellow from 1 to <10 MB/s', () => {
    const c = netColors(1_048_576);       // exactly 1 MB/s
    expect(c.upload).toBe('#ffd166');
    expect(c.download).toBe('#ffd166');
  });

  it('returns red/red at 10 MB/s and above', () => {
    const c = netColors(10 * 1_048_576);
    expect(c.upload).toBe('#ff6b6b');
    expect(c.download).toBe('#ff6b6b');
  });
});

describe('formatRate', () => {
  it('formats 0 as "0 B/s"', () => {
    expect(formatRate(0)).toBe('0 B/s');
  });

  it('formats bytes below 1 KB', () => {
    expect(formatRate(512)).toBe('512 B/s');
  });

  it('formats KB/s with one decimal', () => {
    expect(formatRate(1536)).toBe('1.5 KB/s');
  });

  it('formats MB/s with one decimal', () => {
    expect(formatRate(1_572_864)).toBe('1.5 MB/s');
  });

  it('formats exactly 1 KB/s', () => {
    expect(formatRate(1_024)).toBe('1.0 KB/s');
  });
});

describe('formatBytes', () => {
  it('formats 0 bytes', () => {
    expect(formatBytes(0)).toBe('0 B');
  });

  it('formats GB with one decimal', () => {
    expect(formatBytes(1_073_741_824)).toBe('1.0 GB');
    expect(formatBytes(8 * 1_073_741_824)).toBe('8.0 GB');
  });

  it('formats MB rounded', () => {
    expect(formatBytes(536_870_912)).toBe('512 MB');
  });

  it('formats KB rounded', () => {
    expect(formatBytes(2_048)).toBe('2 KB');
  });
});
