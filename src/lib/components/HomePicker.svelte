<script lang="ts">
  import { onMount } from 'svelte';
  import * as maplibre from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { buildStyle } from '../mapStyle';

  let { lat = $bindable(), lon = $bindable(), height = '14rem' }: { lat: number | null; lon: number | null; height?: string } = $props();
  let el: HTMLDivElement;
  let map: maplibre.Map | undefined;
  let marker: maplibre.Marker | undefined;
  let failed = $state(false);

  onMount(() => {
    try {
      map = new maplibre.Map({
        container: el, style: buildStyle(), center: [lon ?? -0.1278, lat ?? 51.5074], zoom: lat != null ? 9 : 2.5,
        attributionControl: { compact: true }, dragRotate: false, maxPitch: 0,
      });
      map.on('error', () => (failed = true));
      map.on('click', (e) => {
        lat = Math.round(e.lngLat.lat * 1e5) / 1e5;
        lon = Math.round(e.lngLat.lng * 1e5) / 1e5;
      });
    } catch {
      failed = true;
    }
    return () => map?.remove();
  });
  $effect(() => {
    if (!map || lat == null || lon == null || !isFinite(lat) || !isFinite(lon)) return;
    if (!marker) {
      const m = document.createElement('div');
      m.style.cssText = 'width:18px;height:18px;border:2px solid var(--accent);border-radius:50%;box-shadow:var(--glow);background:color-mix(in srgb,var(--accent) 25%,transparent)';
      marker = new maplibre.Marker({ element: m }).setLngLat([lon, lat]).addTo(map);
    } else marker.setLngLat([lon, lat]);
  });
</script>

<div class="relative border border-line2" style:height>
  <div bind:this={el} class="absolute inset-0" style="background: var(--map-land); cursor: crosshair"></div>
  {#if failed}
    <div class="pointer-events-none absolute inset-x-0 top-0 bg-panel/90 px-3 py-1 text-xs text-warn">Map tiles unavailable offline. Enter coordinates instead.</div>
  {/if}
  <div class="pointer-events-none absolute bottom-6 left-2 bg-panel/85 px-2 py-0.5 text-[10px] uppercase tracking-widest text-ink2">Click the map to set home</div>
</div>
