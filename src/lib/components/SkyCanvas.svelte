<script lang="ts">
  import { onMount } from 'svelte';
  import { app, scene } from '../store.svelte';
  import { readPalette, type Palette } from '../theme';
  import { nearestHit, reducedMotion, startLoop } from '../canvas';
  import { drawSky } from '../draw/sky';
  import type { Hit } from '../draw/planes';

  let canvas: HTMLCanvasElement;
  let hits: Hit[] = [];
  let pal: Palette;
  let palKey = '';
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
        hits = drawSky({
          ctx, w, h, pal, scene, units: app.settings.units, selectedHex: app.pinnedHex, hoverHex: app.hoverHex,
          featuredHex: app.featuredHex, time, reduceMotion: reduce,
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
    const h = nearestHit(hits, ...pos(e));
    app.hover(h?.hex ?? null);
    canvas.style.cursor = h ? 'pointer' : 'default';
  }
  function click(e: PointerEvent) {
    const h = nearestHit(hits, ...pos(e));
    if (h) app.pin(h.hex);
  }
</script>

<canvas
  bind:this={canvas}
  class="block h-full w-full"
  onpointermove={move}
  onpointerleave={() => app.hover(null)}
  onpointerdown={click}
  aria-label="Sky view: polar plot of aircraft by azimuth and elevation"
></canvas>
