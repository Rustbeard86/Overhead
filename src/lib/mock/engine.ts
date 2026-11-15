// Browser-only stand-in for the Rust core, used for `npm run dev` without Tauri
// and for headless design screenshots. It emits the same Snapshot shape from a
// SYNTHETIC scene (mirrors crates/core/src/synth.rs). Never used in the app.

import type { AircraftView, Badge, Category, Snapshot, TodayData } from '../types';
import { NM_M, FT_M, KT_MS, elevationDeg, enuToLatLon } from '../geo';

interface P {
  hex: string; cs: string; reg: string; type: string; name: string; airline?: string; color?: string; cat: Category;
  p0: [number, number]; v: [number, number]; alt0: number; vr: number; period: number;
  badges?: Badge[]; squawk?: string; ground?: boolean;
}

const HOME = { lat: 51.5074, lon: -0.1278 };

const SCENE: P[] = [
  { hex: '5c0001', cs: 'BAW117', reg: 'G-EUPT', type: 'A320', name: 'Airbus A320', airline: 'British Airways', color: '#2b4c9b', cat: 'narrowbody', p0: [-26000, 0], v: [200, 0], alt0: 9843, vr: 0, period: 260 },
  { hex: '5c0002', cs: 'EZY81KP', reg: 'G-UZHA', type: 'A20N', name: 'Airbus A320neo', airline: 'easyJet', color: '#ff6600', cat: 'narrowbody', p0: [-30000, 4000], v: [200, 0], alt0: 6000, vr: 0, period: 300 },
  { hex: '5c0003', cs: 'CFE24D', reg: 'G-LCYJ', type: 'E190', name: 'Embraer 190', airline: 'BA CityFlyer', color: '#2b4c9b', cat: 'narrowbody', p0: [0, -24000], v: [0, 130], alt0: 5000, vr: -700, period: 200 },
  { hex: '5c0004', cs: 'GXPOL', reg: 'G-POLC', type: 'EC35', name: 'Airbus H135', cat: 'helicopter', p0: [1500, 800], v: [0, 0], alt0: 800, vr: 0, period: 0 },
  { hex: '4ca001', cs: 'VIR3N', reg: 'G-VLUX', type: 'B789', name: 'Boeing 787-9', airline: 'Virgin Atlantic', color: '#e10a0a', cat: 'widebody', p0: [-60000, -22000], v: [230, 80], alt0: 9500, vr: 800, period: 480, badges: ['heavy'] },
  { hex: '4ca002', cs: 'UAE7', reg: 'A6-EOA', type: 'A388', name: 'Airbus A380-800', airline: 'Emirates', color: '#d71921', cat: 'widebody', p0: [45000, 30000], v: [-215, -95], alt0: 4200, vr: -900, period: 400, badges: ['heavy', 'rare'] },
  { hex: '43c001', cs: 'RRR7216', reg: 'ZZ336', type: 'A400', name: 'Airbus A400M Atlas', airline: 'Royal Air Force', cat: 'military', p0: [50000, -10000], v: [-170, 25], alt0: 12000, vr: 0, period: 520, badges: ['military', 'heavy'], squawk: '4201' },
  { hex: '4ca003', cs: 'RYR42XE', reg: 'EI-DWB', type: 'B738', name: 'Boeing 737-800', airline: 'Ryanair', color: '#073590', cat: 'narrowbody', p0: [-12000, 40000], v: [60, -190], alt0: 11000, vr: -1800, period: 420, badges: ['emergency'], squawk: '7700' },
  { hex: '400f01', cs: 'GOSEY', reg: 'G-OSEY', type: 'C172', name: 'Cessna 172 Skyhawk', cat: 'light', p0: [22000, -8000], v: [-30, 12], alt0: 2300, vr: 0, period: 1500 },
  { hex: '400a01', cs: 'BAW19C', reg: 'G-TTNA', type: 'A320', name: 'Airbus A320', airline: 'British Airways', color: '#2b4c9b', cat: 'narrowbody', p0: [9000, -6000], v: [0, 0], alt0: 0, vr: 0, period: 0, ground: true },
  { hex: '400a02', cs: 'EZY62RH', reg: 'G-UZLK', type: 'A319', name: 'Airbus A319', airline: 'easyJet', color: '#ff6600', cat: 'narrowbody', p0: [9200, -6000], v: [0, 0], alt0: 0, vr: 0, period: 0, ground: true },
];
const BG = [
  ['DLH4RA', 'A21N', 'Airbus A321neo', 'Lufthansa', 'narrowbody'], ['AFR1380', 'A320', 'Airbus A320', 'Air France', 'narrowbody'], ['KLM1002', 'E195', 'Embraer 195', 'KLM', 'narrowbody'],
  ['SAS4631', 'A320', 'Airbus A320', 'SAS Scandinavian', 'narrowbody'], ['IBE3162', 'A21N', 'Airbus A321neo', 'Iberia', 'narrowbody'], ['EIN156', 'A320', 'Airbus A320', 'Aer Lingus', 'narrowbody'],
  ['SWR238', 'A359', 'Airbus A350-900', 'Swiss', 'widebody'], ['AAL100', 'B77W', 'Boeing 777-300ER', 'American Airlines', 'widebody'], ['UAL925', 'B789', 'Boeing 787-9', 'United Airlines', 'widebody'],
  ['QTR8', 'A359', 'Airbus A350-900', 'Qatar Airways', 'widebody'], ['TAP1359', 'A20N', 'Airbus A320neo', 'TAP Air Portugal', 'narrowbody'], ['AEE601', 'A320', 'Airbus A320', 'Aegean Airlines', 'narrowbody'],
  ['NAX1D', 'B38M', 'Boeing 737 MAX 8', 'Norwegian', 'narrowbody'], ['WZZ4372', 'A21N', 'Airbus A321neo', 'Wizz Air', 'narrowbody'], ['LOT282', 'DH8D', 'Dash 8 Q400', 'LOT Polish Airlines', 'narrowbody'], ['ETD14', 'B789', 'Boeing 787-9', 'Etihad Airways', 'widebody'],
] as const;

