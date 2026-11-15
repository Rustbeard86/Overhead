<script lang="ts">
  import { onMount } from 'svelte';
  import * as maplibregl from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import workerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?url';
  import { LocateFixed } from '@lucide/svelte';
  import { app, scene } from '../store.svelte';
  import { readPalette, type Palette } from '../theme';
  import { nearestHit, reducedMotion, startLoop } from '../canvas';
  import { buildStyle } from '../mapStyle';
  import { drawPlaneLayer, type Hit } from '../draw/planes';
  import { enuToLatLon, NM_M } from '../geo';
  import { distFromKm } from '../units';
  import IconButton from './IconButton.svelte';

  maplibregl.setWorkerUrl(workerUrl);

  let host: HTMLDivElement;
  let overlay: HTMLCanvasElement;
  let map: maplibregl.Map | undefined;
  let hits: Hit[] = [];
  let pal: Palette;
  let palKey = '';
  let offline = $state(false);
  const reduce = reducedMotion();

  const home = $derived(app.snapshot?.home ?? { lat: 51.5074, lon: -0.1278 });

  function fitZoom(): number {
    const r = (app.snapshot?.view_radius_nm ?? 40) * NM_M;
    const px = Math.min(host.clientWidth, host.clientHeight) / 2;
    const mpp = r / Math.max(60, px - 20);
    return Math.log2((78271.517 * Math.cos((home.lat * Math.PI) / 180)) / mpp);
  }

  function snapHome(animate = true) {
    if (!map) return;
    const opts = { center: [home.lon, home.lat] as [number, number], zoom: fitZoom(), bearing: 0, pitch: 0 };
    animate && !reduce ? map.flyTo({ ...opts, duration: 700 }) : map.jumpTo(opts);
  }

  onMount(() => {
    map = new maplibregl.Map({
      container: host,
      style: buildStyle(),
      center: [home.lon, home.lat],
      zoom: 8,
      attributionControl: { compact: true },
      dragRotate: false,
      pitchWithRotate: false,
      maxPitch: 0,
      fadeDuration: 200,
    });
    map.touchZoomRotate.disableRotation();
    map.on('error', (e) => {
      const m = String(e.error?.message ?? '');
      if (/Failed to fetch|NetworkError|Load failed|AJAX|timeout/i.test(m)) offline = true;
    });
    map.on('load', () => (offline = false));
    map.once('load', () => snapHome(false));
    snapHome(false);
    let userMoved = false;
    map.on('dragstart', () => (userMoved = true));
    map.on('wheel', () => (userMoved = true));
    const ro = new ResizeObserver(() => {
      map?.resize();
      if (!userMoved) snapHome(false);
    });
    ro.observe(host);

    const loop = startLoop(
      overlay,
      (ctx, w, h, time) => {
        if (palKey !== app.settings.theme) {
          pal = readPalette();
          palKey = app.settings.theme;
        }
        ctx.clearRect(0, 0, w, h);
        scene.update(performance.now());
        const m = map!;
        const hm = home;
        const proj = (e: number, n: number): [number, number] => {
          const ll = enuToLatLon(hm, e, n);
          const p = m.project([ll.lon, ll.lat]);
          return [p.x, p.y];
        };
        // range rings in map pixels
        const c0 = m.project([hm.lon, hm.lat]);
        const east = proj(1000, 0);
        const pxPerKm = Math.hypot(east[0] - c0.x, east[1] - c0.y);
        const rNm = app.snapshot?.view_radius_nm ?? 40;
        const radiusKm = (rNm * NM_M) / 1000;
        ctx.lineWidth = 1;
        ctx.font = '400 10px "B612 Mono", monospace';
        const ru = distFromKm(radiusKm, app.settings.units);
        const steps = [1, 2, 5, 10, 20, 25, 50, 100];
        const step = steps.find((s) => s >= ru / 4 * 0.8) ?? 100;
        for (let u = step; u <= ru * 1.001; u += step) {
          const rkm = (u / ru) * radiusKm;
          ctx.strokeStyle = u >= ru * 0.999 ? pal.ringStrong : pal.ring;
          ctx.setLineDash(u >= ru * 0.999 ? [] : [3, 5]);
          ctx.beginPath();
          ctx.arc(c0.x, c0.y, rkm * pxPerKm, 0, Math.PI * 2);
          ctx.stroke();
          ctx.setLineDash([]);
          ctx.fillStyle = pal.ink2;
          ctx.fillText(`${u} ${app.settings.units.dist}`, c0.x + rkm * pxPerKm * Math.SQRT1_2 + 3, c0.y - rkm * pxPerKm * Math.SQRT1_2);
        }
        // home marker
        ctx.strokeStyle = pal.accent;
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.arc(c0.x, c0.y, 5, 0, Math.PI * 2);
        ctx.moveTo(c0.x - 11, c0.y);
        ctx.lineTo(c0.x - 7, c0.y);
        ctx.moveTo(c0.x + 7, c0.y);
        ctx.lineTo(c0.x + 11, c0.y);
        ctx.moveTo(c0.x, c0.y - 11);
        ctx.lineTo(c0.x, c0.y - 7);
        ctx.moveTo(c0.x, c0.y + 7);
        ctx.lineTo(c0.x, c0.y + 11);
        ctx.stroke();

        hits = drawPlaneLayer({
          ctx, pal, scene, proj, ui: 1.05, units: app.settings.units, selectedHex: app.pinnedHex, hoverHex: app.hoverHex,
          featuredHex: app.featuredHex, time, reduceMotion: reduce, width: w, height: h,
        });
      },
      { idleFrameMs: () => (scene.list.length ? 0 : 50), alpha: true },
    );
    return () => {
      ro.disconnect();
      loop.destroy();
      map?.remove();
    };
  });

  // restyle the map live when the theme changes; recentre when the radius or home changes
  let lastTheme = '';
  let lastRadius = 0;
  let lastHome = '';
  $effect(() => {
    const t = app.settings.theme;
    const r = app.snapshot?.view_radius_nm ?? 40;
    const hk = `${home.lat},${home.lon}`;
    if (!map) return;
    if (lastTheme && t !== lastTheme) queueMicrotask(() => map?.setStyle(buildStyle()));
    if ((lastRadius && r !== lastRadius) || (lastHome && hk !== lastHome)) snapHome(true);
    lastTheme = t;
    lastRadius = r;
    lastHome = hk;
  });

  function pos(e: PointerEvent | MouseEvent) {
    const r = overlay.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top] as const;
  }
