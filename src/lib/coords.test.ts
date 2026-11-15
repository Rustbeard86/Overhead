import { describe, expect, it } from 'vitest';
import { parseCoords } from './coords';

describe('parseCoords', () => {
  it('reads the Google Maps right-click format', () => {
    expect(parseCoords('51.5074, -0.1278')).toEqual({ lat: 51.5074, lon: -0.1278 });
    expect(parseCoords(' 40.7 -74.0 ')).toEqual({ lat: 40.7, lon: -74 });
    expect(parseCoords('(40.7; -74.0)')).toEqual({ lat: 40.7, lon: -74 });
  });
  it('reads Google Maps URLs', () => {
    expect(parseCoords('https://www.google.com/maps/place/X/@48.8584,2.2945,17z/data=!4m6')).toEqual({ lat: 48.8584, lon: 2.2945 });
    expect(parseCoords('https://x/data=!8m2!3d-33.8568!4d151.2153')).toEqual({ lat: -33.8568, lon: 151.2153 });
  });
  it('reads hemisphere letters', () => {
    expect(parseCoords('33.86 S, 151.21 E')).toEqual({ lat: -33.86, lon: 151.21 });
  });
  it('rejects junk and out-of-range values', () => {
    expect(parseCoords('')).toBeNull();
    expect(parseCoords('51.5')).toBeNull();
    expect(parseCoords('120, 20')).toBeNull();
    expect(parseCoords('hello')).toBeNull();
  });
});
