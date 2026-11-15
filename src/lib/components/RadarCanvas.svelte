<script lang="ts">
  import { onMount } from 'svelte';
  import { app, scene } from '../store.svelte';
  import { readPalette, type Palette } from '../theme';
  import { StaticLayer, nearestHit, reducedMotion, startLoop } from '../canvas';
  import { drawRadar, drawRadarBase } from '../draw/radar';
  import type { Hit } from '../draw/planes';

  let canvas: HTMLCanvasElement;
  let hits: Hit[] = [];
  let pal: Palette;
  let palKey = '';
  const layer = new StaticLayer();
  const reduce = reducedMotion();

  onMount(() => {
    const loop = startLoop(
      canvas,
      (ctx, w, h, time) => {
        if (palKey !== app.settings.theme) {
          pal = readPalette();
          palKey = app.settings.theme;
        }
        scene.update(performance.now());
        const units = app.settings.units;
        const radius = app.snapshot?.view_radius_nm ?? app.settings.view_radius_nm;
        layer.blit(ctx, w, h, `${app.settings.theme}|${radius}|${units.dist}`, (c) =>
          drawRadarBase({ ctx: c, w, h, pal, units, viewRadiusNm: radius }),
        );
        hits = drawRadar({
          ctx, w, h, pal, scene, units, selectedHex: app.pinnedHex, hoverHex: app.hoverHex, featuredHex: app.featuredHex,
          time, reduceMotion: reduce, viewRadiusNm: radius,
        }).hits;
      },
      { idleFrameMs: () => (scene.list.length ? 0 : 50) },
    );
    return () => loop.destroy();
  });

  function pos(e: PointerEvent) {
    const r = canvas.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top] as const;
  }
  function move(e: PointerEvent) {
    const [x, y] = pos(e);
    const h = nearestHit(hits, x, y);
    app.hover(h?.hex ?? null);
    canvas.style.cursor = h ? 'pointer' : 'default';
  }
  function click(e: PointerEvent) {
    const [x, y] = pos(e);
    const h = nearestHit(hits, x, y);
    if (h) app.pin(h.hex);
    else if (app.pinnedHex) app.pin(app.pinnedHex);
  }
</script>

<canvas
  bind:this={canvas}
  class="block h-full w-full"
  onpointermove={move}
  onpointerleave={() => app.hover(null)}
  onpointerdown={click}
  aria-label="Radar view of aircraft around home"
></canvas>
