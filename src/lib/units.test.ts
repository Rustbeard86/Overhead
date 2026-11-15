import { describe, expect, it } from 'vitest';
import { altHundreds, fmtAgo, fmtAlt, fmtDist, fmtSpeed, fmtVRate } from './units';
import type { Settings } from './types';

const nmFt: Settings['units'] = { dist: 'nm', alt: 'ft' };
const kmM: Settings['units'] = { dist: 'km', alt: 'm' };

describe('units', () => {
  it('distance', () => {
    expect(fmtDist(1.852, nmFt)).toBe('1.0 nm');
    expect(fmtDist(18.52, nmFt)).toBe('10 nm');
    expect(fmtDist(3.14159, kmM)).toBe('3.1 km');
    expect(fmtDist(null, kmM)).toBe('');
  });
  it('altitude', () => {
    expect(fmtAlt(35000, nmFt)).toBe('35,000 ft');
    expect(fmtAlt(10000, kmM)).toBe('3,048 m');
    expect(fmtAlt(null, nmFt)).toBe('');
  });
  it('radar altitude in hundreds', () => {
    expect(altHundreds(35000, nmFt)).toBe('350');
    expect(altHundreds(1200, nmFt)).toBe('012');
    expect(altHundreds(null, nmFt)).toBe('---');
  });
  it('speed and vertical rate', () => {
    expect(fmtSpeed(100, nmFt)).toBe('100 kt');
    expect(fmtSpeed(100, kmM)).toBe('185 km/h');
    expect(fmtVRate(1530, nmFt)).toBe('+1,550 ft/min');
    expect(fmtVRate(-980, nmFt)).toBe('-1,000 ft/min');
  });
  it('relative time', () => {
    expect(fmtAgo(1000, 6000)).toBe('5s ago');
    expect(fmtAgo(0, 10 * 60_000)).toBe('10 min ago');
  });
});
