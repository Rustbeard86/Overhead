import type { Settings } from './types';
import { FT_M, NM_M } from './geo';

type Units = Settings['units'];

export function distFromKm(km: number, u: Units): number {
  switch (u.dist) {
    case 'nm': return (km * 1000) / NM_M;
    case 'mi': return km / 1.609344;
    default: return km;
  }
}
export function distUnit(u: Units): string {
  return u.dist;
}
export function fmtDist(km: number | null | undefined, u: Units, digits?: number): string {
  if (km == null || !isFinite(km)) return '';
  const v = distFromKm(km, u);
  const d = digits ?? (v < 10 ? 1 : 0);
  return `${v.toFixed(d)} ${u.dist}`;
}
export function fmtAlt(ft: number | null | undefined, u: Units): string {
  if (ft == null) return '';
  const v = u.alt === 'm' ? ft * FT_M : ft;
  return `${Math.round(v).toLocaleString('en-US')} ${u.alt}`;
}
/** Radar-style altitude: hundreds, 3 digits ("350" = 35,000 ft). */
export function altHundreds(ft: number | null | undefined, u: Units): string {
  if (ft == null) return '---';
  const v = u.alt === 'm' ? ft * FT_M : ft;
  return String(Math.max(0, Math.round(v / 100))).padStart(3, '0');
}
export function speedFromKt(kt: number, u: Units): number {
  switch (u.dist) {
    case 'km': return kt * 1.852;
    case 'mi': return kt * 1.15078;
    default: return kt;
  }
}
export function speedUnit(u: Units): string {
  return u.dist === 'km' ? 'km/h' : u.dist === 'mi' ? 'mph' : 'kt';
}
export function fmtSpeed(kt: number | null | undefined, u: Units): string {
  if (kt == null) return '';
  return `${Math.round(speedFromKt(kt, u))} ${speedUnit(u)}`;
}
export function fmtVRate(fpm: number | null | undefined, u: Units): string {
  if (fpm == null) return '';
  const v = u.alt === 'm' ? fpm * FT_M : fpm;
  const r = Math.round(v / 50) * 50;
  return `${r > 0 ? '+' : ''}${r.toLocaleString('en-US')} ${u.alt}/min`;
}
export function fmtAgo(ms: number, now = Date.now()): string {
  const s = Math.max(0, Math.round((now - ms) / 1000));
  if (s < 90) return `${s}s ago`;
  const m = Math.round(s / 60);
  if (m < 90) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 36) return `${h} h ago`;
  return `${Math.round(h / 24)} d ago`;
}
export function fmtDur(s: number): string {
  if (s < 90) return `${Math.round(s)} s`;
  return `${Math.round(s / 60)} min`;
}

/** Inverse of distFromKm. */
export function kmFromDist(v: number, u: Units): number {
  switch (u.dist) {
    case 'nm': return (v * NM_M) / 1000;
    case 'mi': return v * 1.609344;
    default: return v;
  }
}
export function altFromFt(ft: number, u: Units): number {
  return u.alt === 'm' ? ft * FT_M : ft;
}
export function ftFromAlt(v: number, u: Units): number {
  return u.alt === 'm' ? v / FT_M : v;
}
/** Round to the slider's step so converted values show as clean numbers. */
export function snapTo(v: number, step: number): number {
  return Math.round(v / step) * step;
}
/** Slider bounds as [min, max, step] in the unit the person picked. */
export const DIST_RANGES = {
  radius: { nm: [10, 150, 5], km: [20, 280, 10], mi: [10, 170, 5] },
  offset: { nm: [1, 13, 1], km: [1, 25, 1], mi: [1, 15, 1] },
  flyover: { nm: [0.25, 8, 0.25], km: [0.5, 15, 0.5], mi: [0.25, 10, 0.25] },
} as const;
export const CEILING_RANGES = { ft: [1000, 40000, 500], m: [300, 12000, 100] } as const;
