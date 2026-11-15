import { describe, expect, it } from 'vitest';
import { compass, elevationDeg, enuToLatLon, latLonToEnu, smoothstep } from './geo';

describe('geo', () => {
  it('elevation matches the core formula', () => {
    expect(elevationDeg(0, 10_000)).toBeCloseTo(90, 6);
    expect(elevationDeg(10_000, 10_000)).toBeCloseTo(44.9, 0);
    expect(elevationDeg(300_000, 3_000)).toBeLessThan(0);
  });
  it('enu round trip', () => {
    const home = { lat: 51.5, lon: -0.12 };
    const p = enuToLatLon(home, 12_345, -6_789);
    const [e, n] = latLonToEnu(home, p.lat, p.lon);
    expect(e).toBeCloseTo(12_345, 0);
    expect(n).toBeCloseTo(-6_789, 0);
  });
  it('compass points', () => {
    expect(compass(0)).toBe('N');
    expect(compass(92)).toBe('E');
    expect(compass(359)).toBe('N');
    expect(compass(225)).toBe('SW');
  });
  it('smoothstep clamps', () => {
    expect(smoothstep(0, 1, -1)).toBe(0);
    expect(smoothstep(0, 1, 2)).toBe(1);
    expect(smoothstep(0, 1, 0.5)).toBeCloseTo(0.5, 6);
  });
});