let seed = 0x9e3779b9;
const rnd = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296);
BG.forEach((b, i) => {
  const brg = rnd() * Math.PI * 2;
  const r0 = 65000;
  const speed = 120 + rnd() * 110;
  const off = (rnd() - 0.5) * 70000;
  const s: [number, number] = [r0 * Math.sin(brg), r0 * Math.cos(brg)];
  const tgt: [number, number] = [Math.cos(brg) * off, -Math.sin(brg) * off];
  const len = Math.hypot(tgt[0] - s[0], tgt[1] - s[1]);
  const v: [number, number] = [((tgt[0] - s[0]) / len) * speed, ((tgt[1] - s[1]) / len) * speed];
  const period = ((2 * r0) / speed) * 1.05;
  const phase = rnd() * period;
  const alt = 4000 + rnd() * 34000;
  SCENE.push({
    hex: '4d' + (i + 1).toString(16).padStart(4, '0'), cs: b[0], reg: '', type: b[1], name: b[2], airline: b[3], cat: b[4] as Category,
    p0: [s[0] + v[0] * phase, s[1] + v[1] * phase], v, alt0: Math.max(1500, Math.min(41000, alt)), vr: alt < 12000 ? -1200 + rnd() * 2400 : 0, period,
    badges: b[4] === 'widebody' ? ['heavy'] : [],
  });
});

function stateAt(p: P, t: number) {
  const tt = p.period > 0 ? ((t % p.period) + p.period) % p.period : t;
  return { e: p.p0[0] + p.v[0] * tt, n: p.p0[1] + p.v[1] * tt, alt: p.ground ? 0 : Math.max(0, p.alt0 + (p.vr * tt) / 60) };
}

const trails = new Map<string, [number, number, number, number, number][]>(); // e,n,alt,t(sec)

export class MockEngine {
  t0: number;
  seq = 0;
  pinned: string | null = null;
  radiusNm = 40;
  featured: string | null = null;
  constructor(t0 = 0) { this.t0 = t0; }

