// Canvas plumbing: DPR-aware sizing, resize observer, rAF loop with an idle
// throttle, and cached static layers.

export interface CanvasLoop {
  destroy(): void;
  size(): { w: number; h: number };
  invalidate(): void;
}

export function reducedMotion(): boolean {
  return typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export function startLoop(
  canvas: HTMLCanvasElement,
  draw: (ctx: CanvasRenderingContext2D, w: number, h: number, time: number) => void,
  opts: { idleFrameMs?: () => number; alpha?: boolean } = {},
): CanvasLoop {
  const ctx = canvas.getContext('2d', { alpha: opts.alpha ?? false })!;
  let w = 0;
  let h = 0;
  let dpr = 1;
  let raf = 0;
  let last = 0;
  let dead = false;

  const resize = () => {
    const r = canvas.getBoundingClientRect();
    dpr = Math.min(window.devicePixelRatio || 1, 2);
    w = Math.max(1, Math.round(r.width));
    h = Math.max(1, Math.round(r.height));
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
  };
  resize();
  const ro = new ResizeObserver(() => resize());
  ro.observe(canvas);

  const frame = (t: number) => {
    if (dead) return;
    raf = requestAnimationFrame(frame);
    const minGap = opts.idleFrameMs?.() ?? 0;
    if (minGap > 0 && t - last < minGap) return;
    last = t;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw(ctx, w, h, t);
  };
  raf = requestAnimationFrame(frame);
  return {
    destroy() {
      dead = true;
      cancelAnimationFrame(raf);
      ro.disconnect();
    },
    size: () => ({ w, h }),
    invalidate() {},
  };
}

/** A cached offscreen layer redrawn only when `key` changes. */
export class StaticLayer {
  private cv: HTMLCanvasElement | null = null;
  private key = '';
  blit(ctx: CanvasRenderingContext2D, w: number, h: number, key: string, paint: (c: CanvasRenderingContext2D) => void) {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const k = `${w}x${h}@${dpr}|${key}`;
    if (!this.cv || this.key !== k) {
      this.cv ??= document.createElement('canvas');
      this.cv.width = Math.round(w * dpr);
      this.cv.height = Math.round(h * dpr);
      const c = this.cv.getContext('2d')!;
      c.setTransform(dpr, 0, 0, dpr, 0, 0);
      paint(c);
      this.key = k;
    }
    ctx.save();
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.drawImage(this.cv, 0, 0);
    ctx.restore();
  }
}

export function nearestHit<T extends { x: number; y: number; r: number }>(hits: T[], x: number, y: number): T | null {
  let best: T | null = null;
  let bd = Infinity;
  for (const h of hits) {
    const d = Math.hypot(h.x - x, h.y - y);
    if (d <= h.r && d < bd) {
      best = h;
      bd = d;
    }
  }
  return best;
}
