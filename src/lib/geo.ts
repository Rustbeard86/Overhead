// Small geometry helpers used by the drawing code. The Rust core is the source
// of truth for distance, bearing, elevation and scoring; the frontend only
// repeats the cheap formulas it needs to animate smoothly between snapshots.

export const R_EARTH = 6_371_000;
export const NM_M = 1852;
export const FT_M = 0.3048;
export const KT_MS = NM_M / 3600;

const rad = (d: number) => (d * Math.PI) / 180;
const deg = (r: number) => (r * 180) / Math.PI;

/** Elevation angle (deg) of a target at ground distance d (m), height h (m) above the observer. */
export function elevationDeg(d: number, h: number): number {
  const a = d / R_EARTH;
  const num = (R_EARTH + h) * Math.cos(a) - R_EARTH;
  const den = (R_EARTH + h) * Math.sin(a);
  if (Math.abs(den) < 1e-9) return h >= 0 ? 90 : -90;
  return deg(Math.atan2(num, den));
}

/** Inverse of the core's azimuthal-equidistant ENU projection. */
export function enuToLatLon(home: { lat: number; lon: number }, e: number, n: number): { lat: number; lon: number } {
  const d = Math.hypot(e, n);
  if (d < 1e-6) return { ...home };
  const brg = Math.atan2(e, n);
  const dr = d / R_EARTH;
  const p1 = rad(home.lat);
  const l1 = rad(home.lon);
  const p2 = Math.asin(Math.sin(p1) * Math.cos(dr) + Math.cos(p1) * Math.sin(dr) * Math.cos(brg));
  const l2 = l1 + Math.atan2(Math.sin(brg) * Math.sin(dr) * Math.cos(p1), Math.cos(dr) - Math.sin(p1) * Math.sin(p2));
  return { lat: deg(p2), lon: ((deg(l2) + 540) % 360) - 180 };
}

export function latLonToEnu(home: { lat: number; lon: number }, lat: number, lon: number): [number, number] {
  const p1 = rad(home.lat);
  const p2 = rad(lat);
  const dl = rad(lon - home.lon);
  const h = Math.sin((p2 - p1) / 2) ** 2 + Math.cos(p1) * Math.cos(p2) * Math.sin(dl / 2) ** 2;
  const d = 2 * R_EARTH * Math.asin(Math.min(1, Math.sqrt(h)));
  if (d < 1e-6) return [0, 0];
  const y = Math.sin(dl) * Math.cos(p2);
  const x = Math.cos(p1) * Math.sin(p2) - Math.sin(p1) * Math.cos(p2) * Math.cos(dl);
  const b = Math.atan2(y, x);
  return [d * Math.sin(b), d * Math.cos(b)];
}

export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
export const smoothstep = (a: number, b: number, v: number) => {
  const t = clamp((v - a) / (b - a), 0, 1);
  return t * t * (3 - 2 * t);
};
export const easeOutCubic = (t: number) => 1 - (1 - clamp(t, 0, 1)) ** 3;

/** Compass point for a bearing, e.g. 92 -> "E". */
export function compass(bearing: number): string {
  const pts = ['N', 'NNE', 'NE', 'ENE', 'E', 'ESE', 'SE', 'SSE', 'S', 'SSW', 'SW', 'WSW', 'W', 'WNW', 'NW', 'NNW'];
  return pts[Math.round((((bearing % 360) + 360) % 360) / 22.5) % 16];
}