  snapshot(wall: number, startWall: number): Snapshot {
    const t = this.t0 + (wall - startWall) / 1000;
    const aircraft: AircraftView[] = [];
    for (const p of SCENE) {
      const s = stateAt(p, t);
      const sp = Math.hypot(p.v[0], p.v[1]);
      const dist = Math.hypot(s.e, s.n);
      if (dist > this.radiusNm * NM_M + 10 * NM_M) continue;
      const tr = trails.get(p.hex) ?? [];
      const last = tr[tr.length - 1];
      if (!p.ground && !last) { for (let k = 300; k >= 6; k -= 6) { const q = stateAt(p, t - k); if (p.period > 0 && ((t - k) % p.period + p.period) % p.period > ((t % p.period) + p.period) % p.period) continue; tr.push([q.e, q.n, q.alt, t - k, 0]); } }
      if (!p.ground && (!tr.length || Math.hypot(tr[tr.length - 1][0] - s.e, tr[tr.length - 1][1] - s.n) > 80)) tr.push([s.e, s.n, s.alt, t, 0]);
      while (tr.length && t - tr[0][3] > 300) tr.shift();
      if (tr.length > 120) tr.splice(0, tr.length - 120);
      trails.set(p.hex, tr);
      const track = sp < 0.01 ? null : ((Math.atan2(p.v[0], p.v[1]) * 180) / Math.PI + 360) % 360;
      const altFt = p.ground ? null : s.alt;
      const h = (altFt ?? 0) * FT_M;
      const el = p.ground ? null : elevationDeg(dist, h);
      const slant = Math.hypot(dist, h) / 1000;
      const vel: [number, number] = p.ground ? [0, 0] : p.v;
      const v2 = vel[0] ** 2 + vel[1] ** 2;
      let ca_t: number | null = null, ca_d: number | null = null;
      if (v2 > 1) {
        const tc = -(s.e * vel[0] + s.n * vel[1]) / v2;
        const d = Math.hypot(s.e + vel[0] * tc, s.n + vel[1] * tc);
        if (tc >= 0 && tc <= 900) { ca_t = tc; ca_d = d / 1000; }
      }
      const badges = p.badges ?? [];
      let score = 100 / (1 + (slant / 6) ** 2);
      if (ca_t != null && ca_d! < 5) score += 40 * (1 - ca_d! / 5) * (1 - ca_t / 900);
      if (badges.includes('military')) score += 30;
      if (badges.includes('emergency')) score += 80;
      if (badges.includes('heavy')) score += 10;
      if (p.cat === 'helicopter') { score += 15; badges.includes('helicopter') || badges.push('helicopter'); }
      if (badges.includes('rare')) score += 15;
      if (!p.ground) score += 10 * Math.max(0, 1 - (altFt ?? 0) / 10000); else score *= 0.08;
      const ll = enuToLatLon(HOME, s.e, s.n);
      aircraft.push({
        hex: p.hex, callsign: p.cs, registration: p.reg || null, type_code: p.type, type_name: p.name, airline: p.airline ?? null,
        airline_color: p.color ?? null, category: p.cat, lat: ll.lat, lon: ll.lon, e_m: Math.round(s.e), n_m: Math.round(s.n),
        ve_ms: vel[0], vn_ms: vel[1], alt_ft: altFt, alt_geom_ft: altFt, on_ground: !!p.ground, gs_kt: p.ground ? 0 : sp / KT_MS,
        track_deg: track, vrate_fpm: p.ground ? 0 : p.vr, squawk: p.squawk ?? '2000', seen_pos_s: 0.4, dist_km: dist / 1000,
        bearing_deg: dist < 1 ? 0 : ((Math.atan2(s.e, s.n) * 180) / Math.PI + 360) % 360, elev_deg: el, slant_km: slant,
        ca_t_s: ca_t, ca_d_km: ca_d, score, badges: [...badges], stale: false, in_view: dist <= this.radiusNm * NM_M,
        trail: tr.map((q) => [Math.round(q[0]), Math.round(q[1]), Math.round(t - q[3]), Math.round(q[2])] as [number, number, number, number]),
      });
    }
    aircraft.sort((a, b) => a.dist_km - b.dist_km);
    const inView = aircraft.filter((a) => a.in_view && !a.stale);
    const best = inView.reduce<AircraftView | null>((b, a) => (!b || a.score > b.score ? a : b), null);
    const cur = inView.find((a) => a.hex === this.featured);
    if (!cur || (best && best.hex !== cur.hex && best.score > cur.score * 1.2)) this.featured = best?.hex ?? null;
    if (this.pinned && !inView.some((a) => a.hex === this.pinned)) this.pinned = null;
    this.seq++;
    return {
      seq: this.seq, t_ms: wall, source: 'demo', link: 'live', provider: 'demo', data_age_s: 0.4, providers: [], home: HOME, home_is_demo: true,
      view_radius_nm: this.radiusNm, aircraft, featured: this.pinned ?? this.featured, pinned: this.pinned,
      last_seen: { hex: '5c0001', callsign: 'BAW117', type_code: 'A320', t_ms: wall - 4 * 60_000, min_dist_km: 2.6 },
      today: { count: 47, closest_km: 0.8, lowest_ft: 1650 },
    };
  }
}

export function mockToday(): TodayData {
  const hourly = [0, 0, 0, 0, 1, 3, 6, 9, 12, 10, 8, 7, 9, 11, 8, 6, 5, 4, 2, 1, 0, 0, 0, 0];
  const now = Date.now();
  const rows = SCENE.filter((p) => !p.ground).slice(0, 12).map((p, i) => ({
    id: i, hex: p.hex, callsign: p.cs, registration: p.reg || null, type_code: p.type,
    first_seen_ms: now - (i + 1) * 17 * 60_000, last_seen_ms: now - (i + 1) * 17 * 60_000 + 6 * 60_000,
    min_dist_km: 0.8 + i * 1.9, closest_ms: now - (i + 1) * 17 * 60_000 + 3 * 60_000, min_alt_ft: 1650 + i * 1700,
  }));
  return { stats: { count: 47, closest_km: 0.8, lowest_ft: 1650 }, hourly, sightings: rows };
}
