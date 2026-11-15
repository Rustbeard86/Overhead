// Radar scope: range rings, compass ring with ticks, sweep with phosphor
// persistence, home marker, then the shared plane layer.

import type { Palette } from '../theme';
import type { Scene } from '../scene';
import type { Settings } from '../types';
import { NM_M } from '../geo';
import { distFromKm } from '../units';
import { drawPlaneLayer, type Hit } from './planes';

export const SWEEP_PERIOD_MS = 7000;

export interface RadarOpts {
  ctx: CanvasRenderingContext2D;
  w: number;
  h: number;
  pal: Palette;
  scene: Scene;
  units: Settings['units'];
  selectedHex: string | null;
  hoverHex: string | null;
  featuredHex: string | null;
  time: number;
  reduceMotion: boolean;
  viewRadiusNm: number;
}

export interface RadarGeometry {
  cx: number;
  cy: number;
  R: number;
  mPerPx: number;
  ui: number;
}

export function radarGeometry(w: number, h: number, viewRadiusNm: number): RadarGeometry {
  const ui = Math.min(1.5, Math.max(0.75, Math.min(w, h) / 720));
  const R = Math.max(40, Math.min(w, h) / 2 - 38 * ui);
  return { cx: w / 2, cy: h / 2, R, mPerPx: (viewRadiusNm * NM_M) / R, ui };
}

function niceStep(maxUnits: number): number {
  const target = maxUnits / 4;
  const steps = [0.5, 1, 2, 5, 10, 20, 25, 50, 100, 200];
  return steps.find((s) => s >= target * 0.8) ?? 200;
}

export interface RadarBaseOpts {
  ctx: CanvasRenderingContext2D;
  w: number;
  h: number;
  pal: Palette;
  units: Settings['units'];
  viewRadiusNm: number;
}

/** Static scope furniture: background, rings, compass ring. Cached by the caller. */
export function drawRadarBase(o: RadarBaseOpts) {
  const { ctx, w, h, pal, units } = o;
  const g = radarGeometry(w, h, o.viewRadiusNm);
  const { cx, cy, R, ui } = g;

  // background
  ctx.fillStyle = pal.bg;
  ctx.fillRect(0, 0, w, h);
  const bg = ctx.createRadialGradient(cx, cy, 0, cx, cy, R * 1.05);
  bg.addColorStop(0, pal.radarCore);
  bg.addColorStop(1, pal.radarBg);
  ctx.fillStyle = bg;
  ctx.beginPath();
  ctx.arc(cx, cy, R + 8 * ui, 0, Math.PI * 2);
  ctx.fill();

  // range rings
  const radiusUnits = distFromKm((o.viewRadiusNm * NM_M) / 1000, units);
  const step = niceStep(radiusUnits);
  ctx.lineWidth = 1;
  ctx.font = `400 ${Math.round(9.5 * ui)}px "B612 Mono", monospace`;
  ctx.textBaseline = 'middle';
  for (let u = step; u < radiusUnits * 0.999; u += step) {
    const r = (u / radiusUnits) * R;
    ctx.strokeStyle = pal.ring;
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.stroke();
    // label on the 45-degree spoke, with a small knock-out behind it
    const lx = cx + r * Math.SQRT1_2;
    const ly = cy - r * Math.SQRT1_2;
    const txt = `${u}`;
    const tw = ctx.measureText(txt).width;
    ctx.fillStyle = pal.radarBg;
    ctx.globalAlpha = 0.85;
    ctx.fillRect(lx - 2, ly - 6 * ui, tw + 4, 12 * ui);
    ctx.globalAlpha = 1;
    ctx.fillStyle = pal.ink3;
    ctx.fillText(txt, lx, ly);
  }
  // outer ring (view radius) with unit label
  ctx.strokeStyle = pal.ringStrong;
  ctx.lineWidth = 1.4;
  ctx.beginPath();
  ctx.arc(cx, cy, R, 0, Math.PI * 2);
  ctx.stroke();
  {
    const txt = `${Math.round(radiusUnits)} ${units.dist}`;
    const tw = ctx.measureText(txt).width;
    const lx = cx + R * Math.SQRT1_2;
    const ly = cy - R * Math.SQRT1_2;
    ctx.fillStyle = pal.radarBg;
    ctx.fillRect(lx - 3, ly - 6.5 * ui, tw + 6, 13 * ui);
    ctx.fillStyle = pal.ink2;
    ctx.fillText(txt, lx, ly);
  }

  // cardinal cross
  ctx.strokeStyle = pal.ring;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(cx - R, cy);
  ctx.lineTo(cx + R, cy);
  ctx.moveTo(cx, cy - R);
  ctx.lineTo(cx, cy + R);
  ctx.stroke();

  // compass ring with tick marks
  const rc = R + 9 * ui;
  ctx.strokeStyle = pal.ringStrong;
  ctx.beginPath();
  ctx.arc(cx, cy, rc, 0, Math.PI * 2);
  ctx.stroke();
  ctx.fillStyle = pal.ink2;
  ctx.textAlign = 'center';
  ctx.font = `600 ${Math.round(10 * ui)}px "Chakra Petch", sans-serif`;
  for (let a = 0; a < 360; a += 5) {
    const major = a % 30 === 0;
    const mid = a % 10 === 0;
    const len = (major ? 9 : mid ? 5.5 : 3) * ui;
    const rad = ((a - 90) * Math.PI) / 180;
    const c = Math.cos(rad);
    const s = Math.sin(rad);
    ctx.strokeStyle = major ? pal.ringStrong : pal.ring;
    ctx.lineWidth = major ? 1.4 : 1;
    ctx.beginPath();
    ctx.moveTo(cx + c * rc, cy + s * rc);
    ctx.lineTo(cx + c * (rc + len), cy + s * (rc + len));
    ctx.stroke();
    if (major) {
      const label = a === 0 ? 'N' : a === 90 ? 'E' : a === 180 ? 'S' : a === 270 ? 'W' : String(a).padStart(3, '0');
      const card = a % 90 === 0;
      ctx.fillStyle = card ? pal.ink : pal.ink3;
      ctx.font = card ? `700 ${Math.round(12 * ui)}px "Chakra Petch", sans-serif` : `400 ${Math.round(9 * ui)}px "B612 Mono", monospace`;
      ctx.fillText(label, cx + c * (rc + len + 10 * ui), cy + s * (rc + len + 10 * ui));
    }
  }
  ctx.textAlign = 'start';
}

