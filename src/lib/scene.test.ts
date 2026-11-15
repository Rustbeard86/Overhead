import { describe, expect, it } from 'vitest';
import { Scene, linkAlphaFor, MAX_EXTRAPOLATE_S, BLEND_MS } from './scene';
import type { AircraftView, Snapshot } from './types';
import { NM_M } from './geo';

function plane(over: Partial<AircraftView> = {}): AircraftView {
  return {
    hex: 'a', callsign: 'TST1', registration: null, type_code: 'A320', type_name: null, airline: null, airline_color: null,
    category: 'narrowbody', lat: 0, lon: 0, e_m: 0, n_m: 0, ve_ms: 100, vn_ms: 0, alt_ft: 10000, alt_geom_ft: null,
    on_ground: false, gs_kt: 194, track_deg: 90, vrate_fpm: 0, squawk: null, seen_pos_s: 0, dist_km: 0, bearing_deg: 0,
    elev_deg: 90, slant_km: 3, ca_t_s: null, ca_d_km: null, score: 1, badges: [], stale: false, in_view: true, trail: [], ...over,
  };
}
function snap(aircraft: AircraftView[], radiusNm = 40): Snapshot {
  return {
    seq: 1, t_ms: 0, source: 'demo', link: 'live', provider: null, data_age_s: 0, providers: [], home: { lat: 0, lon: 0 },
    home_is_demo: true, view_radius_nm: radiusNm, aircraft, featured: null, pinned: null, last_seen: null,
    today: { count: 0, closest_km: null, lowest_ft: null },
  };
}

describe('dead reckoning', () => {
  it('extrapolates by speed and track since the snapshot', () => {
    const s = new Scene();
    s.ingest(snap([plane({ e_m: 1000 })]), 0);
    s.update(2000);
    expect(s.get('a')!.x).toBeCloseTo(1200, 3);
    expect(s.get('a')!.y).toBeCloseTo(0, 3);
  });
  it('includes the age of the fix (seen_pos)', () => {
    const s = new Scene();
    s.ingest(snap([plane({ e_m: 0, seen_pos_s: 3 })]), 0);
    expect(s.get('a')!.x).toBeCloseTo(300, 3);
  });
  it('never extrapolates beyond 60 s', () => {
    const s = new Scene();
    s.ingest(snap([plane()]), 0);
    s.update(200_000);
    expect(s.get('a')!.x).toBeCloseTo(100 * MAX_EXTRAPOLATE_S, 3);
  });
  it('aircraft on the ground do not move', () => {
    const s = new Scene();
    s.ingest(snap([plane({ on_ground: true, ve_ms: 0, vn_ms: 0, e_m: 50 })]), 0);
    s.update(10_000);
    expect(s.get('a')!.x).toBe(50);
  });
});

describe('blending and fading', () => {
  it('blends a correction over ~300 ms without jumping', () => {
    const s = new Scene();
    s.ingest(snap([plane({ e_m: 0 })]), 0);
    s.update(3000); // displayed at x=300
    // new fix says the plane is actually at x=360 at t=3000
    s.ingest(snap([plane({ e_m: 360 })]), 3000);
    expect(s.get('a')!.x).toBeCloseTo(300, 0); // no jump
    s.update(3000 + BLEND_MS / 2);
    const mid = s.get('a')!.x;
    expect(mid).toBeGreaterThan(300 + 100 * 0.15);
    s.update(3000 + BLEND_MS + 50);
    expect(s.get('a')!.x).toBeCloseTo(360 + 100 * ((BLEND_MS + 50) / 1000), 0);
  });
  it('fades new planes in', () => {
    const s = new Scene();
    s.ingest(snap([plane()]), 0);
    expect(s.get('a')!.alpha).toBeLessThan(0.1);
    s.update(1000);
    expect(s.get('a')!.alpha).toBeCloseTo(1, 1);
  });
  it('eases out beyond the range edge', () => {
    const s = new Scene();
    s.ingest(snap([plane({ e_m: 40 * NM_M - 10, ve_ms: 0 })]), 0);
    s.update(2000);
    expect(s.get('a')!.alpha).toBeGreaterThan(0.95);
    const s2 = new Scene();
    s2.ingest(snap([plane({ e_m: 45 * NM_M, ve_ms: 0 })]), 0);
    s2.update(2000);
    expect(s2.get('a')!.alpha).toBe(0);
    const s3 = new Scene();
    s3.ingest(snap([plane({ e_m: 42 * NM_M, ve_ms: 0 })]), 0);
    s3.update(2000);
    const a = s3.get('a')!.alpha;
    expect(a).toBeGreaterThan(0);
    expect(a).toBeLessThan(1);
  });
  it('stale planes are dimmed', () => {
    const s = new Scene();
    s.ingest(snap([plane({ stale: true })]), 0);
    s.update(2000);
    expect(s.get('a')!.alpha).toBeLessThan(0.5);
  });
});

describe('link fade', () => {
  it('fades out after 60 s without data', () => {
    expect(linkAlphaFor(null)).toBe(1);
    expect(linkAlphaFor(10)).toBe(1);
    expect(linkAlphaFor(67)).toBeGreaterThan(0);
    expect(linkAlphaFor(67)).toBeLessThan(1);
    expect(linkAlphaFor(80)).toBe(0);
  });
});
