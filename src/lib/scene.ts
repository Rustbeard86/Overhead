// The scene holds the latest snapshot and per-aircraft render state. It does
// dead reckoning (extrapolating from the last fix by speed and track, max 60 s),
// blends corrections over ~300 ms when a new snapshot arrives, fades planes in,
// and eases them in/out at the range edge. Pure TypeScript: unit-tested.

import type { AircraftView, Snapshot } from './types';
import { NM_M, clamp, easeOutCubic, smoothstep } from './geo';

export const MAX_EXTRAPOLATE_S = 60;
export const BLEND_MS = 300;
export const FADE_IN_MS = 700;
/** Planes ease in/out over this band beyond the view radius (nm). */
export const EDGE_BAND_NM = 4;

export interface RenderPlane {
  v: AircraftView;
  /** Displayed ENU position, metres. */
  x: number;
  y: number;
  /** Displayed altitude, metres above home. */
  altM: number;
  /** 0..1 overall opacity (edge easing, fade-in, staleness, link). */
  alpha: number;
  /** Heading used for drawing, degrees. */
  hdg: number;
  born: number;
  // blend state
  offX: number;
  offY: number;
  blendStart: number;
  // label layout memory
  slot: number;
}

export class Scene {
  snap: Snapshot | null = null;
  snapT = 0; // performance.now() when received
  planes = new Map<string, RenderPlane>();
  list: RenderPlane[] = [];
  radiusM = 40 * NM_M;
  /** 0..1 global fade for "Data unavailable". */
  linkAlpha = 1;

  /** Extrapolated ENU position at time `t` (performance.now ms) from a view. */
  static extrapolate(v: AircraftView, snapT: number, t: number): [number, number] {
    if (v.on_ground || (v.ve_ms === 0 && v.vn_ms === 0)) return [v.e_m, v.n_m];
    const dt = clamp((t - snapT) / 1000 + v.seen_pos_s, 0, MAX_EXTRAPOLATE_S);
    return [v.e_m + v.ve_ms * dt, v.n_m + v.vn_ms * dt];
  }

  ingest(s: Snapshot, now: number) {
    // capture current displayed positions to blend from
    const prev = this.planes;
    const next = new Map<string, RenderPlane>();
    this.snap = s;
    this.snapT = now;
    this.radiusM = s.view_radius_nm * NM_M;
    for (const v of s.aircraft) {
      const old = prev.get(v.hex);
      const [tx, ty] = Scene.extrapolate(v, now, now);
      const rp: RenderPlane = old ?? {
        v, x: tx, y: ty, altM: 0, alpha: 0, hdg: v.track_deg ?? 0, born: now, offX: 0, offY: 0, blendStart: now, slot: -1,
      };
      if (old) {
        // displayed - new target = residual to blend away
        rp.offX = old.x - tx;
        rp.offY = old.y - ty;
        rp.blendStart = now;
      }
      rp.v = v;
      next.set(v.hex, rp);
    }
    this.planes = next;
    this.list = [...next.values()];
    this.update(now);
  }

  /** Advance render state to time `now` (call once per frame). */
  update(now: number) {
    const s = this.snap;
    if (!s) return;
    for (const p of this.list) {
      const v = p.v;
      const [tx, ty] = Scene.extrapolate(v, this.snapT, now);
      const b = 1 - easeOutCubic((now - p.blendStart) / BLEND_MS);
      p.x = tx + p.offX * b;
      p.y = ty + p.offY * b;
      const dt = clamp((now - this.snapT) / 1000 + v.seen_pos_s, 0, MAX_EXTRAPOLATE_S);
      const altFt = v.on_ground ? 0 : (v.alt_geom_ft ?? v.alt_ft ?? 0) + (v.vrate_fpm ?? 0) * (dt / 60);
      p.altM = Math.max(0, altFt) * 0.3048;
      if (v.track_deg != null) p.hdg = v.track_deg;
      // opacity
      const d = Math.hypot(p.x, p.y);
      const band = EDGE_BAND_NM * NM_M;
      const edge = 1 - smoothstep(this.radiusM, this.radiusM + band, d);
      const born = clamp((now - p.born) / FADE_IN_MS, 0, 1);
      p.alpha = edge * easeOutCubic(born) * (v.stale ? 0.45 : 1) * this.linkAlpha;
    }
  }

  /** Planes sorted far-to-near so near ones draw on top. */
  drawOrder(): RenderPlane[] {
    return [...this.list].sort((a, b) => b.v.dist_km - a.v.dist_km);
  }

  get(hex: string | null | undefined): RenderPlane | undefined {
    return hex ? this.planes.get(hex) : undefined;
  }
}

/** Link-loss fade: 1 while data is fresh, fading to 0 between 60 and 75 s of age. */
export function linkAlphaFor(dataAgeS: number | null): number {
  if (dataAgeS == null) return 1;
  return 1 - smoothstep(60, 75, dataAgeS);
}
