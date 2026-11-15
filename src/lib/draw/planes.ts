// Shared plane layer: trails, glyphs, selection marks and radar-style data
// blocks with leader lines. Used by the radar and the map overlay.

import type { Palette } from '../theme';
import type { RenderPlane, Scene } from '../scene';
import type { Settings, AircraftView } from '../types';
import { GLYPH_SIZE, glyphPath } from '../glyphs';
import { altHundreds, speedFromKt } from '../units';

export interface Hit {
  hex: string;
  x: number;
  y: number;
  r: number;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface PlaneLayerOpts {
  ctx: CanvasRenderingContext2D;
  pal: Palette;
  scene: Scene;
  /** ENU metres -> canvas px (CSS px, DPR already applied to ctx). */
  proj: (e: number, n: number) => [number, number];
  ui: number;
  units: Settings['units'];
  selectedHex: string | null;
  hoverHex: string | null;
  featuredHex: string | null;
  time: number;
  reduceMotion: boolean;
  width: number;
  height: number;
  /** Planes whose bearing was just swept get brighter. */
  sweepRad?: number;
  /** Clip planes outside this ENU radius (m) from drawing at all (radar). */
  labelsEnabled?: boolean;
  glow?: boolean;
}

const textCache = new Map<string, number>();
function measure(ctx: CanvasRenderingContext2D, font: string, text: string): number {
  const k = font + '|' + text;
  let w = textCache.get(k);
  if (w == null) {
    ctx.font = font;
    w = ctx.measureText(text).width;
    if (textCache.size > 4000) textCache.clear();
    textCache.set(k, w);
  }
  return w;
}

function colorFor(v: AircraftView, pal: Palette): string {
  if (v.badges.includes('emergency')) return pal.danger;
  if (v.badges.includes('military')) return pal.mil;
  if (v.on_ground) return pal.ink3;
  return pal.plane;
}

export function glyphSize(v: AircraftView, ui: number): number {
  return GLYPH_SIZE[v.category] * ui * (v.on_ground ? 0.8 : 1);
}

function sweepBoost(p: RenderPlane, o: PlaneLayerOpts): number {
  if (o.sweepRad == null || o.reduceMotion) return 0.55;
  const ang = Math.atan2(p.x, p.y); // bearing from north, clockwise
  const behind = (((o.sweepRad - ang) % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
  return Math.exp(-behind / 0.9);
}

function drawTrail(o: PlaneLayerOpts, p: RenderPlane, color: string, alpha: number) {
  const { ctx, proj } = o;
  const tr = p.v.trail;
  if (tr.length < 1 || alpha <= 0.02) return;
  const pts: [number, number][] = tr.map((t) => proj(t[0], t[1]));
  pts.push(proj(p.x, p.y));
  const n = pts.length;
  const K = Math.min(6, n - 1);
  if (K < 1) return;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  for (let k = 0; k < K; k++) {
    const i0 = Math.floor((k * (n - 1)) / K);
    const i1 = Math.floor(((k + 1) * (n - 1)) / K);
    const a = ((k + 1) / K) ** 1.6;
    ctx.beginPath();
    ctx.moveTo(pts[i0][0], pts[i0][1]);
    for (let i = i0 + 1; i <= i1; i++) ctx.lineTo(pts[i][0], pts[i][1]);
    // soft persistence underlay, then the crisp line
    ctx.globalAlpha = alpha * a * 0.22;
    ctx.lineWidth = 4 * o.ui;
    ctx.strokeStyle = color;
    ctx.stroke();
    ctx.globalAlpha = alpha * a * 0.85;
    ctx.lineWidth = 1.3 * o.ui;
    ctx.stroke();
  }
}

function drawArrow(ctx: CanvasRenderingContext2D, x: number, y: number, dir: -1 | 0 | 1, s: number, color: string) {
  ctx.fillStyle = color;
  ctx.strokeStyle = color;
  if (dir === 0) {
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    ctx.moveTo(x - s, y);
    ctx.lineTo(x + s, y);
    ctx.stroke();
    return;
  }
  ctx.beginPath();
  ctx.moveTo(x, y - dir * s * 1.1);
  ctx.lineTo(x + s, y + dir * s * 0.8);
  ctx.lineTo(x - s, y + dir * s * 0.8);
  ctx.closePath();
  ctx.fill();
}

function overlap(a: Rect, b: Rect): number {
  const w = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
  const h = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
  return w > 0 && h > 0 ? w * h : 0;
}

export function drawPlaneLayer(o: PlaneLayerOpts): Hit[] {
  const { ctx, pal, scene, ui } = o;
  const order = scene.drawOrder();
  const hits: Hit[] = [];
  const glow = o.glow ?? order.length <= 110;
  const screen = new Map<string, { x: number; y: number; r: number }>();

  // ---- trails (under glyphs)
  ctx.save();
  if (pal.dark) ctx.globalCompositeOperation = 'lighter';
  for (const p of order) {
    if (p.alpha < 0.03 || p.v.on_ground) continue;
    const boost = sweepBoost(p, o);
    drawTrail(o, p, colorFor(p.v, pal), p.alpha * (0.7 + 0.3 * boost));
  }
  ctx.restore();

  // ---- glyphs
  for (const p of order) {
    if (p.alpha < 0.02) continue;
    const [x, y] = o.proj(p.x, p.y);
    const size = glyphSize(p.v, ui);
    screen.set(p.v.hex, { x, y, r: size * 0.6 });
    if (p.alpha > 0.3) hits.push({ hex: p.v.hex, x, y, r: Math.max(12, size * 0.7) });
    const color = colorFor(p.v, pal);
    const boost = sweepBoost(p, o);
    const a = p.alpha * (p.v.on_ground ? 0.55 : 0.7 + 0.3 * boost);
    ctx.save();
    ctx.translate(x, y);
    ctx.rotate(((p.v.on_ground && p.v.track_deg == null ? 0 : p.hdg) * Math.PI) / 180);
    const s = (size / 50) * (1 + (p.v.on_ground ? 0 : 0.06 * boost));
    ctx.scale(s, s);
    ctx.globalAlpha = a;
    if (glow && pal.dark && !p.v.on_ground && p.alpha > 0.3) {
      ctx.shadowColor = color;
      ctx.shadowBlur = (4 + 8 * boost) / s;
    }
    ctx.fillStyle = color;
    ctx.fill(glyphPath(p.v.category));
    ctx.restore();
    if (p.v.category === 'helicopter' && !p.v.on_ground) {
      // spinning rotor disc
      ctx.save();
      ctx.translate(x, y);
      ctx.globalAlpha = a * 0.7;
      ctx.strokeStyle = color;
      ctx.lineWidth = 1.2;
      ctx.setLineDash([size * 0.5, size * 0.28]);
      ctx.lineDashOffset = o.reduceMotion ? 0 : -o.time * 0.03;
      ctx.beginPath();
      ctx.arc(0, 0, size * 0.62, 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    }
  }

  // ---- selection marks
  const mark = (hex: string | null, kind: 'select' | 'hover' | 'feature') => {
    const sc = hex ? screen.get(hex) : undefined;
    const p = hex ? scene.get(hex) : undefined;
    if (!sc || !p || p.alpha < 0.05) return;
    const r = Math.max(sc.r * 1.35, 12 * ui);
    ctx.save();
    ctx.globalAlpha = Math.min(1, p.alpha + 0.2);
    ctx.translate(sc.x, sc.y);
    if (kind === 'select') {
      ctx.strokeStyle = pal.select;
      ctx.lineWidth = 1.6;
      ctx.setLineDash([4, 3]);
      ctx.lineDashOffset = o.reduceMotion ? 0 : -o.time * 0.012;
      ctx.beginPath();
      ctx.arc(0, 0, r, 0, Math.PI * 2);
      ctx.stroke();
    } else if (kind === 'hover') {
      ctx.strokeStyle = pal.select;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.arc(0, 0, r, 0, Math.PI * 2);
      ctx.stroke();
    } else {
      // featured: corner brackets
      ctx.strokeStyle = pal.accent;
      ctx.lineWidth = 1.6;
      const b = r * 1.05;
      const t = b * 0.45;
      ctx.beginPath();
      for (const [sx, sy] of [[-1, -1], [1, -1], [1, 1], [-1, 1]]) {
        ctx.moveTo(sx * b, sy * (b - t));
        ctx.lineTo(sx * b, sy * b);
        ctx.lineTo(sx * (b - t), sy * b);
      }
      ctx.stroke();
    }
    ctx.restore();
  };
  mark(o.featuredHex, 'feature');
  mark(o.hoverHex, 'hover');
  mark(o.selectedHex, 'select');

  // ---- data blocks
  if (o.labelsEnabled !== false) drawLabels(o, order, screen);
  return hits;
}

const FONT_CS = (ui: number) => `700 ${Math.round(11.5 * ui)}px "Chakra Petch", sans-serif`;
const FONT_DATA = (ui: number) => `400 ${Math.round(10 * ui)}px "B612 Mono", monospace`;

function drawLabels(o: PlaneLayerOpts, order: RenderPlane[], screen: Map<string, { x: number; y: number; r: number }>) {
  const { ctx, pal, ui } = o;
  const fCs = FONT_CS(ui);
  const fData = FONT_DATA(ui);
  const lineH = 12 * ui;
  const placed: Rect[] = [];
  const glyphs: Rect[] = [];
  for (const [, s] of screen) glyphs.push({ x: s.x - s.r, y: s.y - s.r, w: s.r * 2, h: s.r * 2 });

  const prio = (p: RenderPlane) =>
    (p.v.hex === o.selectedHex ? 1e9 : 0) + (p.v.hex === o.hoverHex ? 5e8 : 0) + (p.v.hex === o.featuredHex ? 2e8 : 0) + (p.v.badges.includes('emergency') ? 1e8 : 0) - p.v.dist_km;
  const cands = order
    .filter((p) => p.alpha > 0.2 && (!p.v.on_ground || p.v.hex === o.selectedHex || p.v.hex === o.hoverHex))
    .sort((a, b) => prio(b) - prio(a))
    .slice(0, 70);

  for (const p of cands) {
    const sc = screen.get(p.v.hex);
    if (!sc) continue;
    const cs = p.v.callsign ?? p.v.hex.toUpperCase();
    const alt = p.v.on_ground ? 'GND' : altHundreds(p.v.alt_ft, o.units);
    const spd = p.v.gs_kt != null && !p.v.on_ground ? String(Math.round(speedFromKt(p.v.gs_kt, o.units))) : '';
    const vr = p.v.vrate_fpm ?? 0;
    const dir: -1 | 0 | 1 = p.v.on_ground ? 0 : vr > 250 ? 1 : vr < -250 ? -1 : 0;
    const arrowW = 10 * ui;
    const l2 = `${alt}`;
    const l2b = spd;
    const w1 = measure(ctx, fCs, cs);
    const w2 = measure(ctx, fData, l2) + arrowW + (l2b ? 4 * ui + measure(ctx, fData, l2b) : 0);
    const w = Math.max(w1, w2) + 6 * ui;
    const h = lineH * 2 + 3 * ui;
    const r = sc.r + 4 * ui;
    const slots: [number, number][] = [
      [r, -h - r * 0.3],
      [r, r * 0.3],
      [-w - r, -h - r * 0.3],
      [-w - r, r * 0.3],
      [-w / 2, -h - r - 6 * ui],
      [-w / 2, r + 6 * ui],
      [r + 14 * ui, -h / 2],
      [-w - r - 14 * ui, -h / 2],
    ];
    let best = -1;
    let bestScore = Infinity;
    const tryOrder = p.slot >= 0 ? [p.slot, ...slots.map((_, i) => i).filter((i) => i !== p.slot)] : slots.map((_, i) => i);
    for (const i of tryOrder) {
      const rect: Rect = { x: sc.x + slots[i][0], y: sc.y + slots[i][1], w, h };
      let pen = 0;
      if (rect.x < 2 || rect.y < 2 || rect.x + w > o.width - 2 || rect.y + h > o.height - 2) pen += 400;
      for (const q of placed) pen += overlap(rect, q) * 3;
      for (const g of glyphs) {
        if (g.x + g.w / 2 === sc.x && g.y + g.h / 2 === sc.y) continue;
        pen += overlap(rect, g);
      }
      if (i !== tryOrder[0]) pen += 0.5; // mild stickiness
      if (pen < bestScore) {
        bestScore = pen;
        best = i;
        if (pen === 0 && i === tryOrder[0]) break;
      }
    }
    if (best < 0) continue;
    p.slot = best;
    const rect: Rect = { x: sc.x + slots[best][0], y: sc.y + slots[best][1], w, h };
    placed.push(rect);

    const emerg = p.v.badges.includes('emergency');
    const mil = p.v.badges.includes('military');
    const hi = p.v.hex === o.selectedHex || p.v.hex === o.hoverHex;
    const a = Math.min(1, p.alpha + (hi ? 0.3 : 0));
    const col1 = emerg ? pal.danger : mil ? pal.mil : hi ? pal.select : pal.ink;

    ctx.save();
    ctx.globalAlpha = a;
    // leader line to nearest point on the block
    const nx = Math.max(rect.x, Math.min(sc.x, rect.x + rect.w));
    const ny = Math.max(rect.y, Math.min(sc.y, rect.y + rect.h));
    const dx = nx - sc.x;
    const dy = ny - sc.y;
    const dist = Math.hypot(dx, dy);
    if (dist > sc.r + 7 * ui) {
      ctx.strokeStyle = pal.ringStrong;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(sc.x + (dx / dist) * (sc.r + 2), sc.y + (dy / dist) * (sc.r + 2));
      ctx.lineTo(nx, ny);
      ctx.stroke();
    }
    // block backing + tick
    ctx.globalAlpha = a * 0.55;
    ctx.fillStyle = pal.bg;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    ctx.globalAlpha = a;
    ctx.fillStyle = emerg ? pal.danger : hi ? pal.select : pal.ringStrong;
    ctx.fillRect(rect.x, rect.y, 1.6, rect.h);
    ctx.textBaseline = 'top';
    ctx.font = fCs;
    ctx.fillStyle = col1;
    ctx.fillText(cs, rect.x + 5 * ui, rect.y + 1);
    ctx.font = fData;
    ctx.fillStyle = pal.ink2;
    const ty = rect.y + lineH + 2 * ui;
    ctx.fillText(l2, rect.x + 5 * ui, ty);
    const ax = rect.x + 5 * ui + measure(ctx, fData, l2) + 6 * ui;
    drawArrow(ctx, ax, ty + 5.5 * ui, dir, 3.2 * ui, dir === 1 ? pal.accent : dir === -1 ? pal.warn : pal.ink3);
    if (l2b) {
      ctx.fillStyle = pal.ink2;
      ctx.fillText(l2b, ax + 8 * ui, ty);
    }
    ctx.restore();
  }
}
