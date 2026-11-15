// Sky view: polar plot. Centre = straight up, edge = horizon, angle = compass
// bearing (north up, clockwise). Each plane sits at its azimuth and elevation,
// its recent path drawn as a trail in the same projection.

import type { Palette } from '../theme';
import type { RenderPlane, Scene } from '../scene';
import type { Settings } from '../types';
import { clamp, compass, elevationDeg, FT_M } from '../geo';
import { glyphPath, GLYPH_SIZE } from '../glyphs';
import { altHundreds } from '../units';
import type { Hit } from './planes';

export interface SkyOpts {
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
}

export function skyGeometry(w: number, h: number) {
  const ui = clamp(Math.min(w, h) / 720, 0.75, 1.5);
  const R = Math.max(40, Math.min(w, h) / 2 - 38 * ui);
  return { cx: w / 2, cy: h / 2, R, ui };
}

/** Polar position for (azimuth deg, elevation deg). */
export function skyXY(cx: number, cy: number, R: number, az: number, el: number): [number, number] {
  const r = (R * (90 - clamp(el, -5, 90))) / 90;
  const a = (az * Math.PI) / 180;
  return [cx + r * Math.sin(a), cy - r * Math.cos(a)];
}

function azEl(e: number, n: number, altM: number): { az: number; el: number } {
  const d = Math.hypot(e, n);
  const az = d < 1 ? 0 : ((Math.atan2(e, n) * 180) / Math.PI + 360) % 360;
  return { az, el: elevationDeg(d, altM) };
}

