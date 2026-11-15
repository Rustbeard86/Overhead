<script lang="ts">
  import { ArrowUp, ArrowDown, Minus } from '@lucide/svelte';
  import { app } from '../store.svelte';
  import { altHundreds, distFromKm, speedFromKt } from '../units';
  import GlyphIcon from './GlyphIcon.svelte';
  import type { AircraftView } from '../types';

  const u = $derived(app.settings.units);
  const rows = $derived.by(() => {
    const list = (app.snapshot?.aircraft ?? []).filter((a) => a.in_view);
    const key: Record<typeof app.sort, (a: AircraftView) => number> = {
      dist: (a) => a.dist_km,
      alt: (a) => -(a.on_ground ? -1 : (a.alt_ft ?? -1)),
      speed: (a) => -(a.gs_kt ?? -1),
    };
    return [...list].sort((a, b) => key[app.sort](a) - key[app.sort](b));
  });
  const trend = (a: AircraftView) => (a.on_ground ? 0 : (a.vrate_fpm ?? 0) > 250 ? 1 : (a.vrate_fpm ?? 0) < -250 ? -1 : 0);
  const tag = (a: AircraftView) => (a.badges.includes('emergency') ? 'EMG' : a.badges.includes('military') ? 'MIL' : '');
</script>

<div class="flex min-h-0 flex-1 flex-col border border-line bg-panel" role="region" aria-label="Aircraft list">
  <div class="flex items-center justify-between border-b border-line px-3 py-1.5">
    <span class="label">{rows.length} in range</span>
    <div class="seg" role="group" aria-label="Sort aircraft by">
      {#each [['dist', 'Dist'], ['alt', 'Alt'], ['speed', 'Spd']] as [k, l] (k)}
        <button data-active={app.sort === k} aria-pressed={app.sort === k} style="height:1.45rem;padding:0 .5rem;font-size:var(--text-2xs)" onclick={() => (app.sort = k as typeof app.sort)}>{l}</button>
      {/each}
    </div>
  </div>
  <div class="strip !h-6 cursor-default !border-l-0 !border-b !bg-transparent !py-0 text-ink3" style="font-size:var(--text-2xs);letter-spacing:.14em" aria-hidden="true">
    <span class="label">Callsign</span><span class="label">Type</span><span class="label">Alt</span><span class="label text-right">Spd</span><span class="label text-right">Dist</span>
  </div>
  <ul class="min-h-0 flex-1 overflow-y-auto" role="listbox" aria-label="Aircraft">
    {#each rows as a (a.hex)}
      <li role="option" aria-selected={app.pinnedHex === a.hex}>
        <button
          class="strip"
          data-hover={app.hoverHex === a.hex}
          data-featured={app.featuredHex === a.hex}
          data-pinned={app.pinnedHex === a.hex}
          data-emergency={a.badges.includes('emergency')}
          data-stale={a.stale}
          onmouseenter={() => app.hover(a.hex)}
          onmouseleave={() => app.hover(null)}
          onfocus={() => app.hover(a.hex)}
          onblur={() => app.hover(null)}
          onclick={() => app.pin(a.hex)}
        >
          <span class="flex min-w-0 items-center gap-1.5">
            <GlyphIcon category={a.category} size={13} rotate={a.track_deg ?? 0} class="shrink-0 {a.badges.includes('emergency') ? 'text-danger' : a.badges.includes('military') ? 'text-mil' : 'text-accent'}" />
            <span class="cs">{a.callsign ?? a.hex.toUpperCase()}</span>
          </span>
          <span class="dim truncate">{a.type_code ?? '—'}{#if tag(a)}<b class="ml-1 text-[9px] tracking-widest {tag(a) === 'EMG' ? 'text-danger' : 'text-mil'}">{tag(a)}</b>{/if}</span>
          <span class="flex items-center gap-1">
            {a.on_ground ? 'GND' : altHundreds(a.alt_ft, u)}
            {#if trend(a) === 1}<ArrowUp size={11} class="text-accent" />{:else if trend(a) === -1}<ArrowDown size={11} class="text-warn" />{:else}<Minus size={11} class="text-ink3" />{/if}
          </span>
          <span class="text-right dim">{a.gs_kt != null && !a.on_ground ? Math.round(speedFromKt(a.gs_kt, u)) : ''}</span>
          <span class="text-right">{distFromKm(a.dist_km, u).toFixed(distFromKm(a.dist_km, u) < 10 ? 1 : 0)}</span>
        </button>
      </li>
    {:else}
      <li class="px-4 py-6 text-center text-sm text-ink3">No aircraft in range</li>
    {/each}
  </ul>
</div>