</script>

<div class="relative h-full w-full" style:background="var(--map-land)">
  <div class="absolute inset-0"><div bind:this={host} class="h-full w-full"></div></div>
  <!-- the overlay passes events to the map except when over a plane -->
  <canvas
    bind:this={overlay}
    class="pointer-events-none absolute inset-0 h-full w-full"
    aria-label="Map view of aircraft around home"
  ></canvas>
  <div class="absolute bottom-8 left-3 flex flex-col gap-2">
    <div class="panel">
      <IconButton label="Snap to home" onclick={() => snapHome(true)}><LocateFixed size={16} /></IconButton>
    </div>
  </div>
  {#if offline}
    <div class="panel absolute left-3 top-3 flex items-center gap-2 px-3 py-1.5 text-xs text-ink2" role="status">
      <span class="label !text-warn">Map tiles offline</span>
      <span>planes still track</span>
    </div>
  {/if}
</div>

<svelte:window
  onpointermove={(e) => {
    if (app.view !== 'map' || !overlay) return;
    const [x, y] = pos(e);
    const h = nearestHit(hits, x, y);
    app.hover(h?.hex ?? null);
    host.style.cursor = h ? 'pointer' : '';
  }}
  onclick={(e) => {
    if (app.view !== 'map' || !overlay) return;
    const r = overlay.getBoundingClientRect();
    if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) return;
    if ((e.target as HTMLElement).closest('button,a,.maplibregl-ctrl')) return;
    const h = nearestHit(hits, ...pos(e));
    if (h) app.pin(h.hex);
  }}
/>