export function drawSky(o: SkyOpts): { hits: Hit[] } {
  const { ctx, w, h, pal, scene } = o;
  const { cx, cy, R, ui } = skyGeometry(w, h);

  ctx.fillStyle = pal.bg;
  ctx.fillRect(0, 0, w, h);
  // dome: brighter toward the horizon
  const dome = ctx.createRadialGradient(cx, cy, R * 0.1, cx, cy, R);
  dome.addColorStop(0, pal.radarBg);
  dome.addColorStop(0.75, pal.radarCore);
  dome.addColorStop(1, pal.radarBg);
  ctx.fillStyle = dome;
  ctx.beginPath();
  ctx.arc(cx, cy, R + 4 * ui, 0, Math.PI * 2);
  ctx.fill();

  // elevation rings
  ctx.font = `400 ${Math.round(9.5 * ui)}px "B612 Mono", monospace`;
  ctx.textBaseline = 'middle';
  for (const el of [60, 30, 0]) {
    const r = (R * (90 - el)) / 90;
    ctx.strokeStyle = el === 0 ? pal.ringStrong : pal.ring;
    ctx.lineWidth = el === 0 ? 1.6 : 1;
    ctx.setLineDash(el === 0 ? [] : [2, 4]);
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.stroke();
    ctx.setLineDash([]);
    const txt = el === 0 ? 'HORIZON' : `${el}°`;
    const tw = ctx.measureText(txt).width;
    const lx = cx + r * Math.SQRT1_2 + 3;
    const ly = cy - r * Math.SQRT1_2;
    ctx.fillStyle = pal.radarBg;
    ctx.fillRect(lx - 2, ly - 6 * ui, tw + 4, 12 * ui);
    ctx.fillStyle = pal.ink3;
    ctx.fillText(txt, lx, ly);
  }
  // spokes + compass labels
  ctx.strokeStyle = pal.ring;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let a = 0; a < 360; a += 45) {
    const rad = (a * Math.PI) / 180;
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + Math.sin(rad) * R, cy - Math.cos(rad) * R);
  }
  ctx.stroke();
  ctx.textAlign = 'center';
  for (let a = 0; a < 360; a += 15) {
    const rad = (a * Math.PI) / 180;
    const major = a % 90 === 0;
    const len = (major ? 9 : a % 45 === 0 ? 6 : 3.5) * ui;
    ctx.strokeStyle = major ? pal.ringStrong : pal.ring;
    ctx.beginPath();
    ctx.moveTo(cx + Math.sin(rad) * (R + 4 * ui), cy - Math.cos(rad) * (R + 4 * ui));
    ctx.lineTo(cx + Math.sin(rad) * (R + 4 * ui + len), cy - Math.cos(rad) * (R + 4 * ui + len));
    ctx.stroke();
    if (a % 45 === 0) {
      ctx.fillStyle = major ? pal.ink : pal.ink3;
      ctx.font = major ? `700 ${Math.round(12 * ui)}px "Chakra Petch", sans-serif` : `600 ${Math.round(10 * ui)}px "Chakra Petch", sans-serif`;
      ctx.fillText(compass(a), cx + Math.sin(rad) * (R + 4 * ui + len + 11 * ui), cy - Math.cos(rad) * (R + 4 * ui + len + 11 * ui));
    }
  }
  ctx.textAlign = 'start';
  // zenith mark
  ctx.strokeStyle = pal.accent;
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.arc(cx, cy, 3.5 * ui, 0, Math.PI * 2);
  ctx.moveTo(cx - 8 * ui, cy);
  ctx.lineTo(cx + 8 * ui, cy);
  ctx.moveTo(cx, cy - 8 * ui);
  ctx.lineTo(cx, cy + 8 * ui);
  ctx.stroke();

  // planes
  const hits: Hit[] = [];
  ctx.save();
  ctx.beginPath();
  ctx.arc(cx, cy, R + 2 * ui, 0, Math.PI * 2);
  ctx.clip();
  const order = scene.drawOrder().filter((p) => !p.v.on_ground);
  const pos = new Map<string, { x: number; y: number; el: number; az: number }>();
  for (const p of order) {
    const { az, el } = azEl(p.x, p.y, p.altM);
    if (el < -1.5) continue;
    const [x, y] = skyXY(cx, cy, R, az, el);
    pos.set(p.v.hex, { x, y, el, az });
  }
  // trails
  if (pal.dark) ctx.globalCompositeOperation = 'lighter';
  ctx.lineCap = 'round';
  for (const p of order) {
    const here = pos.get(p.v.hex);
    if (!here || p.alpha < 0.03) continue;
    const pts: [number, number][] = [];
    for (const t of p.v.trail) {
      const a = azEl(t[0], t[1], t[3] * FT_M);
      if (a.el < -1.5) continue;
      pts.push(skyXY(cx, cy, R, a.az, a.el));
    }
    pts.push([here.x, here.y]);
    for (let i = 1; i < pts.length; i++) {
      const f = (i / pts.length) ** 1.5;
      ctx.strokeStyle = p.v.badges.includes('emergency') ? pal.danger : p.v.badges.includes('military') ? pal.mil : pal.plane;
      ctx.globalAlpha = p.alpha * f * 0.8;
      ctx.lineWidth = 1.4 * ui;
      ctx.beginPath();
      ctx.moveTo(pts[i - 1][0], pts[i - 1][1]);
      ctx.lineTo(pts[i][0], pts[i][1]);
      ctx.stroke();
    }
  }
  ctx.globalCompositeOperation = 'source-over';
  ctx.globalAlpha = 1;

  // glyphs: oriented along their motion on screen
  const draw = (p: RenderPlane, here: { x: number; y: number; el: number; az: number }) => {
    const v = p.v;
    // screen heading: project 2 s ahead
    const ahead = azEl(p.x + v.ve_ms * 2, p.y + v.vn_ms * 2, p.altM);
    const [ax, ay] = skyXY(cx, cy, R, ahead.az, ahead.el);
    let rot = Math.atan2(ax - here.x, -(ay - here.y));
    if (Math.hypot(ax - here.x, ay - here.y) < 0.01) rot = ((v.track_deg ?? 0) * Math.PI) / 180;
    const size = GLYPH_SIZE[v.category] * ui * 1.1;
    const color = v.badges.includes('emergency') ? pal.danger : v.badges.includes('military') ? pal.mil : pal.plane;
    ctx.save();
    ctx.translate(here.x, here.y);
    ctx.rotate(rot);
    const s = size / 50;
    ctx.scale(s, s);
    ctx.globalAlpha = p.alpha;
    if (pal.dark && p.alpha > 0.3) {
      ctx.shadowColor = color;
      ctx.shadowBlur = 8 / s;
    }
    ctx.fillStyle = color;
    ctx.fill(glyphPath(v.category));
    ctx.restore();
    if (p.alpha > 0.3) hits.push({ hex: v.hex, x: here.x, y: here.y, r: Math.max(13, size * 0.7) });
  };
  for (const p of order) {
    const here = pos.get(p.v.hex);
    if (here && p.alpha > 0.02) draw(p, here);
  }
  ctx.restore();

  // marks and labels
  const font1 = `700 ${Math.round(11.5 * ui)}px "Chakra Petch", sans-serif`;
  const font2 = `400 ${Math.round(10 * ui)}px "B612 Mono", monospace`;
  const labelled = order
    .filter((p) => pos.has(p.v.hex) && p.alpha > 0.25)
    .sort((a, b) => (b.v.hex === o.selectedHex ? 1e6 : 0) + (b.v.hex === o.featuredHex ? 1e5 : 0) - b.v.dist_km - ((a.v.hex === o.selectedHex ? 1e6 : 0) + (a.v.hex === o.featuredHex ? 1e5 : 0) - a.v.dist_km))
    .slice(0, 28);
  const placed: { x: number; y: number; w: number; h: number }[] = [];
  for (const p of labelled) {
    const here = pos.get(p.v.hex)!;
    const hi = p.v.hex === o.selectedHex || p.v.hex === o.hoverHex;
    const feat = p.v.hex === o.featuredHex;
    if (hi || feat) {
      ctx.save();
      ctx.strokeStyle = hi ? pal.select : pal.accent;
      ctx.lineWidth = 1.5;
      if (p.v.hex === o.selectedHex) ctx.setLineDash([4, 3]);
      ctx.beginPath();
      ctx.arc(here.x, here.y, 14 * ui, 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    }
    const cs = p.v.callsign ?? p.v.hex.toUpperCase();
    const l2 = `${Math.round(here.el)}° ${altHundreds(p.v.alt_ft, o.units)}`;
    ctx.font = font1;
    const w1 = ctx.measureText(cs).width;
    ctx.font = font2;
    const w2 = ctx.measureText(l2).width;
    const bw = Math.max(w1, w2) + 8 * ui;
    const bh = 26 * ui;
    let rect = { x: here.x + 13 * ui, y: here.y - bh - 2, w: bw, h: bh };
    const alts = [
      { x: here.x + 13 * ui, y: here.y + 3, w: bw, h: bh },
      { x: here.x - bw - 13 * ui, y: here.y - bh - 2, w: bw, h: bh },
      { x: here.x - bw - 13 * ui, y: here.y + 3, w: bw, h: bh },
    ];
    const hitRect = (r: typeof rect) => placed.some((q) => r.x < q.x + q.w && r.x + r.w > q.x && r.y < q.y + q.h && r.y + r.h > q.y);
    if (hitRect(rect)) rect = alts.find((r) => !hitRect(r)) ?? rect;
    placed.push(rect);
    ctx.save();
    ctx.globalAlpha = Math.min(1, p.alpha + (hi ? 0.3 : 0));
    ctx.globalAlpha *= 0.55;
    ctx.fillStyle = pal.bg;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    ctx.globalAlpha = Math.min(1, p.alpha + (hi ? 0.3 : 0));
    ctx.fillStyle = hi ? pal.select : pal.ringStrong;
    ctx.fillRect(rect.x, rect.y, 1.6, rect.h);
    ctx.textBaseline = 'top';
    ctx.font = font1;
    ctx.fillStyle = p.v.badges.includes('emergency') ? pal.danger : hi ? pal.select : pal.ink;
    ctx.fillText(cs, rect.x + 5 * ui, rect.y + 1);
    ctx.font = font2;
    ctx.fillStyle = pal.ink2;
    ctx.fillText(l2, rect.x + 5 * ui, rect.y + 13.5 * ui);
    ctx.restore();
  }
  return { hits };
}