/** Per-frame layer: sweep, home marker, planes. */
export function drawRadar(o: RadarOpts): { hits: Hit[]; geo: RadarGeometry } {
  const { ctx, w, h, pal, units } = o;
  const g = radarGeometry(w, h, o.viewRadiusNm);
  const { cx, cy, R, ui } = g;
  const rc = R + 9 * ui;

  // sweep (conic gradient wedge behind the leading line)
  const sweepRad = o.reduceMotion ? undefined : ((o.time % SWEEP_PERIOD_MS) / SWEEP_PERIOD_MS) * Math.PI * 2;
  if (sweepRad != null) {
    const lead = sweepRad - Math.PI / 2; // canvas angle: 0 = east
    const trail = Math.PI * 0.36;
    ctx.save();
    ctx.beginPath();
    ctx.arc(cx, cy, R, 0, Math.PI * 2);
    ctx.clip();
    const grad = (ctx as CanvasRenderingContext2D & { createConicGradient: (a: number, x: number, y: number) => CanvasGradient }).createConicGradient(lead - trail, cx, cy);
    const f = trail / (Math.PI * 2);
    grad.addColorStop(0, `rgba(${pal.sweep}, 0)`);
    grad.addColorStop(f * 0.999, `rgba(${pal.sweep}, ${pal.dark ? 0.15 : 0.12})`);
    grad.addColorStop(Math.min(1, f * 1.002), `rgba(${pal.sweep}, 0)`);
    grad.addColorStop(1, `rgba(${pal.sweep}, 0)`);
    ctx.fillStyle = grad;
    ctx.fillRect(cx - R, cy - R, R * 2, R * 2);
    ctx.restore();
    ctx.strokeStyle = `rgba(${pal.sweep}, 0.75)`;
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + Math.cos(lead) * R, cy + Math.sin(lead) * R);
    ctx.stroke();
  }

  // home marker
  ctx.strokeStyle = pal.accent;
  ctx.lineWidth = 1.4;
  ctx.beginPath();
  ctx.arc(cx, cy, 4 * ui, 0, Math.PI * 2);
  ctx.moveTo(cx - 9 * ui, cy);
  ctx.lineTo(cx - 5.5 * ui, cy);
  ctx.moveTo(cx + 5.5 * ui, cy);
  ctx.lineTo(cx + 9 * ui, cy);
  ctx.moveTo(cx, cy - 9 * ui);
  ctx.lineTo(cx, cy - 5.5 * ui);
  ctx.moveTo(cx, cy + 5.5 * ui);
  ctx.lineTo(cx, cy + 9 * ui);
  ctx.stroke();

  // planes, clipped to the compass ring so edge fades stay inside the scope
  ctx.save();
  ctx.beginPath();
  ctx.arc(cx, cy, rc, 0, Math.PI * 2);
  ctx.clip();
  const hits = drawPlaneLayer({
    ctx, pal, scene: o.scene, ui, units, selectedHex: o.selectedHex, hoverHex: o.hoverHex, featuredHex: o.featuredHex,
    time: o.time, reduceMotion: o.reduceMotion, width: w, height: h, sweepRad,
    proj: (e, n) => [cx + e / g.mPerPx, cy - n / g.mPerPx],
  });
  ctx.restore();
  return { hits, geo: g };
}
