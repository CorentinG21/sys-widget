import { describe, it, expect } from 'vitest';
import { thresholdColor, formatBytes } from './colors';

describe('thresholdColor', () => {
  it('returns green below 70', () => {
    expect(thresholdColor(0)).toBe('var(--color-ok)');
    expect(thresholdColor(69.9)).toBe('var(--color-ok)');
  });

  it('returns yellow from 70 to 89', () => {
    expect(thresholdColor(70)).toBe('var(--color-warn)');
    expect(thresholdColor(89.9)).toBe('var(--color-warn)');
  });

  it('returns red at 90 and above', () => {
    expect(thresholdColor(90)).toBe('var(--color-danger)');
    expect(thresholdColor(100)).toBe('var(--color-danger)');
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
